macro_rules! pow {
    ($x:literal ^ $n:literal) => {
        $x.pow($n)
    };
}

fn main() {
    let num = pow!(3_i32 ^ 2);
    println!("{}", num);
}
