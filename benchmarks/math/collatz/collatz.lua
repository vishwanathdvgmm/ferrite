-- Lua Benchmark: Collatz Sequence Max Length
function collatz(n)
    local len = 0
    while n > 1 do
        if n % 2 == 0 then
            n = math.floor(n / 2)
        else
            n = 3 * n + 1
        end
        len = len + 1
    end
    return len
end

local max_len = 0
for i = 1, 100000 do
    local clen = collatz(i)
    if clen > max_len then
        max_len = clen
    end
end

print("Max Collatz length up to 100000 = " .. max_len)
