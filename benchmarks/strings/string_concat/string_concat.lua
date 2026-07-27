-- Lua Benchmark: String Concatenation
local result = ""
for i = 1, 10000 do
    result = result .. "x"
end
print("String length = " .. tostring(string.len(result)))
