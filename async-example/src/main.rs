async fn foo() {
    println!("foo");
}

fn main() {
    let _ = foo();
    let _ = async {
        println!("bar");
    };
    println!("baz");
}
