struct Foo {
    val: u32,
}

fn mutable_borrow_and_reborrow() {
    let mut x = Foo { val: 10 }; // x is a mutable variable

    {
        let a = &mut x; // a is a mutable reference to x
        println!("a.val = {}", a.val);

        // The next line would fail to compile because x is mutably borrowed by a.
        // println!("x.val = {}", x.val);

        let b: &Foo = a; // b is an immutable reborrow from mutable reference a

        // The next line would fail to compile because a is currently reborrowed as b.
        // a.val = 20;

        println!("b.val = {}", b.val);
        // b is last used above, so the reborrow ends here (NLL: non-lexical lifetimes).

        a.val = 30; // This is valid again after b is no longer used.
        println!("a.val (after write) = {}", a.val);
    }

    println!("x.val (after mutable borrow scope) = {}", x.val);
}

fn immutable_borrow_blocks_mutable_borrow() {
    let mut x = Foo { val: 100 };

    {
        let c = &x; // c is an immutable reference
        println!("c.val = {}", c.val);

        // The next lines would fail to compile because x is immutably borrowed by c.
        // let d = &mut x;
        // d.val = 40;

        println!("c.val (still valid) = {}", c.val);
    }

    // After c goes out of use, mutable access is possible again.
    x.val = 140;
    println!("x.val (after immutable borrow scope) = {}", x.val);
}

fn main() {
    println!("=== mutable borrow + reborrow demo ===");
    mutable_borrow_and_reborrow();

    println!();
    println!("=== immutable borrow blocks mutable borrow demo ===");
    immutable_borrow_blocks_mutable_borrow();
}
