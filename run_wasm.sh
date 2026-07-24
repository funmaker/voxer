echo "Building WebGPU..."
cargo build --target wasm32-unknown-unknown --no-default-features --features webgpu "$@" || exit 1
wasm-bindgen target/wasm32-unknown-unknown/debug/voxer.wasm --target web --no-typescript --out-dir target/generated --out-name webgpu || exit 1

echo "Building WebGL..."
cargo build --target wasm32-unknown-unknown --no-default-features --features webgl "$@" || exit 1
wasm-bindgen target/wasm32-unknown-unknown/debug/voxer.wasm --target web --no-typescript --out-dir target/generated --out-name webgl2 || exit 1

echo "Copying static files..."
cp web/* target/generated || exit 1

echo "Running web server..."
simple-http-server target/generated -c wasm,html,js -i --coep --coop --ip 127.0.0.1 || exit 1
