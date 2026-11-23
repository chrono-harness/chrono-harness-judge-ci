use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reply {
    previous: Value,
    candidate: String,
    error: Value,
    git_facts: Value,
}

/// The reader can continue after the query, including the final clean-tree check.
#[derive(Default)]
pub(crate) struct GitEvidence {
    pub opening: Value,
    external: Vec<(usize, Vec<Value>)>,
}

impl GitEvidence {
    pub fn finish(self, mut facts: Value) -> Value {
        for (at, rows) in self.external.into_iter().rev() {
            // One append-only caller reader is used from inventory to publication.
            facts["processes"]
                .as_array_mut()
                .expect("bound Git reader")
                .splice(at..at, rows);
        }
        if facts.is_null() { self.opening } else { facts }
    }

    pub fn admit(
        &mut self,
        previous: Option<&str>,
        candidate: &str,
        binding: &Value,
        offset: Option<usize>,
        reply: Reply,
    ) -> Result<(), String> {
        if reply.previous != json!(previous) || reply.candidate != candidate {
            return Err("E_PLAN_BINDING: source plan returned different endpoints".into());
        }
        let error: Option<String> =
            serde_json::from_value(reply.error).map_err(|error| error.to_string())?;
        self.record(binding, offset, reply.git_facts)?;
        error.map_or(Ok(()), Err)
    }

    pub fn record(
        &mut self,
        binding: &Value,
        offset: Option<usize>,
        mut extra: Value,
    ) -> Result<(), String> {
        if !extra.is_null() {
            let more = extra
                .as_object_mut()
                .and_then(|binding| binding.remove("processes"));
            let Some(Value::Array(more)) = more else {
                return Err("E_GIT_FACTS: missing external Git processes".into());
            };
            if &extra != binding {
                return Err(
                    "E_GIT_FACTS: external command observed a different Git binding".into(),
                );
            }
            let offset = offset.ok_or("E_GIT_FACTS: unexpected bound external Git reader")?;
            self.external.push((offset, more));
        }
        Ok(())
    }
}

pub(crate) fn binding(facts: &Value) -> Result<(Value, Option<usize>), String> {
    let binding = facts
        .as_object()
        .map(|fields| {
            Value::Object(
                fields
                    .iter()
                    .filter(|(key, _)| *key != "processes")
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        })
        .unwrap_or(Value::Null);
    let offset = if binding.is_null() {
        None
    } else {
        Some(
            facts["processes"]
                .as_array()
                .ok_or("E_GIT_FACTS: missing caller processes")?
                .len(),
        )
    };
    Ok((binding, offset))
}
