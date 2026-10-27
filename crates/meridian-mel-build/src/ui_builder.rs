//! [`UiBuilder`]: turns a `.meui` element tree into [`UiNode`]s, resolving
//! resources on the way.

use meridian_dsl_core::{Attribute, Element, MarkupExtension, MarkupValue};
use meridian_ui_core::ElementKind;

use crate::ui_node::{UiNode, UiNodeError, element_kind};
use crate::{
    InvalidMarkup, LocalResources, NamedElement, PropertyPath, ResourceSets, ResourceValue,
    UiLiteral,
};

/// Checks and converts the elements of one document. `scopes` holds the
/// `Resources` blocks of the elements being converted, innermost last.
pub(crate) struct UiBuilder<'a> {
    names: &'a [NamedElement],
    resources: &'a ResourceSets,
    scopes: Vec<LocalResources>,
}

impl<'a> UiBuilder<'a> {
    pub(crate) fn new(names: &'a [NamedElement], resources: &'a ResourceSets) -> Self {
        Self {
            names,
            resources,
            scopes: Vec::new(),
        }
    }

    pub(crate) fn node(&mut self, element: &Element) -> Result<UiNode, UiNodeError> {
        let tag = element.name.to_string();
        let Some(kind) = element_kind(element) else {
            return Err((element.span, InvalidMarkup::UnknownElement(tag)));
        };
        let own_scope = self.own_resources(element, &tag)?;
        let pushed = own_scope.is_some();
        self.scopes.extend(own_scope);
        let node = self.contents(element, kind, &tag);
        if pushed {
            self.scopes.pop();
        }
        node
    }

    /// The element's `<Tag.Resources>` block, if it has one.
    fn own_resources(
        &self,
        element: &Element,
        tag: &str,
    ) -> Result<Option<LocalResources>, UiNodeError> {
        let mut blocks = element.children.iter().filter(|c| c.is_property_element());
        let Some(block) = blocks.next() else {
            return Ok(None);
        };
        let is_resources = block.name.owner() == Some(tag) && block.name.local() == "Resources";
        if !is_resources {
            let reason = InvalidMarkup::UnsupportedPropertyElement(block.name.to_string());
            return Err((block.span, reason));
        }
        if let Some(second) = blocks.next() {
            let reason = InvalidMarkup::DuplicateResourcesBlock(tag.to_string());
            return Err((second.span, reason));
        }
        LocalResources::from_element(block).map(Some)
    }

    fn contents(
        &mut self,
        element: &Element,
        kind: ElementKind,
        tag: &str,
    ) -> Result<UiNode, UiNodeError> {
        let field = element
            .directive("Name")
            .and_then(Attribute::text)
            .and_then(|name| self.names.iter().find(|named| named.name == name))
            .map(|named| named.field.clone());
        let mut literals = Vec::new();
        for attr in element.attrs.iter().filter(|a| a.name.prefix().is_none()) {
            literals.extend(self.literal(tag, attr)?);
        }
        let children = element
            .children
            .iter()
            .filter(|child| !child.is_property_element())
            .map(|child| self.node(child))
            .collect::<Result<_, _>>()?;
        Ok(UiNode {
            kind,
            field,
            literals,
            children,
        })
    }

    /// `None` for an `{x:Bind}`, which `apply_bindings` writes.
    fn literal(&self, tag: &str, attr: &Attribute) -> Result<Option<UiLiteral>, UiNodeError> {
        let at = |reason| (attr.span, reason);
        let text = match &attr.value {
            MarkupValue::Text(text) => self.resources.resolve(text).map_err(at)?,
            MarkupValue::Extension(extension) if extension.name.is_directive("Bind") => {
                return Ok(None);
            }
            MarkupValue::Extension(extension) if extension.name.is_plain("StaticResource") => {
                self.static_resource(extension).map_err(at)?.literal()
            }
            MarkupValue::Extension(extension) => {
                return Err(at(InvalidMarkup::UnsupportedExtension(
                    extension.name.to_string(),
                )));
            }
        };
        Ok(Some(UiLiteral {
            path: PropertyPath::resolve(tag, &attr.name).map_err(at)?,
            property: attr.name.to_string(),
            text,
            span: attr.span,
        }))
    }

    /// `{StaticResource Key}`: the nearest `Resources` block that defines
    /// `Key`, or the `.mres` entry when the key is `Set.name`.
    fn static_resource(
        &self,
        extension: &MarkupExtension,
    ) -> Result<&ResourceValue, InvalidMarkup> {
        let [MarkupValue::Text(key)] = extension.positional.as_slice() else {
            return Err(InvalidMarkup::InvalidStaticResource);
        };
        if !extension.named.is_empty() {
            return Err(InvalidMarkup::InvalidStaticResource);
        }
        let local = self.scopes.iter().rev().find_map(|scope| scope.get(key));
        local
            .or_else(|| self.resources.entry(key).map(|entry| &entry.value))
            .ok_or_else(|| InvalidMarkup::UnknownStaticResource(key.clone()))
    }
}
