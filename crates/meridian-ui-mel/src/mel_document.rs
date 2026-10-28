//! [`MelDocument`]: a parsed `.meui` document checked against the
//! code-behind rules of ADR 017.

use std::path::{Path, PathBuf};

use meridian_markup::{Attribute, Element, MarkupValue, parse};
use meridian_ui_domain::UiEventTag;

use crate::{
    BuildError, EventHandler, InvalidMarkup, NamedElement, PropertyBinding, PropertyPath,
    ResourceSets, UiNode,
};

const ROOT_ONLY: [&str; 2] = ["Class", "ViewModel"];
const ANYWHERE: [&str; 2] = ["Name", "Key"];

#[derive(Debug)]
pub struct MelDocument {
    pub path: PathBuf,
    pub root: Element,
    /// Every `x:Name`, in document order.
    pub names: Vec<NamedElement>,
    /// The root's `x:Class`, a Rust type path.
    pub class: Option<String>,
    /// The root's `x:ViewModel`, a Rust type path.
    pub view_model: Option<String>,
    /// Every `{x:Bind}`, in document order.
    pub bindings: Vec<PropertyBinding>,
    /// Every `event="{x:Bind method}"`, in document order.
    pub events: Vec<EventHandler>,
    /// The element tree to build, when the root is a UI element.
    pub ui: Option<UiNode>,
}

impl MelDocument {
    /// Parses and checks `source` with no resources; `path` is only used
    /// in errors and in the generated `include_str!`.
    pub fn parse(path: &Path, source: &str) -> Result<Self, BuildError> {
        Self::parse_with(path, source, &ResourceSets::default())
    }

    /// Like [`Self::parse`], resolving `@res:Set.name` against `resources`.
    pub fn parse_with(
        path: &Path,
        source: &str,
        resources: &ResourceSets,
    ) -> Result<Self, BuildError> {
        let root = parse(source).map_err(|error| BuildError::Parse {
            path: path.to_path_buf(),
            error,
        })?;
        let mut document = Self {
            path: path.to_path_buf(),
            class: type_path(path, &root, "Class")?,
            view_model: type_path(path, &root, "ViewModel")?,
            root,
            names: Vec::new(),
            bindings: Vec::new(),
            events: Vec::new(),
            ui: None,
        };
        let root = document.root.clone();
        document.collect(&root, true)?;
        if UiNode::is_ui_root(&root) {
            let ui = UiNode::from_element(&root, &document.names, resources)
                .map_err(|(span, reason)| BuildError::invalid(path, span, reason))?;
            document.ui = Some(ui);
        }
        Ok(document)
    }

    fn collect(&mut self, element: &Element, is_root: bool) -> Result<(), BuildError> {
        for attr in &element.attrs {
            self.check_directive(element, attr, is_root)?;
        }
        for attr in &element.attrs {
            self.collect_binding(element, attr)?;
        }
        element
            .children
            .iter()
            .try_for_each(|child| self.collect(child, false))
    }

    fn check_directive(
        &mut self,
        element: &Element,
        attr: &Attribute,
        is_root: bool,
    ) -> Result<(), BuildError> {
        if attr.name.prefix().is_none() {
            return Ok(());
        }
        let local = attr.name.local();
        let fail = |reason| Err(BuildError::invalid(&self.path, attr.span, reason));
        if !ROOT_ONLY.contains(&local) && !ANYWHERE.contains(&local) {
            return fail(InvalidMarkup::UnknownDirective(local.to_string()));
        }
        if ROOT_ONLY.contains(&local) && !is_root {
            return fail(InvalidMarkup::RootOnlyDirective(local.to_string()));
        }
        let Some(text) = attr.text() else {
            return fail(InvalidMarkup::DirectiveNotText(local.to_string()));
        };
        if local == "Name" {
            let named = NamedElement::new(text, element.span)
                .map_err(|reason| BuildError::invalid(&self.path, attr.span, reason))?;
            self.add_name(named, attr)?;
        }
        Ok(())
    }

    /// Records `attr` if its value is an `{x:Bind}`; runs after the
    /// element's directives, so its `x:Name` is known.
    fn collect_binding(&mut self, element: &Element, attr: &Attribute) -> Result<(), BuildError> {
        let MarkupValue::Extension(extension) = &attr.value else {
            return Ok(());
        };
        if !extension.name.is_directive("Bind") {
            return Ok(());
        }
        let fail = |reason| Err(BuildError::invalid(&self.path, attr.span, reason));
        if self.view_model.is_none() {
            return fail(InvalidMarkup::BindingWithoutViewModel);
        }
        let named = element
            .directive("Name")
            .and_then(Attribute::text)
            .and_then(|name| self.names.iter().find(|named| named.name == name));
        let Some(named) = named else {
            return fail(InvalidMarkup::BindingWithoutName(element.name.to_string()));
        };
        let tag = element.name.to_string();
        if let Some(event) = Self::event_tag(&tag, attr) {
            let handler =
                EventHandler::new(named, &attr.name.to_string(), event, extension, attr.span)
                    .map_err(|reason| BuildError::invalid(&self.path, attr.span, reason))?;
            self.events.push(handler);
            return Ok(());
        }
        let binding = PropertyBinding::new(named, &tag, &attr.name, extension, attr.span)
            .map_err(|reason| BuildError::invalid(&self.path, attr.span, reason))?;
        self.bindings.push(binding);
        Ok(())
    }

    /// The event `attr` names on `<tag>`: a plain attribute that is not
    /// one of the element's properties (`text` stays the property).
    fn event_tag(tag: &str, attr: &Attribute) -> Option<UiEventTag> {
        if attr.name.owner().is_some() || PropertyPath::resolve(tag, &attr.name).is_ok() {
            return None;
        }
        UiEventTag::from_attribute(attr.name.local())
    }

    fn add_name(&mut self, named: NamedElement, attr: &Attribute) -> Result<(), BuildError> {
        let clash = self
            .names
            .iter()
            .find(|existing| existing.variant == named.variant);
        let reason = match clash {
            None => {
                self.names.push(named);
                return Ok(());
            }
            Some(existing) if existing.name == named.name => {
                InvalidMarkup::DuplicateName(named.name)
            }
            Some(existing) => InvalidMarkup::NameCollision {
                first: existing.name.clone(),
                second: named.name,
            },
        };
        Err(BuildError::invalid(&self.path, attr.span, reason))
    }
}

/// The root's `x:<directive>` as a Rust type path (`crate::scene::Hud`).
fn type_path(path: &Path, root: &Element, directive: &str) -> Result<Option<String>, BuildError> {
    let Some(attr) = root.directive(directive) else {
        return Ok(None);
    };
    let Some(text) = attr.text() else {
        return Ok(None);
    };
    let valid = !text.is_empty()
        && text.split("::").all(|segment| {
            segment.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        });
    if valid {
        Ok(Some(text.to_string()))
    } else {
        let reason = InvalidMarkup::InvalidTypePath(text.to_string());
        Err(BuildError::invalid(path, attr.span, reason))
    }
}

#[cfg(test)]
mod tests;
