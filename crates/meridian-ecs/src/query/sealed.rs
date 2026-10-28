//! Keeps the query element types closed: nothing outside this crate may
//! implement [`Fetch`](crate::Fetch).

pub trait Sealed {}

impl<T> Sealed for &T {}
impl<T> Sealed for &mut T {}
impl<T> Sealed for Option<&T> {}
impl<T> Sealed for Option<&mut T> {}
impl<T> Sealed for crate::With<T> {}
impl<T> Sealed for crate::Without<T> {}
impl<T> Sealed for crate::Changed<T> {}
