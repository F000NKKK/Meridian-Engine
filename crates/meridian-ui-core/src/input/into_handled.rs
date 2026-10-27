//! [`IntoHandled`]: what a handler may return.

use super::Handled;

/// A handler returns `()` (never consumes) or [`Handled`]; generated
/// dispatch code accepts both through this trait.
pub trait IntoHandled {
    fn into_handled(self) -> Handled;
}

impl IntoHandled for () {
    fn into_handled(self) -> Handled {
        Handled::No
    }
}

impl IntoHandled for Handled {
    fn into_handled(self) -> Handled {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_never_consumes() {
        assert_eq!(().into_handled(), Handled::No);
        assert_eq!(Handled::Yes.into_handled(), Handled::Yes);
    }
}
