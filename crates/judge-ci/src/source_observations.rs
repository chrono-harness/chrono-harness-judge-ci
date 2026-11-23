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
    routes: Option<(usize, Vec<Value>)>,
}

impl GitEvidence {
    pub fn finish(self, mut facts: Value) -> Value {
        if let Some((at, rows)) = self.routes {
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
        let mut extra = reply.git_facts;
        if !extra.is_null() {
            let more = extra
                .as_object_mut()
                .and_then(|binding| binding.remove("processes"));
            let Some(Value::Array(more)) = more else {
                return Err("E_GIT_FACTS: missing routes processes".into());
            };
            if &extra != binding {
                return Err("E_GIT_FACTS: routes observed a different Git binding".into());
            }
            let offset = offset.ok_or("E_GIT_FACTS: unexpected bound routes reader")?;
            self.routes = Some((offset, more));
        }
        error.map_or(Ok(()), Err)
    }
}
