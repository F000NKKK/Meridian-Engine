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

impl meridian_foundation::EngineError for DynamicLibraryError {}

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
    ///
    /// # Safety
    ///
    /// No other thread may call a `dl*` function between the call this
    /// reports on and this one; the returned text is copied before return.
    pub unsafe fn take_error() -> Option<String> {
        // SAFETY: dlerror takes no arguments; the caller upholds the
        // thread-ordering contract above.
        let msg = unsafe { dlerror() };
        if msg.is_null() {
            None
        } else {
            Some(
                // SAFETY: a non-null dlerror result is a NUL-terminated C
                // string valid until the next dl* call; it is copied here.
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

// SAFETY: the OS loader APIs are thread-safe (POSIX requires it of
// dlopen/dlsym; Windows likewise); the handle is a process-global token,
// not thread-local state, so moving it to another thread is sound.
unsafe impl Send for DynamicLibrary {}
// SAFETY: every method takes `&self` and only passes the handle to the
// thread-safe loader API; no interior state is mutated.
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
            // SAFETY: take_error only reads and clears the loader's error slot.
            unsafe { dynamic_library_ffi::take_error() };
            // SAFETY: `c_path` is a valid NUL-terminated string that outlives
            // the call; RTLD_NOW is a valid flag.
            let handle = unsafe {
                dynamic_library_ffi::dlopen(c_path.as_ptr(), dynamic_library_ffi::RTLD_NOW)
            };
            if handle.is_null() {
                // SAFETY: reads the error the failed dlopen just set.
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
            // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives
            // the call.
            let handle = unsafe { dynamic_library_ffi::LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() {
                // SAFETY: GetLastError has no preconditions.
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
            // SAFETY: clears the loader's error slot before dlsym.
            unsafe { dynamic_library_ffi::take_error() };
            // SAFETY: `self.handle` came from a successful dlopen and is open
            // until drop; `c_name` is NUL-terminated and outlives the call.
            raw = unsafe { dynamic_library_ffi::dlsym(self.handle, c_name.as_ptr()) };
            // NULL alone isn't proof of failure (a symbol may legally be
            // NULL); dlerror() is the authoritative signal.
            // SAFETY: reads the error the dlsym call above may have set.
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
            // SAFETY: `self.handle` came from a successful LoadLibraryW;
            // `c_name` is NUL-terminated and outlives the call.
            raw = unsafe { dynamic_library_ffi::GetProcAddress(self.handle, c_name.as_ptr()) };
            if raw.is_null() {
                return Err(DynamicLibraryError::SymbolNotFound(name.to_string()));
            }
        }
        // SAFETY: `T` is pointer-sized (asserted above) and the caller
        // guarantees it is the symbol's real type (see `# Safety`).
        Ok(unsafe { std::mem::transmute_copy::<*mut std::os::raw::c_void, T>(&raw) })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: `self.handle` came from a successful open and is closed
        // exactly once, here; symbols must not outlive `self` (see `symbol`).
        #[cfg(unix)]
        unsafe {
            dynamic_library_ffi::dlclose(self.handle);
        }
        // SAFETY: as above, for the Windows loader.
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
            // SAFETY: libm's `cos` has exactly this C signature; `lib` outlives it.
            unsafe { lib.symbol("cos") }.expect("libm must export cos");
        // SAFETY: `cos` is a valid libm function pointer, called with a plain f64.
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
                // SAFETY: the lookup fails; no pointer is produced or called.
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
