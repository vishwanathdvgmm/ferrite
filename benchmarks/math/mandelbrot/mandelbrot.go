// Go Benchmark: Mandelbrot Iteration Counter
package main

import "fmt"

func mandelbrot(c_re float64, c_im float64) int64 {
	z_re := c_re
	z_im := c_im
	var iter int64 = 0
	for z_re*z_re+z_im*z_im <= 4.0 && iter < 1000 {
		new_re := z_re*z_re - z_im*z_im + c_re
		new_im := 2.0*z_re*z_im + c_im
		z_re = new_re
		z_im = new_im
		iter++
	}
	return iter
}

func main() {
	var total_iter int64 = 0
	for y := -1.0; y <= 1.0; y += 0.05 {
		for x := -2.0; x <= 1.0; x += 0.05 {
			total_iter += mandelbrot(x, y)
		}
	}
	fmt.Printf("Mandelbrot total iterations = %d\n", total_iter)
}
