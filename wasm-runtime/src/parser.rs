use wasmparser::{Parser, Payload};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug)]
pub struct WasmModule {
    pub exports: HashMap<String, u32>,
    pub function_count: u32,
}

impl WasmModule {
    pub fn new() -> Self {
        Self {
            exports: HashMap::new(),
            function_count: 0,
        }
    }

    pub fn parse(wasm_bytes: &[u8]) -> Result<Self> {
        let mut module = WasmModule::new();
        let parser = Parser::new(0);

        for payload in parser.parse_all(wasm_bytes) {
            match payload? {
                Payload::FunctionSection(reader) => {
                    module.function_count = reader.count();
                    // Count the number of functions
                    for _ in reader {
                        // Actual processing would be done here
                    }
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export?;
                        if let wasmparser::ExternalKind::Func = export.kind {
                            module.exports.insert(export.name.to_string(), export.index);
                        }
                    }
                }
                _ => {
                    // Ignore other sections
                }
            }
        }

        Ok(module)
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.exports.contains_key(name)
    }
}