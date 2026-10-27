//! [`InvalidMarkup`]: a rule of ADR 017 that a parsed `.mel` document breaks.

/// A rule of ADR 017 a parsed document breaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidMarkup {
    /// An `x:` directive other than `Class`, `ViewModel`, `Name`, `Key`.
    #[error("unknown directive 'x:{0}' (known: x:Class, x:ViewModel, x:Name, x:Key)")]
    UnknownDirective(String),
    /// `x:Class` / `x:ViewModel` below the root element.
    #[error("'x:{0}' is only allowed on the root element")]
    RootOnlyDirective(String),
    /// A directive whose value is a markup extension.
    #[error("'x:{0}' takes plain text")]
    DirectiveNotText(String),
    /// An `x:Class` / `x:ViewModel` value that is not a Rust type path.
    #[error("'{0}' is not a Rust type path")]
    InvalidTypePath(String),
    /// An `x:Name` that does not turn into a Rust identifier.
    #[error("x:Name '{0}' is not usable as an identifier")]
    InvalidName(String),
    #[error("x:Name '{0}' is used twice")]
    DuplicateName(String),
    /// Two different `x:Name`s that map to the same `Name` variant.
    #[error("x:Name '{second}' generates the same identifier as '{first}'")]
    NameCollision { first: String, second: String },
    /// An `{x:Bind}` in a document without a root `x:ViewModel`.
    #[error("{{x:Bind}} needs an x:ViewModel on the root element")]
    BindingWithoutViewModel,
    /// An `{x:Bind}` on an element without an `x:Name` to address it by.
    #[error("{{x:Bind}} on <{0}> needs an x:Name on that element")]
    BindingWithoutName(String),
    #[error("{{x:Bind}} takes exactly one expression")]
    BindingArity,
    #[error("{0}")]
    BindingExpression(crate::BindingExprError),
    #[error("unknown {{x:Bind}} argument '{0}' (known: Mode, BindBack, Fallback)")]
    UnknownBindingArgument(String),
    #[error("invalid value for {{x:Bind}} argument '{0}'")]
    InvalidBindingArgument(String),
    /// `Mode=TwoWay` on an expression that is not a member path, without
    /// `BindBack`.
    #[error("Mode=TwoWay needs a member path to write back to, or BindBack=method")]
    TwoWayNeedsTarget,
    /// `Fallback=` on an expression without `?.`, which never meets `None`.
    #[error("Fallback= only applies to an expression with '?.'")]
    FallbackWithoutOptional,
    /// A `Fallback=` expression with `?.`: it is the value for `None`, so
    /// it cannot itself be missing.
    #[error("Fallback= must not contain '?.'")]
    OptionalFallback,
    /// A bound attribute that is not a registered UI property.
    #[error("<{element}> has no property '{property}'")]
    UnknownProperty { element: String, property: String },
    /// An element of a UI document that is not a `meridian-ui-core` kind.
    #[error("<{0}> is not a UI element (known: Border, StackPanel, DockPanel, Grid, TextBlock)")]
    UnknownElement(String),
    /// A markup extension other than `{x:Bind}` on a UI attribute.
    #[error("{{{0}}} is not supported on UI attributes yet")]
    UnsupportedExtension(String),
    /// `@res:Set.name` with no such entry.
    #[error("no resource '{0}' (write @res:Set.name for an entry of Set.mres)")]
    UnknownResource(String),
    /// An attribute value that starts with '@' but is not a valid
    /// Substrate-Data string.
    #[error("{0}")]
    InvalidLiteral(String),
}
