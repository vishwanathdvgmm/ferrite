// Rust Benchmark: Mandelbrot Iteration Counter
fn mandelbrot(c_re: f64, c_im: f64) -> i64 {
    let mut z_re = c_re;
    let mut z_im = c_im;
    let mut iter = 0;
    while z_re * z_re + z_im * z_im <= 4.0 && iter < 1000 {
        let new_re = z_re * z_re - z_im * z_im + c_re;
        let new_im = 2.0 * z_re * z_im + c_im;
        z_re = new_re;
        z_im = new_im;
        iter += 1;
    }
    iter
}

fn main() {
    let mut total_iter = 0;

    // Rust floating point iteration is best done with steps or mapped integers
    let mut y = -1.0;
    while y <= 1.0 {
        let mut x = -2.0;
        while x <= 1.0 {
            total_iter += mandelbrot(x, y);
            x += 0.05;
        }
        y += 0.05;
    }

    println!("Mandelbrot total iterations = {}", total_iter);
}
