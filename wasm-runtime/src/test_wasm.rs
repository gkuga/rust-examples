// Simple WASM binary byte array created manually
// This corresponds to the following WAT code:
// (module
//   (func $add (param $lhs i32) (param $rhs i32) (result i32)
//     local.get $lhs
//     local.get $rhs
//     i32.add)
//   (export "add" (func $add))
// )

use anyhow::Result;

// WAT source code for the add function
pub const ADD_WAT: &str = r#"
(module
  (func $add (param $lhs i32) (param $rhs i32) (result i32)
    local.get $lhs
    local.get $rhs
    i32.add)
  (export "add" (func $add)))
"#;

// Compile WAT source to WASM binary
pub fn compile_wat_to_wasm(wat_source: &str) -> Result<Vec<u8>> {
    let wasm_binary = wat::parse_str(wat_source)?;
    Ok(wasm_binary)
}

// Get compiled WASM from WAT source
pub fn get_compiled_add_wasm() -> Result<Vec<u8>> {
    compile_wat_to_wasm(ADD_WAT)
}

// Save compiled WAT to test.wasm file
pub fn save_wat_as_test_wasm() -> Result<()> {
    use std::fs;
    let wasm_binary = compile_wat_to_wasm(ADD_WAT)?;
    fs::write("test.wasm", wasm_binary)?;
    println!("Saved compiled WAT to test.wasm");
    Ok(())
}

pub const TEST_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, // magic
    0x01, 0x00, 0x00, 0x00, // version
    
    // Type section
    0x01, // section id
    0x07, // section length
    0x01, // num types
    0x60, // func type
    0x02, 0x7f, 0x7f, // param count, i32, i32
    0x01, 0x7f, // result count, i32
    
    // Function section
    0x03, // section id
    0x02, // section length
    0x01, // num functions
    0x00, // type index
    
    // Export section
    0x07, // section id
    0x07, // section length
    0x01, // num exports
    0x03, 0x61, 0x64, 0x64, // length=3, "add"
    0x00, 0x00, // export kind func, function index
    
    // Code section
    0x0a, // section id
    0x09, // section length
    0x01, // num functions
    0x07, // function body size
    0x00, // local decl count
    0x20, 0x00, // local.get 0
    0x20, 0x01, // local.get 1
    0x6a,       // i32.add
    0x0b,       // end
];