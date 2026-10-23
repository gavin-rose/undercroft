// Usage: node web/crosscheck.mjs <nodejs wasm-bindgen pkg>/undercroft_wasm.js <path to croft>
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";

const [pkg, croft] = process.argv.slice(2);
const wasm = createRequire(import.meta.url)(pkg);
const native = (args) =>
  execFileSync(croft, [...args, "--format", "json"], { stdio: ["ignore", "pipe", "ignore"] }).toString();

let checked = 0;
let mismatches = 0;
for (const { name, source } of JSON.parse(wasm.styles())) {
  for (let i = 0; i < 50; i++) {
    const seed = `cross-${i}`;
    const generated = wasm.generate(source, seed);
    const same = generated.json() === native(["gen", name, "--seed", seed]);
    generated.free();
    checked++;
    if (!same) {
      mismatches++;
      console.error(`mismatch: ${name} --seed ${seed}`);
    }
  }
}
console.log(`${checked} worlds compared, ${mismatches} mismatches`);
process.exit(mismatches ? 1 : 0);
