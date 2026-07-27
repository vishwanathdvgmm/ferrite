// Node.js Benchmark: Collatz Sequence Max Length
function collatz(n) {
  let len = 0;
  while (n > 1) {
    if (n % 2 === 0) {
      n = Math.floor(n / 2);
    } else {
      n = 3 * n + 1;
    }
    len++;
  }
  return len;
}

let maxLen = 0;
for (let i = 1; i <= 100000; i++) {
  const clen = collatz(i);
  if (clen > maxLen) {
    maxLen = clen;
  }
}

console.log(`Max Collatz length up to 100000 = ${maxLen}`);
