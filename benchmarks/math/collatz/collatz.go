// Go Benchmark: Collatz Sequence Max Length
package main

import "fmt"

func collatz(n int64) int64 {
	var length int64 = 0
	for n > 1 {
		if n%2 == 0 {
			n = n / 2
		} else {
			n = 3*n + 1
		}
		length++
	}
	return length
}

func main() {
	var max_len int64 = 0
	for i := int64(1); i <= 100000; i++ {
		clen := collatz(i)
		if clen > max_len {
			max_len = clen
		}
	}
	fmt.Printf("Max Collatz length up to 100000 = %d\n", max_len)
}
