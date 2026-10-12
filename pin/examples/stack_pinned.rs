// The same self-referential struct as `pinned.rs`, pinned on the stack with
// `pin!` instead of on the heap with `Box::pin`.
//
// Because the value lives in the current stack frame, it cannot be returned
// from a function. So construction is split in two steps:
//   1. `SelfRef::new()` returns an *unpinned*, not-yet-initialized value
//      (moving it is fine, since `ptr` is still null).
//   2. The caller pins it with `pin!`, then calls `init()` to set up the
//      self-reference. From here on the value never moves.

use std::marker::PhantomPinned;
use std::pin::{Pin, pin};

struct SelfRef {
    x: u32,
    // must always point to `x` (null until init() is called)
    ptr: *const u32,
    // makes SelfRef !Unpin
    _pin: PhantomPinned,
}

impl SelfRef {
    // Step 1: no self-reference yet, so returning (moving) it is safe.
    fn new(x: u32) -> Self {
        SelfRef {
            x,
            ptr: std::ptr::null(),
            _pin: PhantomPinned,
        }
    }

    // Step 2: only callable on a pinned value, so `x` will not move after this.
    fn init(self: Pin<&mut Self>) {
        let ptr = &self.x as *const u32;
        // Writing through Pin<&mut T> for a !Unpin T requires unsafe:
        // we promise not to move the value out.
        unsafe {
            self.get_unchecked_mut().ptr = ptr;
        }
    }

    fn print(self: Pin<&Self>, label: &str) {
        println!("[{label}]");
        println!("  &x   {:?}", &self.x as *const u32);
        println!("  ptr  {:?}", self.ptr);
        println!("  *ptr {}", unsafe { *self.ptr });
    }
}

// Passing a Pin around only copies the reference; the value stays put.
// Read-only, so a shared Pin<&SelfRef> is enough.
fn use_it(v: Pin<&SelfRef>) {
    v.print("in use_it()");
    assert_eq!(&v.x as *const _, v.ptr);
}

fn main() {
    // pin! moves the value into a hidden local and returns Pin<&mut SelfRef>.
    // No heap allocation.
    let mut v = pin!(SelfRef::new(0));
    v.as_mut().init();
    v.as_ref().print("after init");
    assert_eq!(&v.x as *const _, v.ptr);

    use_it(v.as_ref());

    // What you cannot do with pin!: return the pinned value from a function.
    // The following does not compile, because it returns a reference to a local:
    //
    //     fn make() -> Pin<&'static mut SelfRef> {
    //         let v = pin!(SelfRef::new(0));
    //         v
    //         // error[E0515]: cannot return value referencing temporary value
    //     }
    //
    // Use Box::pin (see pinned.rs) when the pinned value must outlive the function.

    println!("ok: ptr still points to x (no Box)");
}
