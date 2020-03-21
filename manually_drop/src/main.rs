use std::mem::ManuallyDrop;

fn main() {
    let mut orig_val = ManuallyDrop::new("something".to_string());
    unsafe {
        let mut taked_val = ManuallyDrop::take(&mut orig_val);
    }
    println!("{:?}", orig_val);
}
