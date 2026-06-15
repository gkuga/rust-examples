# Rust Deadlock Examples

Demonstrates how deadlocks occur in Rust and how to avoid them using `Arc`, `RwLock`, and proper guard lifetime management.

## Key Concepts

### RAII Pattern (Resource Acquisition Is Initialization)

The RAII pattern ties the resource lifecycle to an object's lifetime:

- **Acquisition**: When a resource (lock) is acquired, an object (guard) is created
- **Automatic Release**: When that object goes out of scope, the resource is automatically released via `drop()`

This prevents resource leaks and ensures cleanup even if code panics.

```rust
let guard = val.read().unwrap();  // Acquire lock → guard created
// ... guard is alive → lock is held ...
// } ← guard scope ends → drop() called → lock released automatically
```

### Guard: The Sentinel Object

A **Guard** is a smart pointer that wraps a lock and manages its lifetime. It acts as a "sentinel" ensuring the lock is held only as long as the guard exists.

- `RwLockReadGuard<T>`: Holds a Read lock; released when dropped
- `RwLockWriteGuard<T>`: Holds a Write lock; released when dropped

The guard's **scope determines when the lock is released**:

```rust
// Guard scope is limited → lock released quickly
let flag = *val.read().unwrap();

// Guard scope extends → lock held until guard is dropped
let guard = val.read().unwrap();
```

## Examples

### deadlock_example()

Shows how a deadlock occurs:

1. Acquire a Read lock (guard holds it)
2. Try to acquire a Write lock while Read lock is still held
3. Write lock waits for the Read lock to be released
4. Read lock guard never goes out of scope → **deadlock**

### deadlock_example2()

Fixed version that avoids deadlock:

1. Dereference `*guard` immediately to copy the value
2. Guard is dropped right after (Read lock released)
3. Write lock can now be acquired safely

### deref_example()

Minimal example showing the difference between:
- `let guard = val.read().unwrap()` (guard variable holds the lock)
- `let flag = *val.read().unwrap()` (guard is dropped immediately)
