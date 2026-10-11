// Fixing the self-referential struct from `src/main.rs` with Pin.
//
// `src/main.rs` shows the problem: `ptr` points to `x`, but returning the struct
// moves it, so `ptr` ends up pointing to the old (invalid) address.
//
// Fix:
//   1. Mark the type as !Unpin with `PhantomPinned`, so safe code can no longer
//      take `&mut SelfRef` out of a `Pin` (and therefore cannot move it).
//   2. Put it on the heap with `Box::pin` *before* setting up the self-reference.
//      Moving the `Pin<Box<_>>` only moves the pointer; the data stays put.

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

        // The data is now pinned on the heap, so its address will never change.
        let ptr = &this.x as *const u32;
        // Writing through Pin<&mut T> for a !Unpin T requires unsafe:
        // we promise not to move the value out.
        unsafe {
            this.as_mut().get_unchecked_mut().ptr = ptr;
        }

        this.print("in new()");
        this
    }

    fn print(&self, label: &str) {
        println!("[{label}]");
        println!("  &x   {:?}", &self.x as *const u32);
        println!("  ptr  {:?}", self.ptr);
        println!("  *ptr {}", unsafe { *self.ptr });
    }
}

fn main() {
    // Returned from new(): the Box moved, but the SelfRef on the heap did not.
    let v = SelfRef::new(0);
    v.print("after return");
    assert_eq!(&v.x as *const _, v.ptr);

    // Moving the Pin<Box<_>> to another variable is fine for the same reason.
    let moved = v;
    moved.print("after move");
    assert_eq!(&moved.x as *const _, moved.ptr);

    // What Pin prevents: getting `&mut SelfRef` in safe code.
    // Uncommenting the following fails to compile, because SelfRef is !Unpin:
    //
    //     let mut other = SelfRef::new(1);
    //     std::mem::swap(moved.as_mut().get_mut(), other.as_mut().get_mut());
    //     // error[E0277]: `PhantomPinned` cannot be unpinned

    println!("ok: ptr still points to x");
}
