use std::any::Any;
use std::sync::RwLock;

/// Typed metadata storage for rooms and clients.
///
/// Each room and client holds exactly one metadata blob — a user-defined
/// struct. The library does not interpret metadata.
pub struct Storage {
    inner: RwLock<Option<Box<dyn Any + Send + Sync + 'static>>>,
}

impl Storage {
    /// Create an empty storage.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    /// Store a typed value. Replaces any previous value.
    #[inline]
    pub fn set<T: Any + Send + Sync + 'static>(&self, value: T) {
        *self.inner.write().unwrap() = Some(Box::new(value));
    }

    /// Read the stored value via a callback.
    ///
    /// The callback receives `&T` if the stored type matches. Returns `None`
    /// if no value is set or the type doesn't match.
    ///
    /// The callback pattern avoids lifetime issues with concurrent guards —
    /// no `Clone` required.
    #[inline]
    pub fn get<T: Any + Send + Sync + 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        let guard = self.inner.read().unwrap();
        let any_ref = guard.as_ref()?;
        let typed = any_ref.downcast_ref::<T>()?;
        Some(f(typed))
    }

    /// Remove and return the stored value, downcasted to `T`.
    ///
    /// Returns `None` if no value is set or the type doesn't match.
    #[inline]
    pub fn take<T: Any + Send + Sync + 'static>(&self) -> Option<T> {
        let any_box = self.inner.write().unwrap().take()?;
        any_box.downcast::<T>().ok().map(|boxed| *boxed)
    }

    /// Check if a value is set.
    #[inline]
    pub fn is_set(&self) -> bool {
        self.inner.read().unwrap().is_some()
    }
}

impl Default for Storage {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
