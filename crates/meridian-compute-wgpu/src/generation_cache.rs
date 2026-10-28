//! A value rebuilt only when the buffers it was built over change.

/// Holds one value together with the key it was built for. Used for bind
/// groups: the key is the [`generation`](crate::ScratchSlot::generation) of
/// every slot the group binds, so the group is rebuilt exactly when one of
/// those buffers was reallocated.
#[derive(Debug)]
pub struct GenerationCache<T> {
    key: Vec<u64>,
    value: Option<T>,
}

impl<T> Default for GenerationCache<T> {
    fn default() -> Self {
        Self {
            key: Vec::new(),
            value: None,
        }
    }
}

impl<T> GenerationCache<T> {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The cached value if it was built for `key`, else the result of `build`
    /// (which is then cached for `key`).
    pub fn reuse_or_build(&mut self, key: &[u64], build: impl FnOnce() -> T) -> &T {
        if self.key != key {
            self.key = key.to_vec();
            self.value = None;
        }
        self.value.get_or_insert_with(build)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_key_builds_once() {
        let mut cache = GenerationCache::new();
        let mut builds = 0;
        for _ in 0..3 {
            cache.reuse_or_build(&[1, 1], || {
                builds += 1;
                "group"
            });
        }
        assert_eq!(builds, 1);
    }

    #[test]
    fn a_changed_generation_rebuilds() {
        let mut cache = GenerationCache::new();
        assert_eq!(*cache.reuse_or_build(&[1, 1], || "old"), "old");
        assert_eq!(*cache.reuse_or_build(&[1, 2], || "new"), "new");
        assert_eq!(*cache.reuse_or_build(&[1, 2], || "unused"), "new");
    }
}
