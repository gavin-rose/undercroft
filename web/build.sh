#!/usr/bin/env sh
# Builds the wasm module into web/pkg.
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version 0.2.117
set -eu
cd "$(dirname "$0")/.."
cargo build --profile wasm-release --target wasm32-unknown-unknown -p undercroft-wasm
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/wasm-release/undercroft_wasm.wasm
cp crates/undercroft-render/assets/kenney-1bit/tiles.png web/pkg/tiles.png
echo "Built web/pkg. Serve it with: python3 -m http.server -d web 8000"
