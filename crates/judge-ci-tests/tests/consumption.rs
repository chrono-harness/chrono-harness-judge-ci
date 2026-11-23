use chrono_judge_ci::consumption::{self, Endpoint};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const UNITS: &str = ".chrono-harness/ci/units.json";
const STARTUP: &str = ".chrono-harness/ci/bootstrap-shared.json";
const CORE: &str = ".chrono-harness/ci/bootstrap-core.json";
const PREPARATION: &str = ".chrono-harness/ci/bootstrap-unit.json";
const CACHE: &str = ".chrono-harness/ci/bootstrap-cache.json";
const CHECK: &str = ".chrono-harness/ci/check.json";
const FM: &str = ".chrono-harness/FILEMAP.json";
const POLICY: &str = ".chrono-harness/git/macos-27.json";

fn host() -> Endpoint {
    let inputs: BTreeMap<String, Value> =
        serde_json::from_str(include_str!("fixtures/startup.json")).unwrap();
    let read = |path: &str| inputs[path].clone();
    Endpoint {
        adoption: read(CHECK)["policy"].get("causal_consumption").cloned(),
        filemap: read(FM),
        startup: Some(serde_json::from_value(read("startup_execution")).unwrap()),
        config: read(POLICY),
        config_paths: BTreeSet::from([POLICY.into()]),
        documents: [UNITS, STARTUP, CORE, CACHE, CHECK, PREPARATION]
            .into_iter()
            .map(|p| (p.into(), read(p)))
            .collect(),
        unavailable: BTreeMap::new(),
    }
}
fn paths(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|s| s.to_string()).collect()
}
fn method<'a>(s: &'a mut Endpoint, op: &str) -> &'a mut consumption::StartupMethod {
    &mut s
        .startup
        .as_mut()
        .unwrap()
        .methods
        .as_mut()
        .unwrap()
        .get_mut(op)
        .unwrap()[0]
}
fn retire_method(s: &mut Endpoint, op: &str, new_id: &str) {
    let methods = s.startup.as_mut().unwrap().methods.as_mut().unwrap();
    let previous = methods.remove(op).unwrap();
    assert!(methods.insert(new_id.to_owned(), previous).is_none());
}

#[test]
fn registered_startup_chain_requires_declared_mock_evidence() {
    let s = host();
    let result = consumption::validate_delta(None, &s, &BTreeSet::new(), &BTreeSet::new()).unwrap();
    assert_eq!(result.checked, paths(&["runner-startup"]));
    assert_eq!(
        result.required_tests,
        paths(&["test:host-bootstrap-native-tests"])
    );
    let mut tools_only = host();
    tools_only.documents.get_mut(CORE).unwrap()["install"]
        .as_array_mut()
        .unwrap()
        .retain(|output| output["destination"] != ".chrono-harness/bin/verus.zip");
    tools_only.documents.get_mut(PREPARATION).unwrap()["operations"] = json!([]);
    tools_only.adoption.as_mut().unwrap()["shared_startup"][0]
        .as_object_mut()
        .unwrap()
        .remove("data_outputs");
    assert_eq!(
        consumption::validate_delta(None, &tools_only, &BTreeSet::new(), &BTreeSet::new())
            .unwrap()
            .required_tests,
        paths(&["test:host-bootstrap-native-tests"])
    );
    let mut legacy = host();
    legacy.adoption = None;
    assert!(
        consumption::validate_delta(
            Some(&legacy),
            &legacy,
            &paths(&["unrelated.md"]),
            &BTreeSet::new()
        )
        .unwrap()
        .checked
        .is_empty()
    );
}

#[test]
fn startup_commands_preserve_registered_arguments() {
    let old = host();
    for operation in [
        "host.bootstrap-shared-produce",
        "host.bootstrap-shared-install-unit_runner",
    ] {
        let mut candidate = host();
        method(&mut candidate, operation)
            .argv
            .insert(0, "-B".into());
        let error =
            consumption::validate_delta(Some(&old), &candidate, &paths(&[CORE]), &BTreeSet::new())
                .unwrap_err();
        assert!(
            error.contains("producer/caller invocation mismatch"),
            "{error}"
        );
    }
}

