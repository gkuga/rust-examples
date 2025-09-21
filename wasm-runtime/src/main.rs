mod parser;
mod runtime;
mod test_wasm;

use std::fs;
use anyhow::Result;

fn main() -> Result<()> {
    println!("Wasm Runtime Starting...");
    
    // Generate test.wasm from WAT source
    println!("\n=== Generating test.wasm from WAT source ===");
    test_wasm::save_wat_as_test_wasm()?;
    
    // First, execute test with manually created WASM binary
    println!("\n=== Testing with embedded WASM binary ===");
    let mut runtime = runtime::WasmRuntime::new();
    runtime.load_module(test_wasm::TEST_WASM)?;
    runtime.execute_function("add", &[5, 3])?;

    // Test WAT compilation feature
    println!("\n=== Testing with WAT compilation ===");
    let mut runtime2 = runtime::WasmRuntime::new();
    let compiled_wasm = test_wasm::get_compiled_add_wasm()?;
    runtime2.load_module(&compiled_wasm)?;
    runtime2.execute_function("add", &[10, 7])?;
    
    // Also execute if WASM file from filesystem exists
    if let Ok(wasm_bytes) = fs::read("test.wasm") {
        println!("\n=== Testing with test.wasm file ===");
        let mut runtime3 = runtime::WasmRuntime::new();
        runtime3.load_module(&wasm_bytes)?;
        runtime3.execute_function("add", &[1, 2])?;
    } else {
        println!("\ntest.wasm not found. Using embedded test binary only.");
    }
    
    Ok(())
}