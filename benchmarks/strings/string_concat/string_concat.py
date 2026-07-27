# Python Benchmark: String Concatenation
result = ""
for i in range(10000):
    result += "x"
print(f"String length = {len(result)}")
