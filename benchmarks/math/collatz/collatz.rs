// Rust Benchmark: Collatz Sequence Max Length
fn collatz(mut n: i64) -> i64 {
    let mut len = 0;
    while n > 1 {
        if n % 2 == 0 {
            n = n / 2;
        } else {
            n = 3 * n + 1;
        }
        len += 1;
    }
    len
}

fn main() {
    let mut max_len = 0;
    for i in 1..=100000 {
        let clen = collatz(i);
        if clen > max_len {
            max_len = clen;
        }
    }
    println!("Max Collatz length up to 100000 = {}", max_len);
}
