-- Lua Benchmark: Mandelbrot Iteration Counter
function mandelbrot(c_re, c_im)
    local z_re = c_re
    local z_im = c_im
    local iter = 0
    while z_re * z_re + z_im * z_im <= 4.0 and iter < 1000 do
        local new_re = z_re * z_re - z_im * z_im + c_re
        local new_im = 2.0 * z_re * z_im + c_im
        z_re = new_re
        z_im = new_im
        iter = iter + 1
    end
    return iter
end

local total_iter = 0
local y = -1.0
while y <= 1.0 do
    local x = -2.0
    while x <= 1.0 do
        total_iter = total_iter + mandelbrot(x, y)
        x = x + 0.05
    end
    y = y + 0.05
end

print("Mandelbrot total iterations = " .. total_iter)
