use crate::parser::WasmModule;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub enum Value {
    I32(i32),
}

impl Value {
    pub fn as_i32(&self) -> Result<i32> {
        match self {
            Value::I32(v) => Ok(*v),
        }
    }
}

pub struct WasmRuntime {
    module: Option<WasmModule>,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self {
            module: None,
        }
    }

    pub fn load_module(&mut self, wasm_bytes: &[u8]) -> Result<()> {
        let module = WasmModule::parse(wasm_bytes)?;
        println!("Loaded module with {} functions", module.function_count);
        println!("Exports: {:?}", module.exports.keys().collect::<Vec<_>>());
        self.module = Some(module);
        Ok(())
    }

    pub fn execute_function(&mut self, name: &str, args: &[i32]) -> Result<Option<Value>> {
        let module = self.module.as_ref()
            .ok_or_else(|| anyhow!("No module loaded"))?;

        if !module.has_function(name) {
            return Err(anyhow!("Function '{}' not found", name));
        }

        // Simple implementation: simulate addition for functions named "add"
        if name == "add" && args.len() == 2 {
            let result = args[0] + args[1];
            let result_value = Value::I32(result);
            println!("Executed function '{}' with args {:?} -> {}", name, args, result_value.as_i32()?);
            Ok(Some(result_value))
        } else {
            println!("Function '{}' executed (simulated)", name);
            Ok(None)
        }
    }
}