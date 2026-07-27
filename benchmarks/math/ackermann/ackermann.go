// Go Benchmark: Ackermann Function
package main

import "fmt"

func ackermann(m int, n int) int {
    if m == 0 {
        return n + 1
    } else if m > 0 && n == 0 {
        return ackermann(m - 1, 1)
    } else {
        return ackermann(m - 1, ackermann(m, n - 1))
    }
}

func main() {
    res := ackermann(3, 8)
    fmt.Printf("Ackermann(3, 8) = %d\n", res)
}
