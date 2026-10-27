//! Reading and writing property values.

use super::{Change, ChangeSource, UiTree};
use crate::property::Invalidates;
use crate::{
    NodeId, Property, PropertyId, PropertyMeta, PropertyType, PropertyValue, UiError, properties,
};

impl UiTree {
    /// Writes a property from code. A write that does not change the value
    /// records nothing and invalidates nothing.
    pub fn set<T: PropertyType>(
        &mut self,
        node: NodeId,
        property: Property<T>,
        value: T,
    ) -> Result<(), UiError> {
        self.store(
            node,
            property.meta(),
            value.into_value(),
            ChangeSource::Code,
        )
    }

    /// Like [`Self::set`], but journaled as a direct user edit so that
    /// `TwoWay` bindings write it back to the view model.
    pub fn set_by_user<T: PropertyType>(
        &mut self,
        node: NodeId,
        property: Property<T>,
        value: T,
    ) -> Result<(), UiError> {
        self.store(
            node,
            property.meta(),
            value.into_value(),
            ChangeSource::User,
        )
    }

    /// Reads a property; unset properties read as their default.
    pub fn get<T: PropertyType>(&self, node: NodeId, property: Property<T>) -> Result<T, UiError> {
        let meta = property.meta();
        T::from_value(&self.value(node, &meta)?).ok_or(UiError::ValueKind(meta.id))
    }

    /// Run-time-typed read for tooling (inspector, designer).
    pub fn value_of(&self, node: NodeId, id: PropertyId) -> Result<PropertyValue, UiError> {
        let meta = properties::meta(id).ok_or(UiError::UnknownProperty(id))?;
        self.value(node, meta)
    }

    /// Run-time-typed write for tooling; the value must be of the
    /// property's kind.
    pub fn set_value(
        &mut self,
        node: NodeId,
        id: PropertyId,
        value: PropertyValue,
    ) -> Result<(), UiError> {
        let meta = properties::meta(id).ok_or(UiError::UnknownProperty(id))?;
        if std::mem::discriminant(&value) != std::mem::discriminant(&(meta.default)()) {
            return Err(UiError::ValueKind(id));
        }
        self.store(node, *meta, value, ChangeSource::Code)
    }

    /// Reads a property for layout, where a stale node simply reads as
    /// the type's default.
    pub(crate) fn get_or_default<T: PropertyType + Default>(
        &self,
        node: NodeId,
        property: Property<T>,
    ) -> T {
        self.get(node, property).unwrap_or_default()
    }

    fn value(&self, node: NodeId, meta: &PropertyMeta) -> Result<PropertyValue, UiError> {
        let stored = self.element(node)?.value(meta.id);
        Ok(stored.cloned().unwrap_or_else(meta.default))
    }

    fn store(
        &mut self,
        node: NodeId,
        meta: PropertyMeta,
        value: PropertyValue,
        source: ChangeSource,
    ) -> Result<(), UiError> {
        if self.value(node, &meta)? == value {
            return Ok(());
        }
        self.element_mut(node)?.put(meta.id, value);
        if meta.invalidates == Invalidates::Layout {
            self.layout_dirty = true;
        }
        self.changes.push(Change {
            node,
            property: meta.id,
            source,
        });
        Ok(())
    }
}
