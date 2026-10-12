// Box alone is not enough to protect a self-referential struct.
//
// Box keeps the value at a fixed heap address *as long as nobody moves the
// value itself*. But `&mut SelfRef` is freely available from a Box, so safe
// code can still move the value (swap it, or move it out of the Box), and the
// self-pointer then points to the wrong place.
//
// Compare with box_pinned.rs, where Pin + PhantomPinned make these operations
// fail to compile.

struct SelfRef {
    x: u32,
    // must always point to `x`
    ptr: *const u32,
}

impl SelfRef {
    fn new(x: u32) -> Box<SelfRef> {
        let mut this = Box::new(SelfRef {
            x,
            ptr: std::ptr::null(),
        });
        // Set the self-pointer after the value is on the heap.
        this.ptr = &this.x;
        this
    }

    fn is_valid(&self) -> bool {
        std::ptr::eq(&self.x, self.ptr)
    }
}

fn main() {
    // --- Case 1: moving the Box itself is fine ---
    let a = SelfRef::new(1);
    let moved = a; // only the pointer on the stack moves
    println!("[case 1] move the Box");
    println!("  &x = {:p}, ptr = {:p}, valid = {}", &moved.x, moved.ptr, moved.is_valid());

    // --- Case 2: swapping the contents of two Boxes breaks it ---
    // Both values stay on the heap, but their bytes are exchanged,
    // so each `ptr` now points into the *other* Box.
    let mut a = SelfRef::new(1);
    let mut b = SelfRef::new(2);
    std::mem::swap(&mut *a, &mut *b); // compiles: Box gives &mut SelfRef
    println!("[case 2] mem::swap the contents");
    println!("  a: &x = {:p}, ptr = {:p}, valid = {}", &a.x, a.ptr, a.is_valid());
    println!("  a.x = {}, but *a.ptr = {} (reads b's x)", a.x, unsafe { *a.ptr });
    println!("  b: &x = {:p}, ptr = {:p}, valid = {}", &b.x, b.ptr, b.is_valid());

    // --- Case 3: moving the value out of the Box breaks it ---
    // `*c` copies the value to the stack and frees the heap allocation,
    // so `ptr` dangles. (We only compare addresses; dereferencing it would be UB.)
    let c = SelfRef::new(3);
    let heap_addr = &c.x as *const u32;
    let out: SelfRef = *c; // compiles: moves the value out of the Box
    println!("[case 3] move the value out of the Box");
    println!("  old heap &x = {:p}", heap_addr);
    println!("  &x = {:p}, ptr = {:p}, valid = {}", &out.x, out.ptr, out.is_valid());
}
