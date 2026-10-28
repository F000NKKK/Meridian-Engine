//! [`ResourceIdTable`]: key to runtime identity, and back.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

use meridian_memory::Handle;

use super::{BindError, Binding};
use crate::ResourceId;

/// Maps an external, stable key (an asset's id from its `.meta`) to the
/// [`ResourceId`] of the resource loaded for it, and back. Two answers only:
/// *which keys exist* ([`declare`](Self::declare), filled from the `.meta`
/// files) and *which of them are loaded as what* ([`bind`](Self::bind) /
/// [`unbind`](Self::unbind)). It does not load anything, hold a resource
/// or decide when one is released: the code that loads binds, the code that
/// frees unbinds. Generic over the key so `resource` knows no asset format.
pub struct ResourceIdTable<K, T> {
    bindings: HashMap<K, Binding<T>>,
    keys: HashMap<Handle, K>,
}

impl<K: Eq + Hash + Copy, T> ResourceIdTable<K, T> {
    /// A table that knows no key.
    #[must_use]
    pub fn new() -> Self {
        ResourceIdTable {
            bindings: HashMap::new(),
            keys: HashMap::new(),
        }
    }

    /// A table that knows `keys`, all unbound.
    #[must_use]
    pub fn declaring(keys: impl IntoIterator<Item = K>) -> Self {
        let mut table = Self::new();
        for key in keys {
            table.declare(key);
        }
        table
    }

    /// Makes `key` known; nothing changes if it already is.
    pub fn declare(&mut self, key: K) {
        self.bindings.entry(key).or_insert(Binding::Unbound);
    }

    /// Records that `key`'s resource is `id`.
    ///
    /// # Errors
    ///
    /// [`BindError`] when the key is unknown or already bound, or `id` is
    /// already another key's identity.
    pub fn bind(&mut self, key: K, id: ResourceId<T>) -> Result<(), BindError> {
        match self.bindings.get(&key) {
            None => return Err(BindError::UnknownKey),
            Some(Binding::Bound(_)) => return Err(BindError::AlreadyBound),
            Some(Binding::Unbound) => {}
        }
        if self.keys.contains_key(&id.handle) {
            return Err(BindError::ResourceInUse);
        }
        self.bindings.insert(key, Binding::Bound(id));
        self.keys.insert(id.handle, key);
        Ok(())
    }

    /// Forgets `key`'s resource (it was released), keeping the key known.
    /// Returns the identity it had.
    pub fn unbind(&mut self, key: K) -> Option<ResourceId<T>> {
        let id = self.bindings.get(&key).copied().and_then(Binding::id)?;
        self.bindings.insert(key, Binding::Unbound);
        self.keys.remove(&id.handle);
        Some(id)
    }

    /// What the table knows about `key`: `None` when it never heard of it.
    #[must_use]
    pub fn binding(&self, key: K) -> Option<Binding<T>> {
        self.bindings.get(&key).copied()
    }

    /// `key`'s resource, if it is bound.
    #[must_use]
    pub fn resource(&self, key: K) -> Option<ResourceId<T>> {
        self.binding(key).and_then(Binding::id)
    }

    /// The key whose resource is `id`. A stale handle (its generation
    /// replaced) is not found.
    #[must_use]
    pub fn key_of(&self, id: ResourceId<T>) -> Option<K> {
        self.keys.get(&id.handle).copied()
    }

    /// How many keys are known.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether no key is known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl<K: Eq + Hash + Copy, T> Default for ResourceIdTable<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> fmt::Debug for ResourceIdTable<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceIdTable")
            .field("keys", &self.bindings.len())
            .field("bound", &self.keys.len())
            .finish()
    }
}
