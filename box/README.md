# Box Examples

`Box<T>` is a pointer that owns a value on the heap.

| Run | What it shows |
|---|---|
| `cargo run` | Basics: create, dereference, move the Box vs move the value out |
| `cargo run --example closures` | `Box<dyn Fn>`: store closures of different types in one `Vec` |

## Key points

- `Box::new(v)` moves `v` to the heap; the Box itself is just a pointer.
- Moving a Box copies only the pointer; the value stays at the same heap address.
- `*b` moves the value out of the Box (for non-`Copy` types this consumes the Box).
- The heap memory is freed automatically when the Box goes out of scope.
- Every closure has its own type. `Box<dyn Fn(..) -> ..>` erases it, so closures can be
  stored in fields or collections. See also `../pin` for `Pin<Box<T>>`.
