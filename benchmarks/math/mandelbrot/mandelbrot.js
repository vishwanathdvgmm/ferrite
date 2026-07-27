// Node.js Benchmark: Mandelbrot Iteration Counter
function mandelbrot(c_re, c_im) {
  let z_re = c_re;
  let z_im = c_im;
  let iter = 0;
  while (z_re * z_re + z_im * z_im <= 4.0 && iter < 1000) {
    const new_re = z_re * z_re - z_im * z_im + c_re;
    const new_im = 2.0 * z_re * z_im + c_im;
    z_re = new_re;
    z_im = new_im;
    iter++;
  }
  return iter;
}

let total_iter = 0;
for (let y = -1.0; y <= 1.0; y += 0.05) {
  for (let x = -2.0; x <= 1.0; x += 0.05) {
    total_iter += mandelbrot(x, y);
  }
}

console.log(`Mandelbrot total iterations = ${total_iter}`);
