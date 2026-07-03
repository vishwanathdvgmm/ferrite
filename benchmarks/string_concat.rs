// Rust Benchmark: String Concatenation
fn main() {
    let mut result = String::new();
    for _ in 0..10000 {
        result.push_str("x");
    }
    println!("String length = {}", result.len());
}
