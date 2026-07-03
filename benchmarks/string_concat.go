// Go Benchmark: String Concatenation
package main

import "fmt"

func main() {
	result := ""
	for i := 0; i < 10000; i++ {
		result += "x"
	}
	fmt.Printf("String length = %d\n", len(result))
}
