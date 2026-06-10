# ternary-cache

Caching with ternary entry states — Invalid (-1), Stale (0), Fresh (+1).

## Why This Exists

Standard caches are binary: hit or miss. But cached data has three meaningful states: **Fresh** (confirmed current), **Stale** (was valid but TTL expired — still usable with a warning), and **Invalid** (explicitly invalidated or never populated). Treating stale data as "miss" forces unnecessary recomputation. Treating it as "hit" serves outdated results. Ternary cache gives you the third option: serve stale data while asynchronously refreshing.

## Architecture

### Core Types

- **`CacheState`** — Enum: `Invalid (-1)`, `Stale (0)`, `Fresh (+1)`.
- **`CacheEntry<V>`** — A generic entry with value, state, access count, and insertion tick.
- **`TernaryCache<V: Clone>`** — LRU cache with capacity tracking and state transitions.

### State Transitions

- `insert` → Fresh
- `get` → Returns `(V, CacheState)`. Fresh entries return normally.
- `stale` → Transition to Stale without removing data.
- `invalidate` → Mark as Invalid (data removed).
- `refresh` → Update value, reset to Fresh.
- `expire(ttl)` → All entries older than `ttl` ticks transition Fresh → Stale.

## Usage

```rust
use ternary_cache::{TernaryCache, CacheState};

let mut cache: TernaryCache<Vec<i8>> = TernaryCache::new(100);

// Insert fresh data
cache.insert("layer_0_weights", vec![1, 0, -1, 1]);

// Get with state awareness
if let Some((weights, state)) = cache.get("layer_0_weights") {
    match state {
        CacheState::Fresh => println!("Using cached weights"),
        CacheState::Stale => println!("Using stale weights, refresh recommended"),
        CacheState::Invalid => println!("Cache miss"),
    }
}

// Time passes — mark as stale
cache.stale("layer_0_weights");

// Periodic expiry
let expired = cache.expire(1000); // entries older than 1000 ticks

// Distribution: (invalid, stale, fresh)
let (inv, stale, fresh) = cache.state_distribution();
```

## API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `new(capacity)` | `TernaryCache<V>` | Create cache with max entries |
| `insert(key, value)` | `()` | Insert as Fresh |
| `get(key)` | `Option<(V, CacheState)>` | Get value with state |
| `invalidate(key)` | `bool` | Mark Invalid (removes data) |
| `stale(key)` | `bool` | Downgrade to Stale |
| `refresh(key, value)` | `bool` | Update value, reset to Fresh |
| `len()` / `is_empty()` | `usize` / `bool` | Entry count |
| `state_distribution()` | `(usize, usize, usize)` | (Invalid, Stale, Fresh) counts |
| `expire(ttl)` | `usize` | Expire entries older than `ttl` ticks |
| `hit_rate(hits, misses)` | `f64` | Calculate hit rate |

## The Deeper Idea

The stale state is **eventual consistency for caches**. In a distributed system, you often have cached data that's "probably still valid" but you haven't confirmed. Rather than blocking on a freshness check (latency) or blindly serving (correctness risk), serve stale with a flag that triggers async refresh. This is the pattern used by DNS (TTL with stale-while-revalidate), HTTP (stale-while-revalidate), and CDN edge caches. Ternary cache makes this a first-class API.

## Related Crates

- **ternary-gc** — garbage collection with ternary marking
- **ternary-intent-cache** — intent-to-bytecode compilation cache
- **ternary-mirror** — state mirroring with ternary consistency
