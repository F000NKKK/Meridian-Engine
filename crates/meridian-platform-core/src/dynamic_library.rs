//! Dynamic library loading (hot-reload/plugins): hand-written FFI
//! over the OS loader per ADR 010's original plan — `dlopen` on POSIX,
//! `LoadLibraryW` on Windows, no external crate.

/// Why [`DynamicLibrary::open`] or [`DynamicLibrary::symbol`] failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicLibraryError {
    /// The OS could not load the library — the message is the loader's
    /// own (`dlerror` text on POSIX, the `GetLastError` code on Windows).
    OpenFailed(String),
    /// The library loaded but doesn't export the requested symbol.
    SymbolNotFound(String),
    /// The path or symbol name contains an interior NUL byte and can't
    /// be passed to the C loader API.
    InvalidName(String),
}

impl std::fmt::Display for DynamicLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicLibraryError::OpenFailed(msg) => {
                write!(f, "failed to load dynamic library: {msg}")
            }
            DynamicLibraryError::SymbolNotFound(name) => {
                write!(f, "symbol not found in dynamic library: {name}")
            }
            DynamicLibraryError::InvalidName(name) => {
                write!(f, "name contains an interior NUL byte: {name}")
            }
        }
    }
}

impl std::error::Error for DynamicLibraryError {}

#[cfg(unix)]
mod dynamic_library_ffi {
    use std::os::raw::{c_char, c_int, c_void};

    pub const RTLD_NOW: c_int = 2;

    // `dl*` live in libc itself on modern glibc (≥ 2.34) and musl; the
    // explicit `dl` link keeps older glibc working, where they were
    // split into libdl.
    #[cfg_attr(target_os = "linux", link(name = "dl"))]
    unsafe extern "C" {
        pub fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        pub fn dlclose(handle: *mut c_void) -> c_int;
        pub fn dlerror() -> *mut c_char;
    }

