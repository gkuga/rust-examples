// Fixing box_not_enough.rs with Pin.
//
// box_not_enough.rs breaks the self-reference in safe code because a Box hands
// out `&mut SelfRef`. Here the same three cases are repeated with
// `Pin<Box<SelfRef>>` and a `PhantomPinned` field:
//
//   - Pin<Box<T>> implements DerefMut only when T: Unpin.
//   - PhantomPinned makes SelfRef !Unpin.
//   => safe code can only get `&SelfRef`, never `&mut SelfRef`,
//      so it cannot swap the value or move it out of the Box.

use std::marker::PhantomPinned;
use std::pin::Pin;

struct SelfRef {
    x: u32,
    // must always point to `x`
    ptr: *const u32,
    // makes SelfRef !Unpin
    _pin: PhantomPinned,
}

impl SelfRef {
    fn new(x: u32) -> Pin<Box<SelfRef>> {
        let mut this = Box::pin(SelfRef {
            x,
            ptr: std::ptr::null(),
            _pin: PhantomPinned,
        });
        // Set the self-pointer after the value is pinned on the heap.
        let ptr = &this.x as *const u32;
        // Writing through Pin<&mut T> for a !Unpin T requires unsafe:
        // we promise not to move the value.
        unsafe {
            this.as_mut().get_unchecked_mut().ptr = ptr;
        }
        this
    }

    fn is_valid(&self) -> bool {
        std::ptr::eq(&self.x, self.ptr)
    }
}

fn main() {
    // --- Case 1: moving the Pin<Box> itself is still fine ---
    let a = SelfRef::new(1);
    let moved = a; // only the pointer on the stack moves
    println!("[case 1] move the Pin<Box>");
    println!("  &x = {:p}, ptr = {:p}, valid = {}", &moved.x, moved.ptr, moved.is_valid());

    // --- Case 2: swapping the contents no longer compiles ---
    let mut a = SelfRef::new(1);
    let mut b = SelfRef::new(2);
    // std::mem::swap(&mut *a, &mut *b);
    // // error[E0596]: cannot borrow data in dereference of `Pin<Box<SelfRef>>` as mutable
    // std::mem::swap(a.as_mut().get_mut(), b.as_mut().get_mut());
    // // error[E0277]: `PhantomPinned` cannot be unpinned
    //
    // Swapping the Pin<Boxes> themselves is allowed: only the two heap
    // pointers are exchanged, the values stay where they are.
    std::mem::swap(&mut a, &mut b);
    println!("[case 2] mem::swap the Pin<Box>es (contents cannot be swapped)");
    println!("  a: x = {}, &x = {:p}, ptr = {:p}, valid = {}", a.x, &a.x, a.ptr, a.is_valid());
    println!("  b: x = {}, &x = {:p}, ptr = {:p}, valid = {}", b.x, &b.x, b.ptr, b.is_valid());

    // --- Case 3: moving the value out of the Box no longer compiles ---
    let c = SelfRef::new(3);
    // let out: SelfRef = *c;
    // // error[E0507]: cannot move out of dereference of `Pin<Box<SelfRef>>`
    // let out: Box<SelfRef> = Pin::into_inner(c);
    // // error[E0277]: `PhantomPinned` cannot be unpinned
    println!("[case 3] move the value out (does not compile, value stays pinned)");
    println!("  &x = {:p}, ptr = {:p}, valid = {}", &c.x, c.ptr, c.is_valid());
}
