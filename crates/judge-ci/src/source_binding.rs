//! CI submits the methods and original receipt to the same fixed candidate Git reader.
#[cfg(not(verus_keep_ghost))]
use crate::{
    route_wire::{Operation, Receipt},
    scheduling_git::Reader,
    verified_collection::binding_original,
};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::path::Path;
use vstd::{hash_set::StringHashSet, prelude::*};

verus! {
/// These are the exact methods submitted to the candidate reader.
#[cfg_attr(not(verus_keep_ghost), derive(serde::Serialize))]
pub struct BindingMethod<'a, C> {
    pub operation: &'a str,
    pub argv: &'a [String],
    pub output_contract: Option<&'a C>,
    pub output_limit_bytes: usize,
}

pub fn compiler_outputs<C, E>(prepared: Result<(), E>, methods: &[BindingMethod<C>])
    -> (result: Result<StringHashSet, E>)
    ensures match prepared {
        Err(e) => result == Err::<StringHashSet,E>(e),
        Ok(()) => result matches Ok(names) && forall|name: Seq<char>| #[trigger] names@.contains(name)
            <==> exists|j: int| 0 <= j < methods.len()
                && methods@[j].operation@ == name && methods@[j].output_contract.is_some(),
    },
{
    if let Err(e) = prepared { return Err(e); }
    let mut names = StringHashSet::new();
    let mut i = 0usize;
    while i < methods.len()
        invariant i <= methods.len(), forall|name: Seq<char>| #[trigger] names@.contains(name)
            <==> exists|j: int| 0 <= j < i
                && methods@[j].operation@ == name && methods@[j].output_contract.is_some(),
        decreases methods.len() - i,
    {
        let row = &methods[i];
        if row.output_contract.is_some() { names.insert(row.operation.to_string()); }
        i += 1;
    }
    Ok(names)
}
}

#[cfg(not(verus_keep_ghost))]
pub(super) struct Binding<'a> {
    reader: &'a Reader,
    root: &'a Path,
    compiler_outputs: StringHashSet,
}

#[cfg(not(verus_keep_ghost))]
impl<'a> Binding<'a> {
    pub fn prepare(
        reader: &'a Reader,
        root: &'a Path,
        operations: &[Operation],
        registry: &Value,
    ) -> Result<Self, String> {
        let methods: Vec<_> = operations
            .iter()
            .map(|step| BindingMethod {
                operation: step.method.operation.as_str(),
                argv: &step.method.argv,
                output_contract: step.method.output_contract.as_ref(),
                output_limit_bytes: step.output_limit_bytes,
            })
            .collect();
        let prepared = reader.call::<()>(
            root,
            json!({"op":"prepare-output-binding","methods":methods,"registry":registry}),
        );
        let compiler_outputs = compiler_outputs(prepared, &methods)?;
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
        )
        .map(|(r, stdout)| {
            json!({
                "operation": r.operation, "argv": r.process.argv, "stdout": stdout,
                "exit": r.process.exit_code, "failed": r.process.failure.is_some(),
                "dependency": r.compiler_dependency, "artifact": r.compiler_artifact
            })
        });
        self.reader.call(self.root,json!({"op":"bind-operation-output","state":state,"operation":operation,"original":original}))
    }
}
