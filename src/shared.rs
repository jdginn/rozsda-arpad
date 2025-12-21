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
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_shared_value() {
        let shared = Shared::new(42);
        shared.with(|val| {
            assert_eq!(*val, 42);
        });
    }

    #[test]
    fn test_with_provides_immutable_access() {
        let shared = Shared::new(String::from("hello"));
        let result = shared.with(|s| s.len());
        assert_eq!(result, 5);
    }

    #[test]
    fn test_with_mut_allows_mutation() {
        let shared = Shared::new(10);
        shared.with_mut(|val| {
            *val += 5;
        });
        shared.with(|val| {
            assert_eq!(*val, 15);
        });
    }

    #[test]
    fn test_with_mut_returns_value() {
        let shared = Shared::new(vec![1, 2, 3]);
        let len = shared.with_mut(|vec| {
            vec.push(4);
            vec.len()
        });
        assert_eq!(len, 4);
    }

    #[test]
    fn test_clone_shares_same_inner_value() {
        let shared1 = Shared::new(100);
        let shared2 = shared1.clone();
        
        shared1.with_mut(|val| {
            *val = 200;
        });
        
        shared2.with(|val| {
            assert_eq!(*val, 200);
        });
    }

    #[test]
    fn test_clone_independent_wrappers() {
        let shared1 = Shared::new(vec![1, 2]);
        let shared2 = shared1.clone();
        
        shared2.with_mut(|vec| {
            vec.push(3);
        });
        
        shared1.with(|vec| {
            assert_eq!(vec.len(), 3);
            assert_eq!(*vec, vec![1, 2, 3]);
        });
    }

    #[test]
    fn test_multiple_clones_share_state() {
        let shared1 = Shared::new(0);
        let shared2 = shared1.clone();
        let shared3 = shared1.clone();
        
        shared1.with_mut(|val| *val += 1);
        shared2.with_mut(|val| *val += 2);
        shared3.with_mut(|val| *val += 3);
        
        shared1.with(|val| {
            assert_eq!(*val, 6);
        });
    }
}
