// Rust Benchmark: Recursive Fibonacci
fn fib(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    let result = fib(30);
    println!("fib(30) = {}", result);
}