#[test]
fn startup_data_requires_installed_output_and_selected_producer_and_consumer() {
    for defect in [
        "uninstalled",
        "producer",
        "consumer",
        "unregistered-consumer",
        "no-preparation",
        "undeclared",
    ] {
        let old = host();
        let mut s = host();
        let chain = &mut s.adoption.as_mut().unwrap()["shared_startup"][0];
        match defect {
            "uninstalled" => {
                let output = chain["data_outputs"][".chrono-harness/bin/verus.zip"].clone();
                chain["data_outputs"][".chrono-harness/bin/absent.zip"] = output;
            }
            "producer" => {
                chain["data_outputs"][".chrono-harness/bin/verus.zip"]["producer"] =
                    json!("host.verus-download")
            }
            "consumer" => {
                chain["data_outputs"][".chrono-harness/bin/verus.zip"]["consumer"] =
                    json!("host.verus-download")
            }
            "unregistered-consumer" => {
                chain["data_outputs"][".chrono-harness/bin/verus.zip"]["consumer"] =
                    json!("absent.operation")
            }
            "no-preparation" => {
                chain.as_object_mut().unwrap().remove("preparation");
            }
            "undeclared" => {
                chain.as_object_mut().unwrap().remove("data_outputs");
            }
            _ => unreachable!(),
        }
        let error = consumption::validate_delta(Some(&old), &s, &paths(&[CHECK]), &BTreeSet::new())
            .unwrap_err();
        let expected = match defect {
            "uninstalled" => "not installed",
            "undeclared" => "declared producer claim",
            "producer" => "declared producer operation",
            "consumer" => "preparation operation",
            "unregistered-consumer" => "operation",
            "no-preparation" => "preparation consumer",
            _ => unreachable!(),
        };
        assert!(error.contains(expected), "{defect}: {error}");
    }
}

