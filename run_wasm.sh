echo "Building WebGPU..."
cargo build --target wasm32-unknown-unknown --no-default-features --features webgpu "$@"
wasm-bindgen target/wasm32-unknown-unknown/debug/voxer.wasm --target web --no-typescript --out-dir target/generated --out-name webgpu

echo "Building WebGL..."
cargo build --target wasm32-unknown-unknown --no-default-features --features webgl "$@"
wasm-bindgen target/wasm32-unknown-unknown/debug/voxer.wasm --target web --no-typescript --out-dir target/generated --out-name webgl2

echo "Copying static files..."
cp web/* target/generated

echo "Running web server..."
simple-http-server target/generated -c wasm,html,js -i --coep --coop --ip 127.0.0.1
