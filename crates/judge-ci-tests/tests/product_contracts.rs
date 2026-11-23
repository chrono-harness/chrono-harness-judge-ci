//! Contract cases exercised against the pinned routes, registration and runner interfaces.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as source_identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_verifier as source_verifier;
#[cfg(verus_keep_ghost)]
include!(env!("CHRONO_VERUS_CONTINUITY_PLAN"));
#[cfg(not(verus_keep_ghost))]
use chrono_judge_routes::validate_source_plan;
#[cfg(verus_keep_ghost)]
use continuity_plan::validate_source_plan;
#[cfg(verus_keep_ghost)]
include!(env!("CHRONO_VERUS_SOURCE_VERIFIER"));
#[cfg(not(verus_keep_ghost))]
use chrono_judge_ci::verified_collection::validate_reference_syntax;
#[cfg(verus_keep_ghost)]
use source_verifier::identifiers as source_identifiers;
#[path = "support/expect.rs"]
mod expectation;
use expectation::expect;
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn combined_source_plan_keeps_base_errors_before_affected_file_errors() {
    let registry = source_verifier::sources::source_registry(vec![source_verifier::sources::SourceFile {
        path: "x".to_string(), verification: None,
    }], Some(true));
    let nodes = vec!["file:x".to_string()];
    let empty = source_verifier::sources::continuity::Continuity::empty();
    let result = validate_source_plan(&registry, Err("原基线错误\0é"),
        &nodes, &Vec::new(), &Vec::new(), &empty);
    expect(result.is_err());
    let exact = match result { Err(error) => source_identifiers::equal_text(error.path, "原基线错误\0é"), Ok(_) => false };
    expect(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combined_source_plan_rejects_unknown_after_a_successful_inventory() {
    let registry = source_verifier::sources::source_registry(vec![source_verifier::sources::SourceFile {
        path: "x".to_string(), verification: None,
    }], Some(true));
    let nodes = vec!["file:x".to_string()];
    let empty = source_verifier::sources::continuity::Continuity::empty();
    proof {
        reveal_with_fuel(source_verifier::sources::classifications_error, 3);
        reveal_with_fuel(source_verifier::sources::selected_plan_error, 3);
        reveal(source_verifier::sources::file_plan_error);
        reveal_strlit("file:"); reveal_strlit("x"); reveal_strlit("file:x");
        assert("file:x"@ =~= "file:"@ + "x"@);
        assert(source_verifier::sources::node_affected("file:"@, "x"@, nodes@)) by {
            assert(nodes[0]@ == "file:"@ + "x"@);
        }
    }
    let result = validate_source_plan(&registry, Ok(Some(&registry)),
        &nodes, &Vec::new(), &Vec::new(), &empty);
    expect(result.is_err());
    let exact = match result { Err(error) => source_identifiers::equal_text(error.message,
        "E_VERUS_COVERAGE: affected file lacks explicit verification classification: "), Ok(_) => false };
    expect(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn combined_source_plan_accepts_the_complete_empty_initial_inventory() {
    let registry = source_verifier::sources::source_registry(Vec::new(), Some(true));
    let empty = source_verifier::sources::continuity::Continuity::empty();
    let result = validate_source_plan(&registry, Ok(None),
        &Vec::new(), &Vec::new(), &Vec::new(), &empty);
    expect(result.is_ok());
}


// Registration proves prefix and base admission for the source owner's path verdict.
#[cfg_attr(not(verus_keep_ghost), test)]
fn reference_file_x_is_canonical() {
    proof {
        reveal_strlit("x");
        vstd::utf8::is_ascii_chars_encode_utf8("x"@);
        assert(vstd::utf8::encode_utf8("x"@) =~= seq![120u8]);
        assert(source_verifier::sources::canonical_source_path(seq![120u8])) by (compute);
    }
    expect(source_verifier::sources::canonical_source_name("x"));
}

} // verus!

#[cfg(not(verus_keep_ghost))]
#[test]
fn reference_declarations_validate_actual_prefixes_and_explicit_bases() {
    use chrono_harness::verified_verifier::sources::continuity::ReferenceSyntax;
    let check = |prefix: &str, base| {
        let form = ReferenceSyntax {
            file: "x".into(),
            before: prefix.into(),
            after: prefix.into(),
            trailing: String::new(),
            suffix_before: String::new(),
            suffix_after: String::new(),
            base,
        };
        validate_reference_syntax(&vec![form])
    };
    assert!(check("\"", 0).is_ok());
    assert!(check("/", 2).is_ok());
    assert!(check("/", 0).is_err());
    assert!(check("", 0).is_err());
    assert!(check("\"", 3).is_err());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn ci_rows_preserve_the_runner_protocol_in_both_directions() {
    use chrono_judge_ci::verified_collection::result_types::{CheckResult, Status};
    for (wire, expected) in [
        ("passed", Status::Passed),
        ("failed", Status::Failed),
        ("not-required", Status::NotRequired),
        ("blocked", Status::Blocked),
    ] {
        let raw = serde_json::json!({
            "id":"build", "status":wire, "cause":"原始错误\0é", "exit_code":-7
        });
        let incoming: chrono_harness::CheckResult = serde_json::from_value(raw.clone()).unwrap();
        let row = CheckResult::from(incoming);
        assert!(row.status == expected);
        assert_eq!(row.cause, "原始错误\0é");
        assert_eq!(row.exit_code, Some(-7));
        let outgoing = chrono_harness::CheckResult::from(row);
        assert_eq!(serde_json::to_value(outgoing).unwrap(), raw);
    }
}