#[test]
fn affected_chain_rejects_missing_dangling_and_misbound_truths() {
    for defect in [
        "need",
        "use",
        "caller",
        "producer",
        "unit",
        "profiles",
        "output",
        "missing-output",
        "source",
        "duplicate-output",
        "check",
        "verify",
        "evidence-method",
        "evidence-plan",
        "evidence-output",
        "missing-doc",
        "unregistered-doc",
    ] {
        let old = host();
        let mut s = host();
        match defect {
            "need" => {
                s.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_runner"].as_object_mut().unwrap().remove("need");
            }
            "use" => {
                s.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_runner"]
                    ["output_use"] = json!("")
            }
            "caller" => {
                method(&mut s, "host.bootstrap-shared-install-unit_runner").argv[4] =
                    "unit_ci".into()
            }
            "producer" => retire_method(&mut s, "host.bootstrap-shared-produce", "retired.produce"),
            "unit" => {
                s.documents.get_mut(UNITS).unwrap()["units"]
                    .as_object_mut()
                    .unwrap()
                    .remove("runner");
            }
            "profiles" => {
                s.documents.get_mut(STARTUP).unwrap()["consumers"]["unit_runner"] = json!([])
            }
            "output" => {
                s.documents.get_mut(CORE).unwrap()["install"][0]["destination"] =
                    json!("unrelated/tool")
            }
            "missing-output" => {
                s.documents.get_mut(CORE).unwrap()["install"]
                    .as_array_mut()
                    .unwrap()
                    .remove(1);
            }
            "source" => {
                s.documents.get_mut(CORE).unwrap()["install"][0]["source"] = json!("unrelated/tool")
            }
            "duplicate-output" => {
                let row = s.documents[CORE]["install"][0].clone();
                s.documents.get_mut(CORE).unwrap()["install"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            "check" => s.config["canonical_check"]["argv"] = json!(["some-other-runner", "check"]),
            "verify" => {
                method(&mut s, "host.bootstrap-shared-verify-core").argv[4] =
                    "absent-profile".into()
            }
            "evidence-method" => retire_method(
                &mut s,
                "test.host-bootstrap-native-tests",
                "retired.evidence",
            ),
            "evidence-plan" => {
                *s.startup
                    .as_mut()
                    .unwrap()
                    .plans
                    .as_mut()
                    .unwrap()
                    .get_mut("test:host-bootstrap-native-tests")
                    .unwrap() = vec!["build.host-bootstrap-native".into()]
            }
            "evidence-output" => {
                s.filemap["execution_scheduling"]["claims"]["test.host-bootstrap-native-tests"]
                    .as_object_mut()
                    .unwrap()
                    .remove("outputs");
            }
            "missing-doc" => {
                s.documents.remove(CORE);
            }
            "unregistered-doc" => s.filemap["files"]
                .as_array_mut()
                .unwrap()
                .retain(|f| f["path"] != CORE),
            _ => unreachable!(),
        }
        let error = consumption::validate_delta(Some(&old), &s, &paths(&[CORE]), &BTreeSet::new())
            .unwrap_err();
        assert!(error.starts_with("E_CONSUMPTION:"), "{defect}: {error}");
    }
}

#[test]
fn preparation_requires_registered_invocation_and_early_verification() {
    for defect in [
        "invocation",
        "missing",
        "ordinary-operation",
        "duplicate",
        "schema",
        "entrypoint",
        "document",
        "tool",
        "projects",
    ] {
        let old = host();
        let mut new = host();
        match defect {
            "invocation" => {
                new.documents.get_mut(UNITS).unwrap()["units"]["runner"]["bootstrap"][2] =
                    json!("other")
            }
            "missing" => new.documents.get_mut(PREPARATION).unwrap()["prerequisites"] = json!([]),
            "ordinary-operation" => {
                new.documents.get_mut(PREPARATION).unwrap()["prerequisites"] = json!([]);
                new.documents.get_mut(PREPARATION).unwrap()["operations"] =
                    json!(["host.bootstrap-shared-verify-core"]);
            }
            "duplicate" => {
                new.documents.get_mut(PREPARATION).unwrap()["prerequisites"] = json!([
                    "host.bootstrap-shared-verify-core",
                    "host.bootstrap-shared-verify-core"
                ])
            }
            "schema" => new.documents.get_mut(PREPARATION).unwrap()["schema"] = json!("other"),
            "tool" => new.documents.get_mut(PREPARATION).unwrap()["tools"]["bash"] = json!("other"),
            "projects" => new.documents.get_mut(PREPARATION).unwrap()["projects"] = json!("other"),
            "entrypoint" => {
                new.documents.get_mut(PREPARATION).unwrap()["entrypoint"]["script"] = json!("other")
            }
            "document" => {
                new.documents.remove(PREPARATION);
            }
            _ => unreachable!(),
        }
        assert!(
            consumption::validate_delta(Some(&old), &new, &paths(&[PREPARATION]), &BTreeSet::new())
                .unwrap_err()
                .starts_with("E_CONSUMPTION:"),
            "{defect}"
        );
    }
}

#[test]
fn both_endpoints_select_removed_inputs_and_old_owners_without_global_rejudgment() {
    let mut old = host();
    let mut new = host();
    // A historical malformed need remains scoped until its chain is affected.
    new.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_runner"]["need"] =
        json!("");
    let unaffected = consumption::validate_delta(
        Some(&old),
        &new,
        &paths(&["unrelated.md"]),
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(unaffected.historical, paths(&["runner-startup"]));
    assert!(
        consumption::validate_delta(
            Some(&old),
            &new,
            &BTreeSet::new(),
            &paths(&["project:runner"])
        )
        .is_err()
    );
    // Removed old profile input must select even after the new declaration no longer references it.
    let old_core = ".chrono-harness/ci/old-core.json";
    old.documents
        .insert(old_core.into(), old.documents[CORE].clone());
    let mut registration = old.filemap["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == CORE)
        .unwrap()
        .clone();
    registration["path"] = json!(old_core);
    old.filemap["files"]
        .as_array_mut()
        .unwrap()
        .push(registration);
    old.documents.get_mut(STARTUP).unwrap()["profiles"]["core"]["config"] = json!(old_core);
    assert!(
        consumption::validate_delta(Some(&old), &new, &paths(&[old_core]), &BTreeSet::new())
            .is_err()
    );
    // Old operation owners select a defect even after the candidate moves that action.
    retire_method(&mut new, "build.runner", "retired.runner");
    assert!(
        consumption::validate_delta(
            Some(&old),
            &new,
            &BTreeSet::new(),
            &paths(&["project:runner"])
        )
        .is_err()
    );
}

#[test]
fn retired_contract_and_unknown_versions_fail_while_legacy_consumers_keep_meaning() {
    let old = host();
    let mut new = host();
    new.adoption = None;
    assert!(
        consumption::validate_delta(Some(&old), &new, &BTreeSet::new(), &BTreeSet::new())
            .unwrap_err()
            .contains("E_CONSUMPTION_RETIRED")
    );
    for value in [
        json!(null),
        json!({"schema":"chrono-internal-causal-consumption/v2","shared_startup":[]}),
        json!({"schema":"chrono-internal-causal-consumption/v1","shared_startup":[]}),
    ] {
        new.adoption = Some(value);
        assert!(consumption::contract(new.adoption.as_ref()).is_err());
    }
    for defect in [
        "unknown-field",
        "duplicate-chain",
        "empty-identity",
        "bad-path",
        "missing-evidence",
    ] {
        let mut adoption = host().adoption.unwrap();
        match defect {
            "unknown-field" => adoption["unrecognized"] = json!(true),
            "duplicate-chain" => {
                let row = adoption["shared_startup"][0].clone();
                adoption["shared_startup"].as_array_mut().unwrap().push(row);
            }
            "empty-identity" => adoption["shared_startup"][0]["caller"] = json!(""),
            "bad-path" => adoption["shared_startup"][0]["units"] = json!("../outside"),
            "missing-evidence" => {
                adoption["shared_startup"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("evidence");
            }
            _ => unreachable!(),
        }
        assert!(consumption::contract(Some(&adoption)).is_err(), "{defect}");
    }
    let mut new = host();
    new.documents.get_mut(UNITS).unwrap()["job_gating"]["startup"]["consumers"]["unit_ci"]["need"] =
        json!("");
    consumption::validate_delta(Some(&old), &new, &paths(&[UNITS]), &BTreeSet::new()).unwrap();
}

#[test]
fn scoped_adapter_reads_only_the_requested_endpoints_and_preserves_failures() {
    struct Documents {
        base: BTreeMap<String, Value>,
        candidate: BTreeMap<String, Value>,
    }
    impl consumption::ScopedDocuments for Documents {
        fn read_json(&self, commit: &str, path: &str) -> Result<Value, String> {
            let entries = match commit {
                "base" => &self.base,
                "candidate" => &self.candidate,
                other => return Err(format!("unexpected commit {other}")),
            };
            entries
                .get(path)
                .cloned()
                .ok_or_else(|| format!("missing {path}"))
        }
        fn contains(&self, commit: &str, path: &str) -> Result<bool, String> {
            Ok(self.read_json(commit, path).is_ok())
        }
        fn configuration(&self, commit: &str, path: &str) -> Result<(String, Value), String> {
            Ok((path.to_owned(), self.read_json(commit, path)?))
        }
    }
    let base: BTreeMap<String, Value> =
        serde_json::from_str(include_str!("fixtures/startup.json")).unwrap();
    let mut candidate = base.clone();
    candidate.get_mut(CORE).unwrap()["install"][0]["source"] = json!("wrong/producer");
    let startup = serde_json::from_value(base["startup_execution"].clone()).unwrap();
    let docs = Documents { base, candidate };
    let check = |oid: &str, changed: &BTreeSet<String>| {
        consumption::check_scoped(
            &docs,
            Some("base"),
            oid,
            FM,
            |_| Some(&startup),
            Some(POLICY),
            CHECK,
            changed,
            &BTreeSet::new(),
        )
    };
    assert!(
        check("base", &paths(&[CORE]))
            .unwrap()
            .required_tests
            .contains("test:host-bootstrap-native-tests")
    );
    let error = check("candidate", &paths(&[CORE])).unwrap_err();
    assert!(
        error.contains("wrong/producer") && error.contains("declared producer claim"),
        "{error}"
    );
    assert!(
        check("candidate", &paths(&["unrelated.md"]))
            .unwrap()
            .checked
            .is_empty()
    );
}
