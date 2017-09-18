use std::sync::{Arc, RwLock};

use config::Config;


#[derive(Debug)]
pub struct ConfigCell {
    lock: RwLock<Option<Arc<Config>>>,
}

impl ConfigCell {
    pub fn new() -> ConfigCell {
        ConfigCell {
            lock: RwLock::new(None),
        }
    }
    pub fn put(&self, cfg: &Arc<Config>) {
        *self.lock.write().expect("config cell is not poisoned") =
            Some(cfg.clone());
    }
    pub fn get(&self) -> Option<Arc<Config>> {
        self.lock.read().expect("config cell is not poisoned").clone()
    }
}
