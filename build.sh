#!/bin/bash
set -e
wasm-pack build --target web
cp src/assets/* pkg
echo "Done. Serve with: cd pkg && python3 -m http.server 8080"