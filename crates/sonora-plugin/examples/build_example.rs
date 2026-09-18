//! Rebuild `plugins/example/plugin.wasm` from `plugins/example/plugin.wat`.
//!
//! Run from the workspace root:
//!
//! ```sh
//! cargo run -p sonora-plugin --example build_example
//! ```

fn main() {
    let wat_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../plugins/example/plugin.wat"
    );
    let wasm_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../plugins/example/plugin.wasm"
    );
    let bytes = wat::parse_file(wat_path).expect("parse plugin.wat");
    assert!(!bytes.is_empty(), "compiled module is empty");
    assert!(
        bytes.len() < 1024 * 1024,
        "example module unexpectedly large"
    );
    std::fs::write(wasm_path, &bytes).expect("write plugin.wasm");
    println!("wrote {} ({} bytes)", wasm_path, bytes.len());
}
