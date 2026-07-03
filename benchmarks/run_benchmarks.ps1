# Run Benchmarks

$ErrorActionPreference = "Stop"

Write-Host "========================================="
Write-Host "       Ferrite Benchmark Suite           "
Write-Host "========================================="
Write-Host ""

# Ensure we're in the right directory
if (-not (Test-Path ".\benchmarks")) {
    Write-Host "Please run this script from the project root."
    exit 1
}

# Build Ferrite in release mode
Write-Host "Building Ferrite (Release mode)..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Ferrite build failed."
    exit 1
}
$FerriteCmd = ".\target\release\ferrite.exe"

# Build Rust benchmarks
Write-Host "Building Rust benchmarks..."
rustc -O .\benchmarks\fibonacci.rs -o .\benchmarks\fibonacci_rs.exe
rustc -O .\benchmarks\loop_sum.rs -o .\benchmarks\loop_sum_rs.exe
rustc -O .\benchmarks\string_concat.rs -o .\benchmarks\string_concat_rs.exe

# Build Go benchmarks
Write-Host "Building Go benchmarks..."
go build -o .\benchmarks\fibonacci_go.exe .\benchmarks\fibonacci.go
go build -o .\benchmarks\loop_sum_go.exe .\benchmarks\loop_sum.go
go build -o .\benchmarks\string_concat_go.exe .\benchmarks\string_concat.go


function Run-Benchmark {
    param(
        [string]$Name,
        [string]$Command,
        [string]$Args
    )
    
    # Check if command exists
    try {
        if ($Command -notlike "*\*") {
            $null = Get-Command $Command -ErrorAction Stop
        }
    } catch {
        return "N/A"
    }

    $sw = [Diagnostics.Stopwatch]::StartNew()
    
    if ($Args) {
        $p = Start-Process -FilePath $Command -ArgumentList $Args -NoNewWindow -Wait -PassThru
    } else {
        $p = Start-Process -FilePath $Command -NoNewWindow -Wait -PassThru
    }
    
    $sw.Stop()
    
    if ($p.ExitCode -eq 0) {
        return "$($sw.Elapsed.TotalMilliseconds.ToString('0.00')) ms"
    } else {
        return "Failed"
    }
}

Write-Host ""
Write-Host "Running benchmarks..."
Write-Host ""

$tests = @("fibonacci", "loop_sum", "string_concat")

# Print Header
Write-Host ("{0,-20} | {1,-10} | {2,-10} | {3,-10} | {4,-10} | {5,-10} | {6,-10}" -f "Benchmark", "Ferrite", "Python", "Lua", "Node.js", "Rust", "Go")
Write-Host ("-" * 95)

foreach ($test in $tests) {
    $f_time = Run-Benchmark -Name $test -Command $FerriteCmd -Args "run .\benchmarks\$test.fe"
    $p_time = Run-Benchmark -Name $test -Command "python" -Args ".\benchmarks\$test.py"
    $l_time = Run-Benchmark -Name $test -Command "lua" -Args ".\benchmarks\$test.lua"
    $n_time = Run-Benchmark -Name $test -Command "node" -Args ".\benchmarks\$test.js"
    $r_time = Run-Benchmark -Name $test -Command ".\benchmarks\${test}_rs.exe" -Args ""
    $g_time = Run-Benchmark -Name $test -Command ".\benchmarks\${test}_go.exe" -Args ""

    Write-Host ("{0,-20} | {1,-10} | {2,-10} | {3,-10} | {4,-10} | {5,-10} | {6,-10}" -f $test, $f_time, $p_time, $l_time, $n_time, $r_time, $g_time)
}

Write-Host ""
Write-Host "========================================="
