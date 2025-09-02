use std::hash::{Hash, Hasher};

use ahash::AHasher;

#[derive(Debug, Clone, Eq)]
pub struct HashedKey {
    pub key: String,
    hash: u64,
}

impl Hash for HashedKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl PartialEq for HashedKey {
    fn eq(&self, other: &HashedKey) -> bool {
        self.key == other.key
    }
}

impl HashedKey {
    pub fn new(key: String) -> Self {
        let mut hasher = AHasher::default();

        key.hash(&mut hasher);

        Self {
            key,
            hash: hasher.finish(),
        }
    }
}
