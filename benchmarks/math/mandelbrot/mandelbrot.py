# Python Benchmark: Mandelbrot Iteration Counter
def mandelbrot(c_re, c_im):
    z_re = c_re
    z_im = c_im
    iter_count = 0
    while z_re * z_re + z_im * z_im <= 4.0 and iter_count < 1000:
        new_re = z_re * z_re - z_im * z_im + c_re
        new_im = 2.0 * z_re * z_im + c_im
        z_re = new_re
        z_im = new_im
        iter_count += 1
    return iter_count

total_iter = 0
y = -1.0
while y <= 1.0:
    x = -2.0
    while x <= 1.0:
        total_iter += mandelbrot(x, y)
        x += 0.05
    y += 0.05

print("Mandelbrot total iterations =", total_iter)
