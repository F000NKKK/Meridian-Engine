//! The typed half of the DSL story: [`DslTag`] is what
//! `meridian-dsl-macros`' `#[dsl_tag(name = "...")]` implements for a
//! game developer's own struct, and [`DslRegistry`] is what
//! `meridian-sdk::dsl` builds a real scene tree from. Still
//! domain-blind: this module knows the *shape* of "a typed thing
//! parseable from attributes", not which types exist — `RigidBody`,
//! `Mesh`, and any custom tag a game registers are all just entries in
//! the same `HashMap`, added the same way.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

use crate::Element;

#[derive(Debug, Clone, PartialEq)]
pub struct TagParseError {
    pub message: String,
}

impl fmt::Display for TagParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TagParseError {}

/// A type that can be built from one DSL element's attributes.
/// `#[dsl_tag(name = "...")]` generates this impl; nothing prevents
/// writing it by hand for a tag whose parsing doesn't fit the macro's
/// "one attribute per field" model.
pub trait DslTag: Sized + 'static {
    /// The tag name this type binds to, e.g. `"RigidBody"` — matched
    /// exactly against [`Element::tag`], case-sensitively.
    const TAG_NAME: &'static str;

    fn from_attrs(attrs: &[(String, String)]) -> Result<Self, TagParseError>;
}

/// One built node of a parsed DSL tree: the source tag name, the typed
/// value a registered [`DslTag`] impl produced from its attributes (as
/// `Box<dyn Any>` — the registry itself has no compile-time knowledge
/// of which concrete type a given tag name maps to; a caller downcasts
/// by tag name, the same pattern `ecs-core`'s type-erased component
/// storage uses for the same reason), and its children, recursively
/// built the same way.
pub struct BuiltNode {
    pub tag: String,
    pub value: Box<dyn Any>,
    pub children: Vec<BuiltNode>,
}

impl fmt::Debug for BuiltNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltNode")
            .field("tag", &self.tag)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl BuiltNode {
    /// Downcasts [`value`](Self::value) to `T`, if this node's tag was
    /// built by `T`'s [`DslTag`] impl. `None` on a mismatch — a caller
    /// walking a tree of mixed tag types checks `node.tag` first, then
    /// calls this for the type it expects.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }
}

type Builder = Box<dyn Fn(&[(String, String)]) -> Result<Box<dyn Any>, TagParseError>>;

/// The set of tag names a DSL document may use, each bound to a
/// [`DslTag`] impl via [`register`](Self::register). Built up once by
/// an application (registering `meridian-sdk`'s own built-in tags plus
/// any custom ones its game defines), then reused to
/// [`build`](Self::build) as many documents as it wants.
#[derive(Default)]
pub struct DslRegistry {
    builders: HashMap<String, Builder>,
}

impl DslRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `T::TAG_NAME` to `T::from_attrs`. Registering the same tag
    /// name twice replaces the previous binding — deliberately permissive
    /// (an application may want to override a built-in tag with its own
    /// definition) rather than an error.
    pub fn register<T: DslTag>(&mut self) {
        self.builders.insert(
            T::TAG_NAME.to_string(),
            Box::new(|attrs| T::from_attrs(attrs).map(|v| Box::new(v) as Box<dyn Any>)),
        );
    }

    /// Recursively builds `element` and every descendant into a
    /// [`BuiltNode`] tree. Fails closed: an element whose tag was never
    /// [`register`](Self::register)ed is a [`TagParseError`], not a
    /// silently-skipped node — a typo'd tag name in a DSL document must
    /// surface, not vanish.
    pub fn build(&self, element: &Element) -> Result<BuiltNode, TagParseError> {
        let builder = self.builders.get(&element.tag).ok_or_else(|| TagParseError {
            message: format!(
                "unknown DSL tag '<{}>' — no type registered for it (typo, or missing DslRegistry::register call?)",
                element.tag
            ),
        })?;
        let value = builder(&element.attrs).map_err(|e| TagParseError {
            message: format!("<{}>: {}", element.tag, e.message),
        })?;
        let children = element
            .children
            .iter()
            .map(|child| self.build(child))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(BuiltNode {
            tag: element.tag.clone(),
            value,
            children,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[derive(Debug, PartialEq)]
    struct RigidBodyTag {
        mass: f32,
    }

    impl DslTag for RigidBodyTag {
        const TAG_NAME: &'static str = "RigidBody";

        fn from_attrs(attrs: &[(String, String)]) -> Result<Self, TagParseError> {
            let mass = attrs
                .iter()
                .find(|(k, _)| k == "mass")
                .ok_or_else(|| TagParseError {
                    message: "missing attribute 'mass'".to_string(),
                })?
                .1
                .parse::<f32>()
                .map_err(|e| TagParseError {
                    message: format!("invalid 'mass': {e}"),
                })?;
            Ok(Self { mass })
        }
    }

    #[test]
    fn registered_tag_builds_and_downcasts() {
        let mut registry = DslRegistry::new();
        registry.register::<RigidBodyTag>();
        let element = parse(r#"<RigidBody mass="2.5" />"#).unwrap();
        let node = registry.build(&element).unwrap();
        assert_eq!(node.tag, "RigidBody");
        assert_eq!(node.downcast_ref::<RigidBodyTag>(), Some(&RigidBodyTag { mass: 2.5 }));
    }

    #[test]
    fn unregistered_tag_is_an_error_not_a_silent_skip() {
        let mut registry = DslRegistry::new();
        registry.register::<RigidBodyTag>();
        let element = parse(r#"<TotallyMadeUpTag />"#).unwrap();
        let err = registry.build(&element).unwrap_err();
        assert!(err.message.contains("unknown DSL tag"));
    }

    #[test]
    fn missing_required_attribute_is_an_error() {
        let mut registry = DslRegistry::new();
        registry.register::<RigidBodyTag>();
        let element = parse(r#"<RigidBody />"#).unwrap();
        let err = registry.build(&element).unwrap_err();
        assert!(err.message.contains("missing attribute 'mass'"));
    }

    #[test]
    fn nested_custom_tags_all_build() {
        #[derive(Debug, PartialEq)]
        struct EntityTag {
            name: String,
        }
        impl DslTag for EntityTag {
            const TAG_NAME: &'static str = "Entity";
            fn from_attrs(attrs: &[(String, String)]) -> Result<Self, TagParseError> {
                let name = attrs
                    .iter()
                    .find(|(k, _)| k == "name")
                    .ok_or_else(|| TagParseError {
                        message: "missing attribute 'name'".to_string(),
                    })?
                    .1
                    .clone();
                Ok(Self { name })
            }
        }

        let mut registry = DslRegistry::new();
        registry.register::<EntityTag>();
        registry.register::<RigidBodyTag>();
        let element = parse(r#"<Entity name="crate"><RigidBody mass="1.0" /></Entity>"#).unwrap();
        let node = registry.build(&element).unwrap();
        assert_eq!(
            node.downcast_ref::<EntityTag>(),
            Some(&EntityTag {
                name: "crate".to_string()
            })
        );
        assert_eq!(node.children.len(), 1);
        assert_eq!(
            node.children[0].downcast_ref::<RigidBodyTag>(),
            Some(&RigidBodyTag { mass: 1.0 })
        );
    }
}
