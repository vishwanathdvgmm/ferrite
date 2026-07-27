// Node.js Benchmark: String Concatenation
let result = "";
for (let i = 0; i < 10000; i++) {
    result += "x";
}
console.log(`String length = ${result.length}`);
