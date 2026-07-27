// Node.js Benchmark: Ackermann Function
function ackermann(m, n) {
  if (m === 0) {
    return n + 1;
  } else if (m > 0 && n === 0) {
    return ackermann(m - 1, 1);
  } else {
    return ackermann(m - 1, ackermann(m, n - 1));
  }
}

const res = ackermann(3, 8);
console.log(`Ackermann(3, 8) = ${res}`);