    /// The pending `dlerror` message, if any (calling it also clears it).
    pub unsafe fn take_error() -> Option<String> {
        let msg = unsafe { dlerror() };
        if msg.is_null() {
            None
        } else {
            Some(
                unsafe { std::ffi::CStr::from_ptr(msg) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

#[cfg(windows)]
mod dynamic_library_ffi {
    use std::os::raw::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn LoadLibraryW(filename: *const u16) -> *mut c_void;
        pub fn GetProcAddress(handle: *mut c_void, symbol: *const u8) -> *mut c_void;
        pub fn FreeLibrary(handle: *mut c_void) -> i32;
        pub fn GetLastError() -> u32;
    }
}

/// A dynamically loaded library (hot-reload/plugins): hand-written FFI
/// over the OS loader, per ADR 010's original plan — see the module doc.
///
/// The library stays loaded for the lifetime of this value and is
/// unloaded on drop. Symbols looked up through [`symbol`](Self::symbol)
/// are raw function pointers into the loaded image — they must not
/// outlive this `DynamicLibrary` (unloading invalidates them), which is
/// one of the reasons `symbol` is `unsafe`.
#[derive(Debug)]
pub struct DynamicLibrary {
    handle: *mut std::os::raw::c_void,
}

// The OS loader APIs are thread-safe (POSIX requires it of dlopen/dlsym;
// Windows likewise); the handle is a process-global token, not
// thread-local state.
unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    /// Loads the library at `path` (a bare soname like `libm.so.6` is
    /// resolved through the platform's usual search order). Symbols are
    /// resolved eagerly (`RTLD_NOW`), so missing transitive symbols fail
    /// here rather than crashing at call time.
    pub fn open(path: &str) -> Result<Self, DynamicLibraryError> {
        #[cfg(unix)]
        {
            let c_path = std::ffi::CString::new(path)
                .map_err(|_| DynamicLibraryError::InvalidName(path.to_string()))?;
            // Clear any stale error state before the call.
            unsafe { dynamic_library_ffi::take_error() };
            let handle = unsafe {
                dynamic_library_ffi::dlopen(c_path.as_ptr(), dynamic_library_ffi::RTLD_NOW)
            };
            if handle.is_null() {
                let msg = unsafe { dynamic_library_ffi::take_error() }
                    .unwrap_or_else(|| path.to_string());
                return Err(DynamicLibraryError::OpenFailed(msg));
            }
            Ok(Self { handle })
        }
        #[cfg(windows)]
        {
            if path.contains('\0') {
                return Err(DynamicLibraryError::InvalidName(path.to_string()));
            }
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = unsafe { dynamic_library_ffi::LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() {
                let code = unsafe { dynamic_library_ffi::GetLastError() };
                return Err(DynamicLibraryError::OpenFailed(format!(
                    "{path}: error code {code}"
                )));
            }
            Ok(Self { handle })
        }
    }

    /// Looks up `name` and returns it as `T` — almost always an
    /// `extern "C" fn` pointer type.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `T` matches the symbol's actual
    /// ABI/signature (the loader can't check it; a mismatch is undefined
    /// behavior at call time), and that the returned value is not used
    /// after this `DynamicLibrary` is dropped.
    pub unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T, DynamicLibraryError> {
        assert_eq!(
            std::mem::size_of::<T>(),
            std::mem::size_of::<*mut std::os::raw::c_void>(),
            "DynamicLibrary::symbol::<T> requires a pointer-sized T (a fn pointer)"
        );
        let raw: *mut std::os::raw::c_void;
        #[cfg(unix)]
        {
            let c_name = std::ffi::CString::new(name)
                .map_err(|_| DynamicLibraryError::InvalidName(name.to_string()))?;
            unsafe { dynamic_library_ffi::take_error() };
            raw = unsafe { dynamic_library_ffi::dlsym(self.handle, c_name.as_ptr()) };
            // NULL alone isn't proof of failure (a symbol may legally be
            // NULL); dlerror() is the authoritative signal.
            if let Some(_msg) = unsafe { dynamic_library_ffi::take_error() } {
                return Err(DynamicLibraryError::SymbolNotFound(name.to_string()));
            }
        }
        #[cfg(windows)]
        {
            let mut c_name = name.as_bytes().to_vec();
            if c_name.contains(&0) {
                return Err(DynamicLibraryError::InvalidName(name.to_string()));
            }
            c_name.push(0);
            raw = unsafe { dynamic_library_ffi::GetProcAddress(self.handle, c_name.as_ptr()) };
            if raw.is_null() {
                return Err(DynamicLibraryError::SymbolNotFound(name.to_string()));
            }
        }
        Ok(unsafe { std::mem::transmute_copy::<*mut std::os::raw::c_void, T>(&raw) })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            dynamic_library_ffi::dlclose(self.handle);
        }
        #[cfg(windows)]
        unsafe {
            dynamic_library_ffi::FreeLibrary(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_os = "linux")]
    fn dynamic_library_loads_libm_and_calls_cos() {
        // libm.so.6 ships with glibc on every Linux this workspace
        // targets; skip (not fail) on exotic setups without it.
        let lib = match DynamicLibrary::open("libm.so.6") {
            Ok(lib) => lib,
            Err(err) => {
                eprintln!("skipping: {err}");
                return;
            }
        };
        let cos: unsafe extern "C" fn(f64) -> f64 =
            unsafe { lib.symbol("cos") }.expect("libm must export cos");
        let value = unsafe { cos(0.0) };
        assert!((value - 1.0).abs() < 1e-12);
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn dynamic_library_reports_missing_library_and_symbol() {
        assert!(matches!(
            DynamicLibrary::open("libmeridian-definitely-not-a-real-library.so"),
            Err(DynamicLibraryError::OpenFailed(_))
        ));
        if let Ok(lib) = DynamicLibrary::open("libm.so.6") {
            let missing: Result<unsafe extern "C" fn(), _> =
                unsafe { lib.symbol("meridian_no_such_symbol") };
            assert!(matches!(
                missing,
                Err(DynamicLibraryError::SymbolNotFound(_))
            ));
        }
    }
    #[test]
    fn dynamic_library_rejects_interior_nul() {
        assert!(matches!(
            DynamicLibrary::open("bad\0name"),
            Err(DynamicLibraryError::InvalidName(_))
        ));
    }
}
