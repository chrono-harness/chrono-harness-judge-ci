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
#[cfg(verus_keep_ghost)]
include!(env!("CHRONO_VERUS_SOURCE_REGISTRATION"));
#[cfg(not(verus_keep_ghost))]
use chrono_judge_ci::verified_collection::validate_reference_syntax;
#[cfg(verus_keep_ghost)]
use source_registration::validate_reference_syntax;
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


fn reference_form(prefix: &str, base: u8) -> (out: source_verifier::sources::continuity::ReferenceSyntax)
    ensures out.file@ == "x"@, out.before@ == prefix@, out.after@ == prefix@, out.base == base,
{
    source_verifier::sources::continuity::ReferenceSyntax {
        file: "x".to_string(), before: prefix.to_string(), after: prefix.to_string(),
        trailing: String::new(), suffix_before: String::new(), suffix_after: String::new(), base,
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn reference_declarations_validate_actual_prefixes_and_explicit_bases() {
    proof {
        reveal_strlit("x"); reveal_strlit("\""); reveal_strlit("/"); reveal_strlit("");
        vstd::utf8::is_ascii_chars_encode_utf8("x"@);
        vstd::utf8::is_ascii_chars_encode_utf8("\""@);
        vstd::utf8::is_ascii_chars_encode_utf8("/"@);
        vstd::utf8::is_ascii_chars_encode_utf8(""@);
        assert(vstd::utf8::encode_utf8("x"@) =~= seq![120u8]);
        assert(vstd::utf8::encode_utf8("\""@) =~= seq![34u8]);
        assert(vstd::utf8::encode_utf8("/"@) =~= seq![47u8]);
        assert(vstd::utf8::encode_utf8(""@) =~= Seq::<u8>::empty());
        assert(source_verifier::sources::canonical_source_path(seq![120u8])) by (compute);
    }
    let quoted = validate_reference_syntax(&vec![reference_form("\"", 0)]);
    let root = validate_reference_syntax(&vec![reference_form("/", 2)]);
    let wrong_base = validate_reference_syntax(&vec![reference_form("/", 0)]);
    let empty = validate_reference_syntax(&vec![reference_form("", 0)]);
    let unknown_base = validate_reference_syntax(&vec![reference_form("\"", 3)]);
    expect(quoted.is_ok() && root.is_ok());
    expect(wrong_base.is_err() && empty.is_err() && unknown_base.is_err());
}

} // verus!

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
