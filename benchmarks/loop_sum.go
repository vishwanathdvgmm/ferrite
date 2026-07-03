// Go Benchmark: Loop Sum
package main

import "fmt"

func main() {
	total := 0
	for i := 1; i <= 1000000; i++ {
		total += i
	}
	fmt.Printf("Sum = %d\n", total)
}
