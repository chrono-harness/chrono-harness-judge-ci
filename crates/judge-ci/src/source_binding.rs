//! CI submits the methods and original receipt to the same fixed candidate Git reader.
use crate::{
    route_wire::{Operation, Receipt},
    scheduling_git::Reader,
    verified_collection::binding_original,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

pub(super) struct Binding<'a> {
    reader: &'a Reader,
    root: &'a Path,
    compiler_outputs: BTreeSet<&'a str>,
}

impl<'a> Binding<'a> {
    pub fn prepare(
        reader: &'a Reader,
        root: &'a Path,
        operations: &'a [Operation],
        registry: &Value,
    ) -> Result<Self, String> {
        let methods: Vec<_> = operations.iter().map(|step| json!({
            "operation":step.method.operation,"argv":step.method.argv,
            "output_contract":step.method.output_contract,"output_limit_bytes":step.output_limit_bytes
        })).collect();
        reader.call::<()>(
            root,
            json!({"op":"prepare-output-binding","methods":methods,"registry":registry}),
        )?;
        let compiler_outputs = operations
            .iter()
            .filter(|op| op.method.output_contract.is_some())
            .map(|op| op.method.operation.as_str())
            .collect();
        Ok(Self {
            reader,
            root,
            compiler_outputs,
        })
    }

    pub fn requires_compiler_output(&self, operation: &str) -> bool {
        self.compiler_outputs.contains(operation)
    }

    pub fn check(
        &self,
        state: &str,
        operation: &str,
        receipt: Option<&Receipt>,
    ) -> Result<(), String> {
        let original = binding_original(
            state,
            self.requires_compiler_output(operation),
            receipt.map(|r| (r, r.process.stdout_bytes.as_slice())),
        ).map(|(r, stdout)| {
            json!({
                "operation": r.operation, "argv": r.process.argv, "stdout": stdout,
                "exit": r.process.exit_code, "failed": r.process.failure.is_some(),
                "dependency": r.compiler_dependency, "artifact": r.compiler_artifact
            })
        });
        self.reader.call(self.root,json!({"op":"bind-operation-output","state":state,"operation":operation,"original":original}))
    }
}
