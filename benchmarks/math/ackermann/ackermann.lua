-- Lua Benchmark: Ackermann Function
function ackermann(m, n)
    if m == 0 then
        return n + 1
    elseif m > 0 and n == 0 then
        return ackermann(m - 1, 1)
    else
        return ackermann(m - 1, ackermann(m, n - 1))
    end
end

local res = ackermann(3, 8)
print("Ackermann(3, 8) = " .. res)
