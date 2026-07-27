// Rust Benchmark: Ackermann Function
fn ackermann(m: i64, n: i64) -> i64 {
    if m == 0 {
        n + 1
    } else if m > 0 && n == 0 {
        ackermann(m - 1, 1)
    } else {
        ackermann(m - 1, ackermann(m, n - 1))
    }
}

fn main() {
    let res = ackermann(3, 8);
    println!("Ackermann(3, 8) = {}", res);
}
