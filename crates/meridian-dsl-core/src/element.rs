//! A parsed element: tag, attributes, children.

/// One parsed element: its tag name, its attributes (in source order,
/// duplicates rejected at parse time rather than silently
/// last-write-wins — a duplicate attribute is almost always a typo, not
/// intent), and its children in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
}

impl Element {
    /// The first attribute value matching `name`, if any — attributes
    /// are stored as a `Vec` (not a `HashMap`) so source order survives
    /// for tools that want to echo it back (formatters, error messages),
    /// but lookups still want to feel like map access.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}
