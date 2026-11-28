//! Boundaries of the CI judge's collection policy, checked against the production implementation.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../judge-ci/src/verified_collection.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_judge_ci::verified_collection as policy;
use policy::result_types::{CheckResult, Status};
#[path = "support/expect.rs"]
mod expectation;
use expectation::expect;
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "requirements.rs"]
mod native_requirements;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn scoped_requests_and_original_git_rows_follow_host_inputs() {
    use policy::judge_request::inherited_key_allowed;
    let name = [80u8,65u8,84u8,72u8];
    expect(inherited_key_allowed(&name,false,false));
    expect(!inherited_key_allowed(&name,true,false));
    expect(!inherited_key_allowed(&name,false,true));
    expect(!inherited_key_allowed(&[],false,false));
    expect(!inherited_key_allowed(&[65u8,61u8],false,false));
    expect(!inherited_key_allowed(&[0u8,66u8],false,false));

    use policy::{judge_request::valid_oid_bytes,request_wire::{request_header,Scope},git_rows::external_rows,trace_metadata::publish_metadata};
    let invalid = [65u8];
    expect(!valid_oid_bytes(&invalid));
    let scope = Some(Scope::Collect { manifest:"reports".to_string() });
    proof { reveal_strlit("id"); reveal_strlit(""); reveal_strlit("chrono-ci-judge/v1"); reveal_strlit("chrono-ci-judge/v2");
        assert("id"@.len() != ""@.len());
        assert("chrono-ci-judge/v1"@[17] != "chrono-ci-judge/v2"@[17]);
    }
    expect(request_header(&scope,"chrono-ci-judge/v2","id"));
    expect(!request_header(&None,"chrono-ci-judge/v2","id"));
    expect(!request_header(&scope,"chrono-ci-judge/v2",""));
    expect(matches!(external_rows(Some(91u8),true,Some(3usize)),Ok((3,91))));
    expect(external_rows(Some(91u8),true,None).is_err());
    expect(external_rows(None::<u8>,false,None).is_err());
    expect(publish_metadata(true,|| {}));
    expect(!publish_metadata(false,|| {}));
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn scoped_endpoints_and_original_preparation_keep_their_coordinates() {
    use policy::registration_reply::{collection_scope, endpoints_match};
    use policy::request_wire::{Scope, adopted_base_matches, initial_parent_allowed, scope_matches, scoped_base};
    proof {
        reveal_strlit("example");
        reveal_strlit("harness");
        reveal_strlit("before");
        reveal_strlit("after");
        reveal_strlit("collect");
        assert("before"@.len() != "after"@.len());
        assert("example"@.len() != "collect"@.len() || "example"@[0] != "collect"@[0]);
        assert("example"@[0] != "harness"@[0]);
    }
    use policy::verified_text::equal_text;
    match scoped_base(true, Some("before")) {
        Err(error) => expect(equal_text(error, "initial cannot have base")),
        Ok(_) => expect(false),
    }
    match scoped_base(false, None) {
        Err(error) => expect(equal_text(error, "base required")),
        Ok(_) => expect(false),
    }
    expect(matches!(scoped_base(true, None), Ok(None)));
    match scoped_base(false, Some("before")) {
        Ok(Some(base)) => expect(equal_text(base, "before")),
        _ => expect(false),
    }
    expect(initial_parent_allowed(&[]));
    expect(!initial_parent_allowed(&["before".to_string()]));
    expect(adopted_base_matches(Some("before"), "before"));
    expect(!adopted_base_matches(Some("after"), "before"));
    expect(!adopted_base_matches(None, "before"));
    expect(collection_scope(Some("collect")));
    expect(!collection_scope(Some("example")));
    expect(!collection_scope(None));
    expect(endpoints_match(("before", "after"), ("before", "after")));
    expect(!endpoints_match(("before", "after"), ("after", "before")));

    let unit = Some(Scope::Unit { unit: "example".to_string() });
    let other = Some(Scope::Unit { unit: "harness".to_string() });
    let collect = Some(Scope::Collect { manifest: "example".to_string() });
    let different_manifest = Some(Scope::Collect { manifest: "harness".to_string() });
    expect(scope_matches(&unit, &unit));
    expect(!scope_matches(&unit, &other));
    expect(!scope_matches(&unit, &collect));
    expect(scope_matches(&collect, &collect));
    expect(!scope_matches(&collect, &different_manifest));
    expect(!scope_matches(&None, &unit));
    expect(scope_matches(&None, &None));
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn session_frames_and_terminal_failures_stay_bound() {
    use policy::{diagnostic_status::terminal_matches, git_terminal::terminal_error, original_ack::admitted_bytes,
        retained_session::{close_result,CloseFailure}, session_frame::complete_line, session_result::finish_reply};
    expect(!complete_line(&[123u8,125u8]));
    expect(complete_line(&[123u8,125u8,10u8]));
    expect(!complete_line(&[]));
    expect(terminal_matches(Some(2),Some(2),0));
    expect(!terminal_matches(None,None,0));
    expect(!terminal_matches(Some(0),Some(0),1));
    expect(terminal_error(true,true,true).is_some());
    expect(terminal_error(false,true,true).is_some());
    expect(matches!(admitted_bytes(Ok::<u8,u8>(9),Ok(false)), Err(None)));
    expect(matches!(admitted_bytes(Err::<u8,u8>(1),Err(3)), Err(Some(3u8))));
    expect(matches!(admitted_bytes(Ok::<u8,u8>(9),Ok(true)), Ok(9u8)));
    expect(matches!(finish_reply(Err::<u8,u8>(1),Err(2)), Err(1u8)));
    expect(matches!(finish_reply(Ok::<u8,u8>(1),Err(2)), Err(2u8)));
    match close_result::<u8,u8>(Err(1),Err(2)) {
        Err(CloseFailure::Both(a,b)) => expect(a==1 && b==2),
        _ => expect(false),
    }
}


fn original_coordinates_and_receipt_counts(id: &str, base: &str, head: &str, count: usize) {
    use policy::registration_reply::projection_matches;
    use policy::route_reply::receipt_header;
    expect(projection_matches((id, base, head), (id, base, head)));
    let swapped = projection_matches((id, base, head), (id, head, base));
    let equal = policy::verified_text::equal_text(base, head);
    expect(swapped == equal);
    expect(receipt_header(id, id, count, count));
    if count < usize::MAX {
        expect(!receipt_header(id, id, count, count + 1));
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn collected_reports_bind_original_endpoints_and_operation_counts() {
    original_coordinates_and_receipt_counts("request", "before", "after", 0);
    original_coordinates_and_receipt_counts("request", "same", "same", usize::MAX);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn route_plan_replies_bind_selection_scope_and_identity() {
    use policy::route_reply::{ReplyMismatch, plan_identity, prepared_plan, selected_plans};
    expect(selected_plans(true, true).is_ok());
    expect(matches!(selected_plans(false, true), Err(ReplyMismatch::Selection)));
    expect(matches!(selected_plans(true, false), Err(ReplyMismatch::Selection)));
    expect(prepared_plan(true, true, true).is_ok());
    expect(matches!(prepared_plan(false, true, true), Err(ReplyMismatch::PreparedPlan)));
    expect(matches!(prepared_plan(true, false, true), Err(ReplyMismatch::PreparedPlan)));
    expect(matches!(prepared_plan(true, true, false), Err(ReplyMismatch::PreparedPlan)));

    proof {
        reveal_strlit("a");
        reveal_strlit("b");
        assert("a"@[0] != "b"@[0]);
    }
    expect(plan_identity("a", "a").is_ok());
    expect(matches!(plan_identity("a", "b"), Err(ReplyMismatch::PlanIdentity)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn full_route_reply_keeps_required_unit_order_and_presence() {
    use policy::route_reply::{ReplyMismatch, required_units};
    let mut declared = Vec::new();
    declared.push("a".to_string());
    declared.push("b".to_string());
    let mut reversed = Vec::new();
    reversed.push("b".to_string());
    reversed.push("a".to_string());
    let declared = Some(declared);
    let reversed = Some(reversed);
    proof {
        reveal_strlit("a");
        reveal_strlit("b");
        assert("a"@[0] != "b"@[0]);
        assert(!((declared.unwrap()@[0])@ == (reversed.unwrap()@[0])@));
    }
    expect(required_units(&declared, &declared).is_ok());
    expect(matches!(required_units(&declared, &reversed), Err(ReplyMismatch::RequiredUnits)));
    expect(matches!(required_units(&None, &Some(Vec::new())), Err(ReplyMismatch::RequiredUnits)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn same_count_receipt_reply_must_include_every_planned_key() {
    use policy::route_reply::{ReplyMismatch, receipt_operations};
    let planned = [1u8, 2u8];
    let valid = receipt_operations("a", "a", &planned, 2, |key: &u8| -> (found: bool)
        ensures found == (*key == 1 || *key == 2)
        { *key == 1 || *key == 2 });
    let wrong_keys = |key: &u8| -> (found: bool)
        ensures found == (*key == 1 || *key == 3)
        { *key == 1 || *key == 3 };
    let substituted = receipt_operations("a", "a", &planned, 2, wrong_keys);
    proof {
        if let Err(ReplyMismatch::ReceiptOperations) = valid {
            let i = choose|i: int| 0 <= i < 2 && #[trigger] planned@[i] != 1u8 && planned@[i] != 2u8;
            assert(i == 0 || i == 1);
        }
        if substituted.is_ok() {
            assert(planned@[1] == 2u8);
            assert(wrong_keys.ensures((&planned@[1],), true));
        }
    }
    expect(valid.is_ok());
    expect(matches!(substituted, Err(ReplyMismatch::ReceiptOperations)));
}


#[cfg(verus_keep_ghost)]
fn original_tool_streams(exit: i32, out: Vec<u8>, err: Vec<u8>) {
    use policy::pairs::{PairFailure, pair_response};
    let ghost want = if exit != 0 { Err(PairFailure::Child(err)) }
        else if out.len() == 0 { Ok(()) } else { Err(PairFailure::UnexpectedOutput) };
    let checked = pair_response(exit, out, err);
    assert(checked == want);
}


#[cfg(verus_keep_ghost)]
fn unit_environment_only_accepts_the_declared_form(label: &str, prepared: bool) {
    use policy::unit_admission::identity_environment;
    let observed = identity_environment(Some(label), prepared);
    let literal = identity_environment(None, prepared);
    assert(literal == Some(false));
    assert(observed == if label@ == "sha256"@ && !prepared { Some(true) } else { None });
}

#[cfg(verus_keep_ghost)]
fn collection_protocol_and_row_identity(unit: String, manifest: String, row: &CheckResult) {
    use policy::unit_protocol::selected_protocol;
    use policy::unit_admission::{response_header, row_error};
    let unit_version = selected_protocol(&Some(unit), "chrono-ci-judge/v1");
    let collect_version = selected_protocol(&Some(manifest), "chrono-ci-judge/v1");
    let unscoped = selected_protocol(&None::<String>, "chrono-ci-judge/v1");
    let name = row.id.as_str();
    let valid = response_header(unit_version, name, name, 1, true);
    let wrong = response_header(unscoped, name, name, 1, true);
    let empty = response_header(collect_version, name, name, 0, true);
    let missing = response_header(collect_version, name, name, 1, false);
    proof {
        reveal_strlit("chrono-ci-judge/v1"); reveal_strlit("chrono-ci-judge/v2");
        assert("chrono-ci-judge/v1"@[17] != "chrono-ci-judge/v2"@[17]);
    }
    assert(valid && !wrong && !empty && !missing);
    let duplicate = row_error(row, false);
    assert(duplicate == Some("invalid/duplicate judge result"));
    let result = row_error(row, true);
    assert(row.id@ != ""@ && row.cause@ != ""@ ==> result.is_some() == match row.status {
            Status::Passed => match row.exit_code { Some(value) => value != 0, None => false },
            Status::Failed => row.exit_code == Some(0i32),
            Status::NotRequired => row.exit_code.is_some(),
            Status::Blocked => false,
        });
}


#[cfg(verus_keep_ghost)]
fn exit_status_survives_diagnostic_delivery(code: u8, sink_accepts: bool) {
    let selected = policy::cli::finish(Ok::<u8, u8>(code), |_error: u8| false);
    let rejected = policy::cli::finish(Err::<u8, u8>(17), |error: u8| error == 17 && sink_accepts);
    assert(selected == code && rejected == 2);
}


#[cfg(verus_keep_ghost)]
fn exact_timeout_edges() {
    let before = policy::timeout_bounds::decimal_timeout(9007199254740991);
    let boundary = policy::timeout_bounds::decimal_timeout(9007199254740992);
    let next = policy::timeout_bounds::decimal_timeout(9007199254740993);
    let last = policy::timeout_bounds::decimal_timeout(u64::MAX);
    assert(!before && boundary && next && last);
}

/// The last representable byte succeeds; counting one more must fail.
fn observed_file_length_boundaries(previous: u64) {
    let remaining = u64::MAX - previous;
    expect(policy::file_counts::stream_byte_count(previous, remaining) == Some(u64::MAX));
    expect(policy::file_counts::stream_byte_count(previous, 0) == Some(previous));
    if remaining < u64::MAX {
        expect(policy::file_counts::stream_byte_count(previous, remaining + 1).is_none());
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn text_comparison_preserves_exact_utf8_contents() {
    use policy::verified_text::equal_text;
    proof {
        reveal_strlit(""); reveal_strlit("编"); reveal_strlit("编译"); reveal_strlit("编码");
        assert(""@.len() != "编"@.len());
        assert("编译"@.len() != "编"@.len());
        assert("编译"@[1] != "编码"@[1]);
    }
    expect(equal_text("", ""));
    expect(equal_text("编译", "编译"));
    expect(!equal_text("", "编"));
    expect(!equal_text("编译", "编"));
    expect(!equal_text("编译", "编码"));
}

/// Independently answer one query by reading the submitted methods from the end.
fn compare_compiler_output<C>(methods: &[policy::output_contracts::BindingMethod<C>], query: &str) {
    use policy::output_contracts::compiler_outputs;
    let prepared = compiler_outputs::<C,u8>(Ok(()), methods);
    let names = match prepared { Ok(names) => names, Err(_) => { expect(false); return; } };
    let required = names.contains(query);
    let mut end = methods.len();
    while end > 0
        invariant end <= methods.len(),
            required == (exists|j: int| 0 <= j < methods.len()
                && methods@[j].operation@ == query@ && methods@[j].output_contract.is_some()),
            forall|j: int| end <= j < methods.len()
                ==> methods@[j].operation@ != query@ || methods@[j].output_contract.is_none(),
        decreases end,
    {
        end -= 1;
        let row = &methods[end];
        if policy::verified_text::equal_text(row.operation, query) {
            if let Some(_) = &row.output_contract {
                assert(methods@[end as int].operation@ == query@);
                expect(required);
                return;
            }
        }
    }
    expect(!required);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_queries_read_any_matching_contract_after_successful_preparation() {
    use policy::output_contracts::{BindingMethod, compiler_outputs};
    let empty: [BindingMethod<(u8, u8)>; 0] = [];
    compare_compiler_output(&empty, "unknown");
    let contracts = [(1u8, 2u8), (2u8, 3u8)];
    let argv: [String; 0] = [];
    let rows = [
        BindingMethod { operation: "ordinary", argv: &argv, output_contract: None, output_limit_bytes: 32 },
        BindingMethod { operation: "proof", argv: &argv, output_contract: None, output_limit_bytes: 32 },
        BindingMethod { operation: "proof", argv: &argv, output_contract: Some(&contracts[0]), output_limit_bytes: 64 },
        BindingMethod { operation: "proof", argv: &argv, output_contract: None, output_limit_bytes: 32 },
        BindingMethod { operation: "编译", argv: &argv, output_contract: Some(&contracts[1]), output_limit_bytes: 64 },
    ];
    let failed = compiler_outputs(Err(43u8), &rows);
    expect(matches!(failed, Err(43)));
    compare_compiler_output(&rows, "proof");
    compare_compiler_output(&rows, "ordinary");
    compare_compiler_output(&rows, "编译");
    compare_compiler_output(&rows, "unknown");
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn required_results_take_precedence_only_in_a_bound_execution_reply() {
    use policy::reply_validation::{needs_receipt, reply_gap, ReplyGap};
    proof { reveal_strlit("blocked"); reveal_strlit("passed");
        assert("blocked"@.len() != "passed"@.len()); }
    expect(needs_receipt("passed", false));
    expect(!needs_receipt("passed", true));
    expect(!needs_receipt("blocked", false));
    for_proof_receipt("verify");
    expect(matches!(reply_gap(None, Some("build"), true, true), Some(ReplyGap::Operations(true))));
    expect(matches!(reply_gap(None, Some("build"), true, false), Some(ReplyGap::Receipt(_, false))));
    expect(matches!(reply_gap(None, None, true, false), Some(ReplyGap::Operations(false))));
    expect(matches!(reply_gap(None, None, false, false), None));
}

fn for_proof_receipt(operation: &str) {
    use policy::reply_validation::{reply_gap, ReplyGap};
    let gap = reply_gap(Some(operation), Some("build"), true, true);
    match gap {
        Some(ReplyGap::Receipt(name, proof_required)) => {
            expect(proof_required);
            expect(policy::verified_text::equal_text(name, operation));
        },
        _ => { expect(false); },
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observed_failure_preserves_the_first_unavailable_result() {
    use policy::{ObservedFailure, finish_observation};
    let wait = finish_observation::<u8, u8, u8>(Err(13), Err(21), Err(32), Err(45));
    expect(matches!(wait, Err(ObservedFailure::Transport(13))));
    let read = finish_observation::<u8, u8, u8>(Ok(false), Err(21), Err(32), Err(45));
    expect(matches!(read, Err(ObservedFailure::Transport(21))));
    let child = finish_observation::<u8, u8, u8>(Ok(false), Ok(82), Err(32), Err(45));
    expect(matches!(child, Err(ObservedFailure::Launcher(82))));
    let write = finish_observation::<u8, u8, u8>(Ok(true), Ok(82), Err(32), Err(45));
    expect(matches!(write, Err(ObservedFailure::Transport(32))));
    let frame = finish_observation::<u8, u8, u8>(Ok(true), Ok(82), Ok(()), Err(45));
    expect(matches!(frame, Err(ObservedFailure::Transport(45))));
    let record = finish_observation::<u8, u8, u8>(Ok(true), Ok(82), Ok(()), Ok(95));
    expect(matches!(record, Ok(95)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observed_frames_separate_the_streams_and_enforce_each_limit() {
    use policy::ObservedFrame::{Stdout,Stderr,Terminal};
    expect(matches!(policy::observed_frame(79u8, 4, 1, 5, 5), Ok(Stdout(5))));
    expect(matches!(policy::observed_frame(69u8, 3, 5, 1, 5), Ok(Stderr(4))));
    expect(matches!(policy::observed_frame(79u8, 0, usize::MAX, 0, usize::MAX), Ok(Stdout(_))));
    expect(policy::observed_frame(79u8, 1, usize::MAX, 0, usize::MAX).is_err());
    expect(policy::observed_frame(69u8, 1, 0, 1, 1).is_err());
    expect(matches!(policy::observed_frame(84u8, 2097152, 0, 0, 0), Ok(Terminal(2097152))));
    expect(policy::observed_frame(84u8, 2097153, 0, 0, 0).is_err());
    expect(policy::observed_frame(0, 0, 0, 0, 0).is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn observed_terminal_binds_both_originals_launcher_child_and_program() {
    proof { reveal_strlit("x"); reveal_strlit(""); assert("x"@.len() != ""@.len()); }
    expect(policy::observed_stream_matches(0, 2, 2, 0, 0, "x", "x", "", ""));
    expect(!policy::observed_stream_matches(1, 2, 2, 0, 0, "x", "x", "", ""));
    expect(!policy::observed_stream_matches(0, 1, 2, 0, 0, "x", "x", "", ""));
    expect(!policy::observed_stream_matches(0, 2, 2, 1, 0, "x", "x", "", ""));
    expect(!policy::observed_stream_matches(0, 2, 2, 0, 0, "", "x", "", ""));
    expect(!policy::observed_stream_matches(0, 2, 2, 0, 0, "x", "x", "", "x"));
    expect(policy::observed_process_matches(Some(123), 123, Some(5), "x", "x"));
    expect(!policy::observed_process_matches(None, 123, Some(5), "x", "x"));
    expect(!policy::observed_process_matches(Some(456), 123, Some(5), "x", "x"));
    expect(!policy::observed_process_matches(Some(123), 123, None, "x", "x"));
    expect(!policy::observed_process_matches(Some(123), 123, Some(0), "x", "x"));
    expect(!policy::observed_process_matches(Some(123), 123, Some(5), "", "x"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn collection_size_accepts_exact_bytes_and_observes_growth_beyond_the_limit() {
    let empty = policy::bounded_original(Vec::new(), 0);
    expect(empty.is_ok());
    let mut bytes = Vec::new(); bytes.push(97u8); bytes.push(0u8);
    let exact = policy::bounded_original(bytes, 2);
    expect(exact.is_ok());
    let returned = exact.unwrap();
    expect(returned.len() == 2); expect(returned[0] == 97); expect(returned[1] == 0);
    let too_long = policy::bounded_original(returned, 1);
    match too_long { Err(size) => expect(size == 2), Ok(_) => expect(false) }
    let zero_cap = policy::original_read_cap(0);
    let last_cap = policy::original_read_cap(u64::MAX - 1);
    let full_cap = policy::original_read_cap(u64::MAX);
    let exact_size = policy::original_size_allowed(u64::MAX, u64::MAX);
    let grew = policy::original_size_allowed(3, 2);
    expect(zero_cap == 1 && last_cap == u64::MAX && full_cap == u64::MAX && exact_size && !grew);
}



#[cfg_attr(not(verus_keep_ghost), test)]
fn unit_originals_bind_profile_path_and_report_bytes() {
    proof { reveal_strlit("A"); reveal_strlit("B"); assert("A"@[0] != "B"@[0]); }
    let exact = policy::profile_matches("A","A"); expect(exact);
    let other = policy::profile_matches("B","A"); expect(!other);
    let missing = policy::report_path_matches(None,"A"); expect(!missing);
    let known = policy::report_path_matches(Some("A"),"A"); expect(known);
    let wrong_path = policy::report_path_matches(Some("A"),"B"); expect(!wrong_path);
    let bytes = [0u8,255u8];
    let accepted = policy::admitted_report(&bytes,"A","A");
    expect(accepted.is_some());
    assert(accepted.unwrap()@ == bytes@);
    let rejected = policy::admitted_report(&bytes,"B","A");
    expect(rejected.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binding_forwards_only_the_compiler_stdout_on_its_original_receipt() {
    proof { reveal_strlit("passed"); reveal_strlit("failed"); assert("passed"@[0] != "failed"@[0]); }
    let bytes = [0xffu8, 0u8, 8u8];
    let absent = policy::binding_original::<u32>("passed", true, None);
    expect(absent.is_none());
    let good = policy::binding_original("passed", true, Some((19u32, &bytes))).unwrap();
    expect(good.0 == 19 && good.1.len() == 3);
    expect(good.1[0] == 0xff && good.1[1] == 0 && good.1[2] == 8);
    let failed = policy::binding_original("failed", true, Some((23u32, &bytes))).unwrap();
    expect(failed.0 == 23 && failed.1.is_empty());
    let ordinary = policy::binding_original("passed", false, Some((29u32, &bytes))).unwrap();
    expect(ordinary.0 == 29 && ordinary.1.is_empty());
    let blocked = policy::binding_original("failed", false, Some((31u32, &bytes))).unwrap();
    expect(blocked.0 == 31 && blocked.1.is_empty());
    let empty: &[u8] = &[];
    let no_bytes = policy::binding_original("passed", true, Some((37u32, empty))).unwrap();
    expect(no_bytes.0 == 37 && no_bytes.1.is_empty());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_action_references_require_the_real_tool_with_no_profile_fallback() {
    let missing = policy::action_tool_allowed(true, false, false);
    let stale_copy = policy::action_tool_allowed(true, false, true);
    let registered_only = policy::action_tool_allowed(true, true, false);
    let both = policy::action_tool_allowed(true, true, true);
    assert(!missing && !stale_copy && registered_only && both);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn standalone_action_references_use_the_explicit_profile_tools() {
    let missing = policy::action_tool_allowed(false, false, false);
    let unrelated_registration = policy::action_tool_allowed(false, true, false);
    let profile_only = policy::action_tool_allowed(false, false, true);
    let both = policy::action_tool_allowed(false, true, true);
    assert(!missing && !unrelated_registration && profile_only && both);
}

fn expect_operation(
    state: &str, error: &Option<String>, exit: Option<i32>,
    expected_state: Status, expected_exit: Option<i32>,
)
    requires expected_state == policy::operation_status(state@),
        expected_exit == policy::operation_exit(state@, exit),
{
    let row = policy::operation_result("é\0operation", state, error, exit);
    assert(row.id@ == "é\0operation"@);
    assert(row.status == expected_state);
    assert(row.exit_code == expected_exit);
    assert(row.cause@ == policy::operation_cause(error));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn operation_state_and_exit_boundaries_use_the_original_fields() {
    proof {
        reveal_strlit("passed"); reveal_strlit("blocked"); reveal_strlit("failed");
        reveal_strlit("error"); reveal_strlit("Passed"); reveal_strlit("passed\0");
        assert("passed"@[0] != "blocked"@[0]);
        assert("passed"@[0] != "failed"@[0]);
        assert("passed"@[0] != "error"@[0]);
        assert("blocked"@.len() != "failed"@.len());
        assert("blocked"@.len() != "error"@.len());
        assert("passed"@[0] != "Passed"@[0]);
        assert("blocked"@.len() != "Passed"@.len());
        assert("passed"@.len() != "passed\0"@.len());
        assert("blocked"@[0] != "passed\0"@[0]);
    }
    let error = Some("original error\0é".to_string());
    let no_error = None;
    expect_operation("passed", &no_error, Some(0), Status::Passed, Some(0));
    expect_operation("passed", &error, Some(-7), Status::Passed, Some(-7));
    expect_operation("passed", &no_error, None, Status::Passed, None);
    expect_operation("blocked", &error, Some(0), Status::Blocked, None);
    expect_operation("blocked", &no_error, Some(7), Status::Blocked, Some(7));
    expect_operation("blocked", &no_error, None, Status::Blocked, None);
    expect_operation("failed", &error, Some(0), Status::Failed, None);
    expect_operation("failed", &error, Some(i32::MIN), Status::Failed, Some(i32::MIN));
    expect_operation("error", &error, Some(i32::MAX), Status::Failed, Some(i32::MAX));
    expect_operation("Passed", &no_error, Some(0), Status::Failed, None);
    expect_operation("passed\0", &no_error, Some(1), Status::Failed, Some(1));
}

/// 任意未知完整名称都失败；错误文本和退出码不反向改写原执行状态。
fn arbitrary_operation_states_are_not_inferred_from_error_or_exit(state: &str, exit: i32)
    requires state@ != "passed"@, state@ != "blocked"@,
{
    let row = policy::operation_result("operation", state, &None, Some(exit));
    assert(row.status == Status::Failed);
    assert(row.exit_code == if exit == 0 { None } else { Some(exit) });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn operation_cause_preserves_empty_unicode_nul_and_default() {
    let original = Some("é\0original error".to_string());
    let row = policy::operation_result("operation", "failed", &original, Some(0));
    assert(row.cause@ == "é\0original error"@);
    let empty = Some("".to_string());
    let empty_cause = policy::operation_result("operation", "passed", &empty, Some(0));
    assert(empty_cause.cause@ == ""@);
    let default_cause = policy::operation_result("operation", "passed", &None, Some(0));
    assert(default_cause.cause@ == "registered command completed with matching receipt"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_binding_failure_cannot_reuse_success_or_fabricate_process_exit() {
    proof { reveal_strlit("failed"); reveal_strlit("passed");
        assert("failed"@[0] != "passed"@[0]); }
    let zero = policy::operation_result_with_sources("proof\0é", "passed", &None, Some(0),
        Err::<(), String>("original source mismatch\0é".to_string()));
    assert(zero.id@ == "proof\0é"@ && zero.status == Status::Failed);
    assert(zero.cause@ == "original source mismatch\0é"@ && zero.exit_code.is_none());
    let nonzero = policy::operation_result_with_sources("proof", "passed", &None, Some(-7),
        Err::<(), String>("".to_string()));
    assert(nonzero.status == Status::Failed && nonzero.exit_code == Some(-7i32));
    assert(nonzero.cause@ == ""@);
    let passed = policy::operation_result_with_sources("proof", "passed", &None, Some(0), Ok(()));
    assert(passed.status == Status::Passed && passed.exit_code == Some(0i32));
    let blocked = policy::operation_result_with_sources("proof", "blocked",
        &Some("original block".to_string()), Some(0), Err::<(), String>("later binding".to_string()));
    proof {
        reveal_strlit("blocked"); reveal_strlit("passed");
        assert("blocked"@[0] != "passed"@[0]);
    }
    assert(blocked.status == Status::Blocked && blocked.exit_code.is_none());
    assert(blocked.cause@ == "original block"@);
    let rows = vec![passed, zero];
    let terminal = policy::response_status(&rows);
    assert(terminal == Status::Failed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_missing_proof_original_keeps_its_specific_reason_and_operation_status() {
    let failure = CheckResult {
        id: "verify.recovery".to_string(), status: Status::Failed,
        cause: "E_VERUS_ARTIFACT: missing original".to_string(), exit_code: None,
    };
    let qualified = policy::coverage_result(failure, true);
    assert(qualified.cause@ == "E_VERUS_COVERAGE: "@ + "E_VERUS_ARTIFACT: missing original"@);
    let blocked = CheckResult {
        id: "verify.recovery".to_string(), status: Status::Blocked,
        cause: "failed prerequisite".to_string(), exit_code: None,
    };
    let qualified = policy::coverage_result(blocked, true);
    assert(qualified.status == Status::Blocked && qualified.exit_code.is_none());
    assert(qualified.cause@ == "E_VERUS_COVERAGE: "@ + "failed prerequisite"@);
    let ordinary = CheckResult {
        id: "business.test".to_string(), status: Status::Blocked,
        cause: "failed prerequisite".to_string(), exit_code: None,
    };
    let qualified = policy::coverage_result(ordinary, false);
    assert(qualified.status == Status::Blocked && qualified.exit_code.is_none());
    assert(qualified.cause@ == "failed prerequisite"@);
    let passed = policy::operation_result("verify.recovery", "passed", &None, Some(0));
    let qualified = policy::coverage_result(passed, true);
    assert(qualified.status == Status::Passed && qualified.exit_code == Some(0i32));
    let unrelated = policy::operation_result("business.test", "failed", &Some("business failure".to_string()), Some(1));
    let unchanged = policy::coverage_result(unrelated, false);
    assert(unchanged.cause@ == "business failure"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn terminal_status_uses_all_original_result_states() {
    let empty: Vec<CheckResult> = Vec::new();
    let empty_status = policy::response_status(&empty);
    assert(empty_status == Status::Passed);
    let first = policy::first_failure(&empty);
    assert(first.is_none());
    let success = vec![
        result("a", Status::NotRequired, Some(-7)),
        result("a", Status::Passed, None),
        result("failed", Status::Passed, Some(1)),
    ];
    let passed = policy::response_status(&success);
    assert(passed == Status::Passed);
    let failed_first = vec![
        result("a", Status::Failed, None), result("a", Status::Passed, Some(0)),
    ];
    let failed = policy::response_status(&failed_first);
    assert(failed == Status::Failed);
    let blocked_last = vec![
        result("a", Status::Passed, Some(0)), result("a", Status::Blocked, None),
    ];
    let blocked = policy::response_status(&blocked_last);
    assert(blocked == Status::Failed);
    let duplicate_failures = vec![
        result("é\0", Status::NotRequired, None),
        result("é\0", Status::Failed, Some(0)),
        result("é\0", Status::Blocked, Some(0)),
    ];
    let duplicates = policy::response_status(&duplicate_failures);
    assert(duplicates == Status::Failed);
    let first = policy::first_failure(&duplicate_failures);
    assert(first == Some(1usize));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn diagnostic_failure_appends_the_real_record_without_replacing_prior_results() {
    let mut rows = vec![result("é\0original", Status::Passed, Some(0))];
    let status = policy::diagnostic_failure(&mut rows);
    assert(status == Status::Failed);
    assert(rows.len() == 2);
    assert(rows[0].id@ == "é\0original"@);
    assert(rows[0].status == Status::Passed);
    assert(rows[0].cause@ == "original report"@);
    assert(rows[0].exit_code == Some(0i32));
    assert(rows[1].id@ == "ci.execution-diagnostics"@);
    assert(rows[1].status == Status::Failed);
    assert(rows[1].exit_code.is_none());
    assert(rows[1].cause@ == "execution diagnostic publication failed; original errors retained"@);
    let mut empty: Vec<CheckResult> = Vec::new();
    let from_empty = policy::diagnostic_failure(&mut empty);
    assert(from_empty == Status::Failed && empty.len() == 1);
    let again = policy::diagnostic_failure(&mut empty);
    assert(again == Status::Failed && empty.len() == 2);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_failure_produces_its_own_failed_terminal_state_and_original_cause() {
    let (status, rows) = policy::inventory_failure("é\0original inventory error".to_string());
    assert(status == Status::Failed);
    assert(rows.len() == 1);
    assert(rows[0].id@ == "ci.inventory"@);
    assert(rows[0].status == Status::Failed);
    assert(rows[0].cause@ == "é\0original inventory error"@);
    assert(rows[0].exit_code.is_none());
}

fn result(id: &str, status: Status, exit_code: Option<i32>) -> (out: CheckResult)
    ensures out.id@ == id@, out.status == status, out.exit_code == exit_code,
        out.cause@ == "original report"@,
{
    CheckResult {
        id: id.to_string(), status, cause: "original report".to_string(), exit_code,
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unit_outcome_uses_the_complete_row_set_and_actual_exit() {
    let mut rows = Vec::new();
    expect(!policy::unit_outcome_matches(&rows, Status::NotRequired, 0));
    rows.push(result("skipped", Status::NotRequired, None));
    assert(rows[0].status == Status::NotRequired);
    expect(policy::unit_outcome_matches(&rows, Status::NotRequired, 0));
    expect(!policy::unit_outcome_matches(&rows, Status::Passed, 0));
    rows.push(result("build", Status::Passed, Some(0)));
    assert(rows[1].status == Status::Passed);
    expect(policy::unit_outcome_matches(&rows, Status::Passed, 0));
    expect(!policy::unit_outcome_matches(&rows, Status::NotRequired, 0));
    expect(!policy::unit_outcome_matches(&rows, Status::Passed, -1));
    rows.push(result("preparation", Status::Blocked, None));
    assert(rows[2].status == Status::Blocked);
    expect(policy::unit_outcome_matches(&rows, Status::Blocked, -1));
    expect(policy::unit_outcome_matches(&rows, Status::Failed, 2));
    expect(!policy::unit_outcome_matches(&rows, Status::Passed, 0));
    expect(!policy::unit_outcome_matches(&rows, Status::Blocked, 0));
    rows.push(result("test", Status::Failed, Some(101)));
    assert(rows[3].status == Status::Failed);
    expect(policy::unit_outcome_matches(&rows, Status::Failed, 101));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn operation_success_requires_all_fields_on_one_exact_result() {
    proof {
        reveal_strlit("build"); reveal_strlit("other");
        assert("build"@[0] != "other"@[0]);
    }
    let values = vec![result("build", Status::Passed, Some(0))];
    let accepted = policy::has_result(&values, "build", &Status::Passed, Some(0));
    assert(accepted);
    let wrong_id = policy::has_result(&values, "other", &Status::Passed, Some(0));
    assert(!wrong_id);
    let wrong_status = policy::has_result(&values, "build", &Status::Failed, Some(0));
    assert(!wrong_status);
    let wrong_exit = policy::has_result(&values, "build", &Status::Passed, Some(7));
    assert(!wrong_exit);
    let absent_exit = policy::has_result(&values, "build", &Status::Passed, None);
    assert(!absent_exit);
    let empty = Vec::new();
    let missing = policy::has_result(&empty, "build", &Status::Passed, Some(0));
    assert(!missing);
    let split = vec![
        result("build", Status::Failed, Some(0)),
        result("other", Status::Passed, Some(0)),
        result("build", Status::Passed, Some(7)),
        result("build", Status::NotRequired, Some(0)),
        result("build", Status::Blocked, Some(0)),
    ];
    let cannot_combine = policy::has_result(&split, "build", &Status::Passed, Some(0));
    assert(!cannot_combine);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_and_unrequired_states_preserve_their_distinct_exit_contracts() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("test");
        assert("ci.inventory"@.len() != "test"@.len());
    }
    let unrequired = vec!["test"];
    let valid = vec![
        result("test", Status::NotRequired, Some(7)),
        result("ci.inventory", Status::Passed, None),
    ];
    let accepted = policy::matches_result_states(&valid, &unrequired);
    assert(accepted);
    let no_inventory = vec![result("test", Status::NotRequired, None)];
    let missing = policy::matches_result_states(&no_inventory, &unrequired);
    assert(!missing);
    let exited_inventory = vec![result("ci.inventory", Status::Passed, Some(0))];
    let forbidden_exit = policy::matches_result_states(&exited_inventory, &unrequired);
    assert(!forbidden_exit);
    let failed_inventory = vec![result("ci.inventory", Status::Failed, None)];
    let failed = policy::matches_result_states(&failed_inventory, &unrequired);
    assert(!failed);
    let wrong_unrequired = vec![
        result("ci.inventory", Status::Passed, None),
        result("test", Status::Passed, None),
    ];
    let wrong_status = policy::matches_result_states(&wrong_unrequired, &unrequired);
    assert(!wrong_status);
    let empty = Vec::new();
    let absent = policy::matches_result_states(&empty, &unrequired);
    assert(!absent);
    // ID 的完整覆盖由独立集合合同承担；状态合同不补造缺少的结果。
    let marker_only = vec![result("ci.inventory", Status::Passed, None)];
    let states_only = policy::matches_result_states(&marker_only, &unrequired);
    assert(states_only);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn borrowed_registered_ids_preserve_order_duplicates_unicode_nul_and_empty() {
    let values = vec!["é".to_string(), "a\0b".to_string(), "é".to_string()];
    let texts = policy::borrow_texts(&values);
    assert(texts.len() == 3);
    assert(texts[0]@ == "é"@);
    assert(texts[1]@ == "a\0b"@);
    assert(texts[2]@ == "é"@);
    let empty = Vec::new();
    let no_texts = policy::borrow_texts(&empty);
    assert(no_texts.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_report_id_projection_preserves_order_duplicates_unicode_and_nul() {
    let values = vec![
        result("é", Status::Blocked, Some(-7)),
        result("a\0b", Status::Passed, None),
        result("é", Status::NotRequired, Some(0)),
    ];
    let ids = policy::result_ids(&values);
    assert(ids.len() == 3);
    assert(ids[0]@ == "é"@);
    assert(ids[1]@ == "a\0b"@);
    assert(ids[2]@ == "é"@);
    let empty = Vec::new();
    let no_ids = policy::result_ids(&empty);
    assert(no_ids.len() == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn actual_report_coverage_uses_the_original_record_ids() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test"@.len());
        assert("build"@.len() != "test"@.len());
    }
    let operation_names = vec!["build".to_string()];
    let unrequired_names = vec!["test".to_string()];
    let operations = policy::borrow_texts(&operation_names);
    let unrequired = policy::borrow_texts(&unrequired_names);
    assert(policy::text_values(operations@) =~= seq!["build"@]);
    assert(policy::text_values(unrequired@) =~= seq!["test"@]);
    assert(policy::expected_result_ids(operations@, unrequired@)
        =~= seq!["ci.inventory"@, "build"@, "test"@]);
    let values = vec![
        result("test", Status::NotRequired, None),
        result("ci.inventory", Status::Passed, None),
        result("build", Status::Passed, Some(0)),
    ];
    let accepted = policy::matches_report_result_ids(&values, &operations, &unrequired);
    assert(accepted);
    let missing_operation = vec![
        result("test", Status::NotRequired, None),
        result("ci.inventory", Status::Passed, None),
    ];
    let missing = policy::matches_report_result_ids(&missing_operation, &operations, &unrequired);
    assert(!missing);
    let undeclared = policy::matches_report_result_ids(&values, &operations, &Vec::new());
    assert(!undeclared);
    let empty = Vec::new();
    let absent = policy::matches_report_result_ids(&empty, &operations, &unrequired);
    assert(!absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_permutations_and_rejects_missing_or_extra_ids() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test");
        reveal_strlit("other");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test"@.len());
        assert("build"@[0] != "other"@[0]);
        assert("build"@.len() != "test"@.len());
        assert("test"@.len() != "other"@.len());
        assert("ci.inventory"@.len() != "other"@.len());
    }
    let expected = vec!["ci.inventory", "build", "test"];
    let reordered = vec!["test", "ci.inventory", "build"];
    let accepted = policy::matches_text_membership(&reordered, &expected);
    assert(accepted);
    let missing = vec!["ci.inventory", "build"];
    let absent = policy::matches_text_membership(&missing, &expected);
    assert(!absent);
    let extra = vec!["ci.inventory", "build", "test", "other"];
    let unexpected = policy::matches_text_membership(&extra, &expected);
    assert(!unexpected);
    let replaced = vec!["ci.inventory", "build", "other"];
    let changed = policy::matches_text_membership(&replaced, &expected);
    assert(!changed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_keeps_empty_and_duplicate_set_semantics() {
    proof { reveal_strlit("test"); }
    let empty: Vec<&str> = Vec::new();
    let same_empty = policy::matches_text_membership(&empty, &empty);
    assert(same_empty);
    let one = vec!["test"];
    let missing = policy::matches_text_membership(&empty, &one);
    assert(!missing);
    let extra = policy::matches_text_membership(&one, &empty);
    assert(!extra);
    let duplicates = vec!["test", "test"];
    let same_set = policy::matches_text_membership(&duplicates, &one);
    assert(same_set);
    let reverse = policy::matches_text_membership(&one, &duplicates);
    assert(reverse);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_exact_unicode_and_nul_identity() {
    proof {
        reveal_strlit("é"); reveal_strlit("é"); reveal_strlit("a\0b");
        reveal_strlit("a");
        assert("é"@.len() != "é"@.len());
        assert("a\0b"@.len() != "a"@.len());
    }
    let composed = vec!["é"];
    let decomposed = vec!["é"];
    let unicode_changed = policy::matches_text_membership(&composed, &decomposed);
    assert(!unicode_changed);
    let nul = vec!["a\0b"];
    let prefix = vec!["a"];
    let truncated = policy::matches_text_membership(&nul, &prefix);
    assert(!truncated);
    let exact = policy::matches_text_membership(&nul, &nul);
    assert(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_inventory_requires_the_marker_and_both_declared_lists() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test:other");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test:other"@.len());
        assert("build"@.len() != "test:other"@.len());
    }
    let operations = vec!["build"];
    let not_required = vec!["test:other"];
    let reordered = vec!["test:other", "ci.inventory", "build"];
    assert(policy::text_values(operations@) =~= seq!["build"@]);
    assert(policy::text_values(not_required@) =~= seq!["test:other"@]);
    assert(policy::expected_result_ids(operations@, not_required@)
        =~= seq!["ci.inventory"@, "build"@, "test:other"@]);
    let accepted = policy::matches_result_ids(&reordered, &operations, &not_required);
    assert(accepted);
    let no_inventory = vec!["build", "test:other"];
    let missing_marker = policy::matches_result_ids(&no_inventory, &operations, &not_required);
    assert(!missing_marker);
    let no_operation = vec!["ci.inventory", "test:other"];
    let missing_operation = policy::matches_result_ids(&no_operation, &operations, &not_required);
    assert(!missing_operation);
    let no_unrequired = vec!["ci.inventory", "build"];
    let missing_unrequired = policy::matches_result_ids(&no_unrequired, &operations, &not_required);
    assert(!missing_unrequired);
    let undeclared = policy::matches_result_ids(&reordered, &operations, &Vec::new());
    assert(!undeclared);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_inventory_preserves_empty_lists_and_exact_declared_names() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("é"); reveal_strlit("a\0b");
        assert("ci.inventory"@.len() != "é"@.len());
        assert("ci.inventory"@.len() != "a\0b"@.len());
        assert("é"@.len() != "a\0b"@.len());
    }
    let empty: Vec<&str> = Vec::new();
    let marker = vec!["ci.inventory"];
    assert(marker[0]@ == "ci.inventory"@);
    assert(policy::expected_result_ids(empty@, empty@) =~= seq!["ci.inventory"@]);
    assert(policy::expected_result_ids(empty@, empty@)[0] == "ci.inventory"@);
    let inventory_only = policy::matches_result_ids(&marker, &empty, &empty);
    assert(inventory_only);
    let no_marker = policy::matches_result_ids(&empty, &empty, &empty);
    assert(!no_marker);
    let operations = vec!["é", "é"];
    let not_required = vec!["a\0b", "é"];
    let actual = vec!["a\0b", "ci.inventory", "é"];
    assert(policy::text_values(operations@) =~= seq!["é"@, "é"@]);
    assert(policy::text_values(not_required@) =~= seq!["a\0b"@, "é"@]);
    assert(policy::expected_result_ids(operations@, not_required@)
        =~= seq!["ci.inventory"@, "é"@, "é"@, "a\0b"@, "é"@]);
    let exact_set = policy::matches_result_ids(&actual, &operations, &not_required);
    assert(exact_set);
}

}

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn routing_requires_all_blockers_to_be_cleared() {
    let none = Vec::<String>::new();
    let blocked = vec!["missing required input".to_string()];
    expect(policy::can_validate_selected_plan(&none));
    expect(!policy::can_validate_selected_plan(&blocked));
}

}

#[cfg(not(verus_keep_ghost))]
#[allow(dead_code, unexpected_cfgs)]
#[path = "../../judge-ci/src/main.rs"]
mod cli;

#[cfg(not(verus_keep_ghost))]
#[test]
fn diagnostics_preserve_request_status_with_open_and_closed_writers() {
    use std::io::{self, Write};
    struct Closed;
    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut bytes = Vec::new();
    for code in ["E_REQUEST", "E_REQUIREMENTS"] {
        bytes.clear();
        assert_eq!(
            cli::finish(Err("invalid"), |msg| cli::write_diagnostic(
                &mut bytes, code, msg
            )),
            2
        );
        assert_eq!(bytes, format!("{code}: invalid\n").as_bytes());
        let mut attempted = false;
        assert_eq!(
            cli::finish(Err("invalid"), |msg| {
                attempted = true;
                cli::write_diagnostic(&mut Closed, code, msg)
            }),
            2
        );
        assert!(attempted);
    }
    for code in [0u8, 1] {
        assert_eq!(
            cli::finish(Ok(code), |_: &str| panic!("unexpected diagnostic")),
            code
        );
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn collected_rows_report_the_first_protocol_violation() {
    use chrono_judge_ci::unit_response;
    use serde_json::json;
    let row = json!({"id":"编译", "status":"passed", "cause":"ok", "exit_code":0});
    let original = json!({"protocol":"chrono-ci-judge/v2", "request_id":"host", "status":"passed",
                         "results":[row], "evidence":{"unit":"build"}});
    for (field, value, error) in [
        ("id", json!(""), "invalid/duplicate judge result"),
        ("cause", json!(""), "invalid/duplicate judge result"),
        ("exit_code", json!(7), "passed result has failing exit"),
        (
            "status",
            json!("failed"),
            "failed result has successful exit",
        ),
        (
            "status",
            json!("not-required"),
            "not-required result must have null exit_code",
        ),
    ] {
        let mut report = original.clone();
        report["results"][0][field] = value;
        assert_eq!(unit_response(&report, "host", 0).err().unwrap(), error);
    }
    let mut duplicate = original.clone();
    let mut invalid = row;
    invalid["exit_code"] = json!(1);
    duplicate["results"].as_array_mut().unwrap().push(invalid);
    assert_eq!(
        unit_response(&duplicate, "host", 0).err().unwrap(),
        "invalid/duplicate judge result"
    );
    for (field, value) in [
        ("protocol", json!("chrono-ci-judge/v1")),
        ("request_id", json!("Host")),
        ("results", json!([])),
        ("evidence", json!({})),
        ("evidence", json!([])),
    ] {
        let mut report = duplicate.clone();
        report[field] = value;
        assert_eq!(
            unit_response(&report, "host", 0).err().unwrap(),
            "empty or mismatched judge response"
        );
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn public_queries_preserve_the_selected_tool_stream() {
    use chrono_judge_ci::{impact_query::Query, project_pairs};
    use serde_json::{Value, json};
    use std::collections::BTreeSet;
    use std::path::Path;
    let config = json!({"protocol":{"timeout_seconds":1, "stdout_limit_bytes":64}});
    let query = Query::full(Path::new("."), &config).unwrap();
    for (code, out, err) in [
        (0, b"null".as_slice(), b"ignored".as_slice()),
        (17, b"null", b"failure\n"),
    ] {
        let run = || Ok((code, out.to_vec(), err.to_vec()));
        let found = query.read_with::<Value>("registered", "approach", &json!({}), |_, _| run());
        let paired = project_pairs::check(
            &json!({}),
            &BTreeSet::new(),
            None,
            String::new(),
            1,
            64,
            |_, _| run(),
        );
        if code == 0 {
            assert_eq!(found.unwrap(), Value::Null);
            assert_eq!(paired.unwrap_err(), "unexpected check-pairs-v1 response");
        } else {
            assert_eq!(found.unwrap_err(), "registered approach exited 17: failure");
            assert_eq!(paired.unwrap_err(), "failure");
        }
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn full_oid_accepts_only_nonzero_lower_hex_at_supported_lengths() {
    use chrono_judge_ci::full_oid;
    for length in [40, 64] {
        assert!(full_oid(&"a".repeat(length)).is_ok());
        assert!(full_oid(&"0".repeat(length)).is_err());
        assert!(full_oid(&format!("{}F", "1".repeat(length - 1))).is_err());
        assert!(full_oid(&"a".repeat(length - 1)).is_err());
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn unit_environment_identities_verify_the_producers_program_offline() {
    use chrono_judge_ci::{codec::digest, unit_originals};
    use serde_json::{Value, json};
    use std::path::Path;

    let config = serde_json::from_str(include_str!("fixtures/unit-profile.json")).unwrap();
    let root = Path::new("/producer");
    let executable = "/producer/tools/judge";
    let candidates = json!([
        digest(&"/other/judge").unwrap(),
        digest(&executable).unwrap()
    ]);
    let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let original_env = json!({"PATH": "/other:tools"});
    let identities =
        json!({"PATH":"9f4b30d25acc829a89a4e3f5c3ac3d06fff4223cbeb6fe9d3c6006dfacb7ec1b"});
    let mut record = json!({
        "argv":[executable],"cwd":root,"environment":identities,
        "environment_digest":digest(&original_env).unwrap(),"environment_representation":"sha256",
        "unit_environment_digest":digest(&json!([digest(&original_env).unwrap(),identities,candidates])).unwrap(),"program_candidates_sha256":candidates,
        "stdin_sha256":empty,"stdout_bytes":[],"stderr_bytes":[],"stdout_sha256":empty,
        "stderr_sha256":empty,"failure":null,"exit_code":0,"stdout":"","stderr":"",
        "executable":executable,"sha256":"a".repeat(64)
    });
    let validate = |judge: &Value, prepared: bool| {
        let raw = json!({"schema":"chrono-check-report/v1","judge":judge,
            "request":{"scope":{"kind":"unit","unit":"unit-a"},"protocol":"chrono-ci-judge/v2",
            "request_id":"unit","host_root":root,"config_path":"profile","config_sha256":"b".repeat(64),
            "candidate":"c".repeat(40),"initial":true}});
        let mut originals = unit_originals(
            root,
            &config,
            "unit-a",
            &serde_json::to_vec(&raw).unwrap(),
            None,
            |_, _| panic!("inline original"),
        )
        .unwrap();
        if prepared {
            originals.request.observations = json!({"preparation":{}});
        }
        originals.validate_producer(&config)
    };
    validate(&record, false).unwrap();
    assert!(validate(&record, true).is_err());
    for (field, bad_value) in [
        ("environment_representation", json!("unknown")),
        ("unit_environment_digest", json!("1".repeat(64))),
        ("environment_digest", json!("2".repeat(64))),
        (
            "program_candidates_sha256",
            json!([digest(&"/other/judge").unwrap()]),
        ),
        ("argv", json!(["/other/judge"])),
        ("environment", json!({"PATH":"plaintext"})),
        ("stdout", json!("different")),
    ] {
        let mut bad = record.clone();
        bad[field] = bad_value;
        assert!(validate(&bad, false).is_err(), "{field}");
    }
    let map = record.as_object_mut().unwrap();
    for field in [
        "environment_representation",
        "unit_environment_digest",
        "program_candidates_sha256",
    ] {
        map.remove(field);
    }
    record["environment"] = original_env;
    validate(&record, false).unwrap();
    validate(&record, true).unwrap();
}
