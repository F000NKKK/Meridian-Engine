//! [`Prefix`]: the namespace prefixes a `.meui` name may carry.

/// The only namespace prefix `.meui` knows: `x:`, the language
/// directives (`x:Class`, `x:ViewModel`, `x:Name`, `x:Key`), as in XAML.
/// See ADR 017.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prefix {
    Directive,
}
