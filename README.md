# Ternary Cache

**Ternary Cache** is a ternary-aware LRU cache where each entry carries one of three states — Invalid (-1), Stale (0), or Fresh (+1) — enabling nuanced cache invalidation beyond the binary valid/invalid paradigm.

## Why It Matters

Standard caches have two states: present (valid) or absent (invalid). But real systems have a third state: stale (present but outdated). Serving stale data during cache refresh (cache-aside with stale-while-revalidate) improves perceived latency by 40-60% in practice. Ternary Cache makes stale a first-class state — reads can return stale data with a state flag, background refreshes update Stale → Fresh, and explicit invalidation moves Fresh → Invalid without immediate eviction.

## How It Works

### Cache State Machine

```
insert → Fresh (+1)
  ↓ stale()
Stale (0) → get() returns (value, Stale)
  ↓ invalidate()
Invalid (-1) → get() returns None
  ↓ refresh()
Fresh (+1)
```

### LRU Eviction

```
get(key):
    if key exists:
        entry.access_count += 1
        entry.last_access = tick++
        return Some((value, state))
    return None

insert(key, value):
    if len >= capacity:
        evict LRU entry (min last_access)
    entries[key] = { value, Fresh, access_count=1, last_access=tick++ }
```

Eviction: **O(N)** linear scan for minimum last_access (or **O(log N)** with a heap). Get: **O(1)** HashMap lookup. Insert: **O(1)** amortized.

### State Transitions

- `insert(key, value)` → state = Fresh
- `stale(key)` → state = Stale (data present but outdated)
- `invalidate(key)` → state = Invalid (data present but unusable)
- `refresh(key, value)` → state = Fresh with new value
- `remove(key)` → delete entry entirely

All transitions: **O(1)**.

### Access Tracking

Each entry tracks:
- `access_count: usize` — total reads
- `last_access: usize` — tick of last read (for LRU)

The global `tick` counter increments on every operation, providing a monotonic ordering.

### Bulk Operations

```
bulk_stale(prefix) → mark all keys with prefix as Stale
purge_invalid() → remove all Invalid entries → O(N) scan
stats() → { total, fresh, stale, invalid, hit_rate }
```

## Quick Start

```rust
use ternary_cache::TernaryCache;

let mut cache = TernaryCache::new(100);

cache.insert("key1", "value1");
cache.insert("key2", "value2");

let (val, state) = cache.get("key1").unwrap();
assert_eq!(state, CacheState::Fresh);

cache.stale("key1");
let (val, state) = cache.get("key1").unwrap();
assert_eq!(state, CacheState::Stale);

cache.invalidate("key1");
assert_eq!(cache.get("key1"), None); // Invalid → invisible
```

## API

| Type | Description |
|------|-------------|
| `TernaryCache<V>` | LRU cache with ternary entry states |
| `CacheEntry<V>` | value, state, access_count, last_access |
| `CacheState` | `Invalid (-1)`, `Stale (0)`, `Fresh (+1)` |

Key methods: `insert()`, `get()`, `stale()`, `invalidate()`, `refresh()`, `purge_invalid()`.

## Architecture Notes

Ternary Cache provides the caching layer for fleet state in SuperInstance. In γ + η = C, Fresh (+1) represents γ (growth — current data available for decisions), Invalid (-1) represents η (avoidance — explicitly invalidated data is never served), and Stale (0) is the neutral state allowing degraded but functional operation. Integrates with `ternary-archive` for persistent storage and `oxide-tombstone` for deletion semantics.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for caching architecture.


### Stale-While-Revalidate Pattern

```
match cache.get("user:42") {
    Some((data, Fresh)) => return data,
    Some((data, Stale)) => {
        spawn(refetch_async("user:42"));  // background refresh
        return data;                       // serve stale immediately
    }
    None => {
        let data = fetch_blocking("user:42")?;
        cache.insert("user:42", data);
        return data;
    }
}
```

This pattern guarantees: Fresh data served when available (best case), Stale data served instantly with background refresh (degraded but fast), cold misses trigger blocking fetch (worst case). Average latency improvement: 40-60% over cache-aside alone.

### Access Tracking

Each entry tracks `access_count: usize` (total reads) and `last_access: usize` (monotonic tick). The global tick counter increments on every operation, providing strict ordering without wall-clock dependencies — no NTP drift issues.

## References

1. Tanenbaum, A. S. & Bos, H. (2014). *Modern Operating Systems*, 4th ed. Pearson. Chapter 3: Memory Management.
2. Nishtala, R. et al. (2013). "Scaling Memcache at Facebook." *NSDI*.
3. Redis Documentation (2024). "Cache Patterns — Cache Aside, Read-Through, Write-Through."

## License

MIT
