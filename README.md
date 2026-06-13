# ternary-cache

A ternary-aware LRU cache where every entry exists in one of three coherence states — **Fresh (+1)**, **Stale (0)**, or **Invalid (−1)** — enabling fine-grained cache invalidation strategies that binary (valid/invalid) caches cannot express.

## Why It Matters

Classical caches use a single validity bit: an entry is either valid or it is not. In distributed agent fleets, network partitions and eventual consistency make this model too rigid. A stale entry — one that is probably still correct but has not been recently confirmed — carries different semantics than an entry known to be wrong.

Ternary caching maps cleanly onto the **γ (state) + η (decision) = C (constraint)** framework of the SuperInstance ecosystem:

| Symbol | Meaning in this crate |
|--------|----------------------|
| γ | `CacheState` ∈ {Invalid(−1), Stale(0), Fresh(+1)} |
| η | Eviction & TTL policy: which entries to reclaim, which to refresh |
| C | Capacity constraint: `len ≤ capacity`, enforced via priority eviction |

The third state (Stale) allows **read-through with background refresh**: serve the stale value immediately while scheduling an async revalidation, rather than blocking the requestor or serving known-bad data.

## How It Works

### State Machine

```
         insert / refresh
  ───────────────────────────▶
  │                           │
  │   stale()    refresh()    │
  │  ┌────────┐ ┌──────────┐  │
  └──│ Stale  │─│  Fresh   │  │
     │  (0)   │ │  (+1)    │  │
     └────┬───┘ └────┬─────┘  │
          │          │        │
     invalidate()  expire(ttl)│
          │          │        │
          ▼          ▼        │
     ┌──────────┐            │
     │ Invalid  │            │
     │  (−1)    │────────────┘
     └──────────┘   evict_if_needed()
```

### Eviction Priority Function

Each entry receives a real-valued eviction score:

$$\text{score}(e) = s(e) + 0.001 \cdot t_{\text{last\_access}}(e)$$

where $s(e)$ is the state penalty:

| State | $s(e)$ |
|-------|--------|
| Invalid | −2.0 |
| Stale | −1.0 |
| Fresh | 0.0 |

The entry with the **minimum** score is evicted. This means invalid entries are always evicted before stale, which are always evicted before fresh. The `0.001 · last_access` term acts as a tiebreaker implementing LRU within each state tier.

### Complexity

| Operation | Time | Space |
|-----------|------|-------|
| `insert` | O(n) worst-case eviction scan, O(1) amortized | O(1) per entry |
| `get` | O(1) expected (HashMap lookup) | O(1) |
| `invalidate` / `stale` / `refresh` | O(1) expected | O(1) |
| `expire(ttl)` | O(n) — full scan | O(1) |
| `state_distribution` | O(n) | O(1) |

### TTL Expiration

The `expire(ttl)` method performs a linear sweep, transitioning Fresh entries whose `last_access` age exceeds `ttl` ticks to Stale:

$$\text{Fresh} \xrightarrow{t_{\text{now}} - t_{\text{last\_access}} > \text{ttl}} \text{Stale}$$

## Quick Start

```rust
use ternary_cache::{TernaryCache, CacheState};

let mut cache = TernaryCache::<i32>::new(100);

// Insert as Fresh
cache.insert("alpha", 42);
let (val, state) = cache.get("alpha").unwrap();
assert_eq!(val, 42);
assert_eq!(state, CacheState::Fresh);

// Mark stale (e.g., upstream source changed)
cache.stale("alpha");
let (_, state) = cache.get("alpha").unwrap();
assert_eq!(state, CacheState::Stale);

// Refresh with new value
cache.refresh("alpha", 43);
let (val, state) = cache.get("alpha").unwrap();
assert_eq!(val, 43);
assert_eq!(state, CacheState::Fresh);

// TTL-based expiration
cache.expire(1000); // marks entries older than 1000 ticks as stale
```

## API

### `CacheState`

```rust
pub enum CacheState { Invalid, Stale, Fresh }
```

Methods: `to_i8()`, `PartialEq`, `Debug`, `Clone`, `Copy`.

### `TernaryCache<V: Clone>`

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(capacity: usize) -> Self` | Create with max entry count |
| `insert` | `(&mut self, key: &str, value: V)` | Insert as Fresh; evicts if at capacity |
| `get` | `(&mut self, key: &str) -> Option<(V, CacheState)>` | Retrieve value + state; updates access metadata |
| `invalidate` | `(&mut self, key: &str) -> bool` | Transition to Invalid |
| `stale` | `(&mut self, key: &str) -> bool` | Transition to Stale |
| `refresh` | `(&mut self, key: &str, value: V) -> bool` | Update value + transition to Fresh |
| `expire` | `(&mut self, ttl: usize) -> usize` | Bulk-expire stale Fresh entries; returns count |
| `state_distribution` | `(&self) -> (usize, usize, usize)` | (Fresh, Stale, Invalid) counts |
| `hit_rate` | `(&self, hits: usize, misses: usize) -> f64` | Compute hit ratio |
| `len` / `is_empty` | `(&self) -> usize / bool` | Current occupancy |

## Architecture Notes

The cache is designed as a building block for the **SuperInstance fleet coordinator**. In that context:

- **γ (state)** is the `CacheState` of each entry, reflecting data coherence.
- **η (decision)** is the eviction algorithm that uses state-weighted scoring.
- **C (constraint)** is the capacity bound and TTL contract.

The cache deliberately avoids lock-free concurrency — it is designed for single-threaded agent loops where the agent processes one message at a time. For shared caches, wrap in a `Mutex<TernaryCache<V>>`.

The eviction scan is O(n) because the priority function blends state with recency in a way that resists the classic O(1) LRU doubly-linked-list trick. For caches under 10,000 entries (the typical fleet-agent scope), this is negligible.

## References

- **Kleene, S. C.** (1952). *Introduction to Metamathematics*. — Three-valued logic semantics.
- **Tanenbaum, A. S., & Van Steen, M.** (2017). *Distributed Systems* (3rd ed.), Ch. 7 — Cache coherence and consistency models.
- **Tian, W. et al.** (2018). "Adaptive TTL-based cache consistency for edge computing." — TTL-based stale-while-revalidate.
- **Megiddo, N., & Modha, D. S.** (2003). "ARC: A Self-Tuning, Low Overhead Replacement Cache." — Adaptive cache eviction with multiple priority tiers.

## License

MIT
