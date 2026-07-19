@echo off

echo "Building WebGPU..."
cargo.exe build --target wasm32-unknown-unknown --no-default-features --features webgpu %* || exit /b
wasm-bindgen.exe target/wasm32-unknown-unknown/debug/voxer.wasm --target web --no-typescript --out-dir target/generated --out-name webgpu || exit /b

echo "Building WebGL..."
cargo.exe build --target wasm32-unknown-unknown --no-default-features --features webgl %* || exit /b
wasm-bindgen.exe target/wasm32-unknown-unknown/debug/voxer.wasm --target web --no-typescript --out-dir target/generated --out-name webgl2 || exit /b

echo "Copying static files..."
copy "web\*.*" "target\generated" || exit /b

echo "Running web server..."
simple-http-server.exe target/generated -c wasm,html,js -i --coep --coop --ip 127.0.0.1
