use std::collections::HashMap;

/// This is a type that contain hashmap which is used as a memory storage
pub struct KvStore {
    store: HashMap<String, String>,
}

/// Methods of type KvStore which consists of usual key value store operation like get, set and
// remove
impl KvStore {
    /// Returns a KvStore with Hashmap created
    pub fn new() -> Self {
        KvStore {
            store: HashMap::new(),
        }
    }
    /// Get data based on given key
    ///
    /// # Arguments
    ///
    /// * `key` - A string that holds the key
    pub fn get(&self, key: String) -> Option<String> {
        if let Some(value) = self.store.get(&key) {
            return Some(value.into());
        }
        None
    }
    /// Create key-value record entry
    ///
    /// # Arguments
    ///
    /// * `key` - A string that holds the key
    /// * `value` - A string that holds the value to store
    pub fn set(&mut self, key: String, value: String) -> Option<String> {
        self.store.insert(key, value)
    }
    /// Remove data based on given key
    ///
    /// # Arguments
    ///
    /// * `key` - A string that holds the key to the targeted entry
    pub fn remove(&mut self, key: String) -> Option<String> {
        self.store.remove(&key)
    }
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}
