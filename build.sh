cargo build -p wasm_guest_binary --target wasm32-unknown-unknown
wasm-pack build ./host_runner --debug --scope host -t nodejs