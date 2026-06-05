//! Ternary cache: caching with entries in {-1=invalid, 0=stale, +1=fresh} states.

use std::collections::HashMap;

/// Cache entry state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CacheState { Invalid, Stale, Fresh }

impl CacheState {
    pub fn to_i8(self) -> i8 { match self { CacheState::Invalid => -1, CacheState::Stale => 0, CacheState::Fresh => 1 } }
}

/// Cache entry
#[derive(Clone, Debug)]
pub struct CacheEntry<V: Clone> {
    pub value: V,
    pub state: CacheState,
    pub access_count: usize,
    pub last_access: usize,
}

/// Ternary-aware LRU cache
pub struct TernaryCache<V: Clone> {
    entries: HashMap<String, CacheEntry<V>>,
    capacity: usize,
    tick: usize,
}

impl<V: Clone> TernaryCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self { entries: HashMap::new(), capacity, tick: 0 }
    }

    pub fn insert(&mut self, key: &str, value: V) {
        self.evict_if_needed();
        self.tick += 1;
        self.entries.insert(key.to_string(), CacheEntry {
            value, state: CacheState::Fresh, access_count: 1, last_access: self.tick,
        });
    }

    pub fn get(&mut self, key: &str) -> Option<(V, CacheState)> {
        self.tick += 1;
        if let Some(entry) = self.entries.get_mut(key) {
            entry.access_count += 1;
            entry.last_access = self.tick;
            Some((entry.value.clone(), entry.state))
        } else { None }
    }

    pub fn invalidate(&mut self, key: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.state = CacheState::Invalid;
            true
        } else { false }
    }

    pub fn stale(&mut self, key: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.state = CacheState::Stale;
            true
        } else { false }
    }

    pub fn refresh(&mut self, key: &str, value: V) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.value = value;
            entry.state = CacheState::Fresh;
            true
        } else { false }
    }

    fn evict_if_needed(&mut self) {
        if self.entries.len() >= self.capacity {
            // Evict least recently used invalid entries first, then stale, then oldest
            let mut best_key: Option<String> = None;
            let mut best_score: f64 = f64::MAX;
            for (key, entry) in &self.entries {
                let state_score = match entry.state {
                    CacheState::Invalid => -2.0,
                    CacheState::Stale => -1.0,
                    CacheState::Fresh => 0.0,
                };
                let score = state_score + entry.last_access as f64 * 0.001;
                if score < best_score {
                    best_score = score;
                    best_key = Some(key.clone());
                }
            }
            if let Some(key) = best_key { self.entries.remove(&key); }
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn state_distribution(&self) -> (usize, usize, usize) {
        let fresh = self.entries.values().filter(|e| e.state == CacheState::Fresh).count();
        let stale = self.entries.values().filter(|e| e.state == CacheState::Stale).count();
        let invalid = self.entries.values().filter(|e| e.state == CacheState::Invalid).count();
        (fresh, stale, invalid)
    }

    /// TTL-based expiration: mark entries older than ttl ticks as stale
    pub fn expire(&mut self, ttl: usize) -> usize {
        let mut expired = 0;
        let current = self.tick;
        for entry in self.entries.values_mut() {
            if entry.state == CacheState::Fresh && current - entry.last_access > ttl {
                entry.state = CacheState::Stale;
                expired += 1;
            }
        }
        expired
    }

    /// Hit rate based on access patterns
    pub fn hit_rate(&self, hits: usize, misses: usize) -> f64 {
        let total = hits + misses;
        if total == 0 { 0.0 } else { hits as f64 / total as f64 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut cache = TernaryCache::<i32>::new(10);
        cache.insert("a", 42);
        let (val, state) = cache.get("a").unwrap();
        assert_eq!(val, 42);
        assert_eq!(state, CacheState::Fresh);
    }

    #[test]
    fn test_miss() {
        let mut cache = TernaryCache::<i32>::new(10);
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn test_invalidate() {
        let mut cache = TernaryCache::<i32>::new(10);
        cache.insert("a", 1);
        cache.invalidate("a");
        let (_, state) = cache.get("a").unwrap();
        assert_eq!(state, CacheState::Invalid);
    }

    #[test]
    fn test_stale() {
        let mut cache = TernaryCache::<i32>::new(10);
        cache.insert("a", 1);
        cache.stale("a");
        let (_, state) = cache.get("a").unwrap();
        assert_eq!(state, CacheState::Stale);
    }

    #[test]
    fn test_refresh() {
        let mut cache = TernaryCache::<i32>::new(10);
        cache.insert("a", 1);
        cache.stale("a");
        cache.refresh("a", 99);
        let (val, state) = cache.get("a").unwrap();
        assert_eq!(val, 99);
        assert_eq!(state, CacheState::Fresh);
    }

    #[test]
    fn test_eviction() {
        let mut cache = TernaryCache::<i32>::new(3);
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);
        cache.invalidate("b");
        cache.insert("d", 4); // should evict b (invalid)
        assert_eq!(cache.len(), 3);
        assert!(cache.get("a").is_some());
        assert!(cache.get("d").is_some());
    }

    #[test]
    fn test_state_distribution() {
        let mut cache = TernaryCache::<i32>::new(10);
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.stale("a");
        let (fresh, stale, invalid) = cache.state_distribution();
        assert_eq!(fresh, 1);
        assert_eq!(stale, 1);
        assert_eq!(invalid, 0);
    }

    #[test]
    fn test_ttl_expiration() {
        let mut cache = TernaryCache::<i32>::new(10);
        cache.insert("a", 1);
        // Simulate time passing
        for _ in 0..20 { cache.tick += 1; }
        let expired = cache.expire(5);
        assert_eq!(expired, 1);
    }
}
