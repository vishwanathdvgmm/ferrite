// Rust Benchmark: Loop Sum
fn main() {
    let mut total: i64 = 0;
    for i in 1..=1000000 {
        total += i;
    }
    println!("Sum = {}", total);
}
