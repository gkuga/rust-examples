// Box<T> basics: a pointer that owns a value on the heap.
//
//   stack            heap
//   ┌─────┐          ┌───┐
//   │  b ─┼────────→ │ 5 │
//   └─────┘          └───┘

fn main() {
    // Box::new moves the value to the heap; `b` holds only a pointer to it.
    let b = Box::new(5);

    // Use it like the value itself: `*b` dereferences, and most operations
    // dereference automatically.
    println!("*b = {}, b + 1 = {}", *b, *b + 1);

    // Two different addresses: where the Box (pointer) is, and where the value is.
    println!("Box at {:p} (stack), value at {:p} (heap)", &b, &*b);

    // Moving a Box copies only the pointer. The value on the heap does not move.
    let moved = b;
    println!("Box at {:p} (stack), value at {:p} (heap)", &moved, &*moved);

    // `*moved` copies the value out to the stack (i32 is Copy, so `moved` stays usable).
    // For a non-Copy type, this would move the value out and free the heap memory.
    let value: i32 = *moved;
    println!("value at {:p} (stack)", &value);

    // A Box frees its heap memory when it goes out of scope (no manual free needed).
}
