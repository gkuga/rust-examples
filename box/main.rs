fn main() {
    let extra_value = 10;
    let calculator: Box<dyn Fn(u64) -> u64>;
    calculator = Box::new(move |y| {
        let x = 5;
        x * y + extra_value
    });
    let result = calculator(3);
    println!("Result: {}", result);
    let mut operations: Vec<Box<dyn Fn(u64) -> u64>> = Vec::new();
    operations.push(Box::new(|n| n + 1));
    operations.push(Box::new(|n| n * n));
    println!("Execute functions in the list:");
    for op in operations {
        println!("Result: {}", op(10));
    }
}
