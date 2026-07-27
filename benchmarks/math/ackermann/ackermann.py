# Python Benchmark: Ackermann Function
import sys
sys.setrecursionlimit(20000)

def ackermann(m, n):
    if m == 0:
        return n + 1
    elif m > 0 and n == 0:
        return ackermann(m - 1, 1)
    else:
        return ackermann(m - 1, ackermann(m, n - 1))

res = ackermann(3, 8)
print("Ackermann(3, 8) =", res)
