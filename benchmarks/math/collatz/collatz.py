# Python Benchmark: Collatz Sequence Max Length
def collatz(n):
    length = 0
    while n > 1:
        if n % 2 == 0:
            n = n // 2
        else:
            n = 3 * n + 1
        length += 1
    return length

max_len = 0
for i in range(1, 100001):
    clen = collatz(i)
    if clen > max_len:
        max_len = clen

print("Max Collatz length up to 100000 =", max_len)
