-- Lua Benchmark: Loop Sum
local total = 0
for i = 1, 1000000 do
    total = total + i
end
print("Sum = " .. tostring(total))
