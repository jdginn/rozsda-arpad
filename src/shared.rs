use std::sync::{Arc, Mutex};

pub struct Shared<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    pub fn with<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
        let guard = self.inner.lock().unwrap();
        f(&*guard)
    }

    pub fn with_mut<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> R {
        let mut guard = self.inner.lock().unwrap();
        f(&mut *guard)
    }

    pub fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
