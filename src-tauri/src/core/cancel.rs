use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Clone)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CancelRegistry {
    tokens: RwLock<HashMap<String, CancelToken>>,
}

impl CancelRegistry {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, task_id: &str) -> CancelToken {
        let token = CancelToken::new();
        if let Ok(mut map) = self.tokens.write() {
            map.insert(task_id.to_string(), token.clone());
        }
        token
    }

    /// Cancel a task. Returns `true` if the task was found and cancelled, `false` otherwise.
    pub fn cancel(&self, task_id: &str) -> bool {
        if let Ok(map) = self.tokens.read() {
            if let Some(token) = map.get(task_id) {
                token.cancel();
                return true;
            }
        }
        false
    }

    pub fn is_cancelled(&self, task_id: &str) -> bool {
        if let Ok(map) = self.tokens.read() {
            map.get(task_id)
                .map(|t| t.is_cancelled())
                .unwrap_or(false)
        } else {
            false
        }
    }

    pub fn contains(&self, task_id: &str) -> bool {
        if let Ok(map) = self.tokens.read() {
            map.contains_key(task_id)
        } else {
            false
        }
    }

    pub fn unregister(&self, task_id: &str) {
        if let Ok(mut map) = self.tokens.write() {
            map.remove(task_id);
        }
    }

    #[allow(dead_code)]
    pub fn take(&self, task_id: &str) -> Option<CancelToken> {
        if let Ok(mut map) = self.tokens.write() {
            map.remove(task_id)
        } else {
            None
        }
    }
}

pub static CANCEL_REGISTRY: LazyLock<CancelRegistry> = LazyLock::new(CancelRegistry::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_check() {
        let registry = CancelRegistry::new();
        let task_id = "test-1";
        assert!(!registry.contains(task_id));
        assert!(!registry.is_cancelled(task_id));

        registry.register(task_id);
        assert!(registry.contains(task_id));
        assert!(!registry.is_cancelled(task_id));
    }

    #[test]
    fn test_cancel() {
        let registry = CancelRegistry::new();
        let task_id = "test-cancel";
        registry.register(task_id);
        assert!(!registry.is_cancelled(task_id));

        assert!(registry.cancel(task_id));
        assert!(registry.is_cancelled(task_id));
    }

    #[test]
    fn test_cancel_non_existent_returns_false() {
        let registry = CancelRegistry::new();
        assert!(!registry.cancel("non-existent"));
    }

    #[test]
    fn test_is_cancelled_non_existent() {
        let registry = CancelRegistry::new();
        assert!(!registry.is_cancelled("non-existent"));
    }

    #[test]
    fn test_contains_non_existent() {
        let registry = CancelRegistry::new();
        assert!(!registry.contains("non-existent"));
    }

    #[test]
    fn test_unregister() {
        let registry = CancelRegistry::new();
        let task_id = "test-unreg";
        registry.register(task_id);
        assert!(registry.contains(task_id));

        registry.unregister(task_id);
        assert!(!registry.contains(task_id));
        assert!(!registry.is_cancelled(task_id));
    }

    #[test]
    fn test_take() {
        let registry = CancelRegistry::new();
        let task_id = "test-take";
        registry.register(task_id);
        assert!(registry.contains(task_id));

        let token = registry.take(task_id);
        assert!(token.is_some());
        assert!(!registry.contains(task_id));
        assert!(!token.unwrap().is_cancelled());
    }

    #[test]
    fn test_take_non_existent() {
        let registry = CancelRegistry::new();
        assert!(registry.take("non-existent").is_none());
    }

    #[test]
    fn test_cancel_token_independence() {
        let registry = CancelRegistry::new();
        registry.register("task-a");
        registry.register("task-b");

        registry.cancel("task-a");
        assert!(registry.is_cancelled("task-a"));
        assert!(!registry.is_cancelled("task-b"));
    }

    #[test]
    fn test_double_cancel_is_idempotent() {
        let registry = CancelRegistry::new();
        let task_id = "test-double";
        registry.register(task_id);
        assert!(registry.cancel(task_id));
        assert!(registry.cancel(task_id));
        assert!(registry.is_cancelled(task_id));
    }

    #[test]
    fn test_cancel_token_clone_shares_state() {
        let token1 = CancelToken::new();
        let token2 = token1.clone();

        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        token1.cancel();
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_cancel_token_default() {
        let token: CancelToken = Default::default();
        assert!(!token.is_cancelled());
    }
}
