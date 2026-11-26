//! 现役 CI 的执行结果转换、终态与离线结果集合合同。
use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

#[path = "result_types.rs"]
pub mod result_types;
use result_types::{CheckResult, Status};

#[cfg(verus_keep_ghost)]
#[path = "project_execution.rs"]
pub mod reply_validation;
#[cfg(not(verus_keep_ghost))]
pub mod reply_validation {
    pub use crate::project_execution::{ReplyGap, needs_receipt, reply_gap};
}

#[cfg(verus_keep_ghost)]
#[path = "files.rs"]
pub mod file_counts;
#[cfg(not(verus_keep_ghost))]
pub mod file_counts {
    pub use crate::files::stream_byte_count;
}

#[cfg(verus_keep_ghost)]
#[path = "source_binding.rs"]
pub mod output_contracts;
#[cfg(not(verus_keep_ghost))]
pub mod output_contracts {
    pub use crate::source_binding::{BindingMethod, compiler_outputs};
}

verus! {

/// A data frame carries the permitted new stream length; a terminal carries its JSON length.
pub const STDOUT_TAG: u8 = 79; // ASCII O
pub const STDERR_TAG: u8 = 69; // ASCII E
pub const TERMINAL_TAG: u8 = 84; // ASCII T

pub enum ObservedFrame { Stdout(usize), Stderr(usize), Terminal(usize) }

/// Keep launcher diagnostics as their original bytes until the caller needs to display them.
pub enum ObservedFailure<E, D> { Transport(E), Launcher(D) }

pub fn finish_observation<T, E, D>(
    wait: Result<bool, E>, diagnostics: Result<D, E>, written: Result<(), E>, received: Result<T, E>,
) -> (result: Result<T, ObservedFailure<E, D>>)
    ensures result == match wait {
        Err(e) => Err(ObservedFailure::Transport(e)),
        Ok(success) => match diagnostics {
            Err(e) => Err(ObservedFailure::Transport(e)),
            Ok(bytes) => if !success { Err(ObservedFailure::Launcher(bytes)) }
                else { match written {
                    Err(e) => Err(ObservedFailure::Transport(e)),
                    Ok(()) => match received { Err(e) => Err(ObservedFailure::Transport(e)), Ok(p) => Ok(p) },
                } },
        },
    },
{
    let success = match wait { Err(e) => return Err(ObservedFailure::Transport(e)), Ok(s) => s };
    let bytes = match diagnostics { Err(e) => return Err(ObservedFailure::Transport(e)), Ok(d) => d };
    if !success { return Err(ObservedFailure::Launcher(bytes)); }
    if let Err(e) = written { return Err(ObservedFailure::Transport(e)); }
    match received { Err(e) => Err(ObservedFailure::Transport(e)), Ok(p) => Ok(p) }
}

pub fn observed_frame(tag: u8, length: usize, stdout: usize, stderr: usize, limit: usize)
    -> (result: Result<ObservedFrame, &'static str>)
    requires stdout <= limit, stderr <= limit,
    ensures match result {
        Ok(ObservedFrame::Stdout(end)) => tag == STDOUT_TAG && end as int == stdout + length && end <= limit,
        Ok(ObservedFrame::Stderr(end)) => tag == STDERR_TAG && end as int == stderr + length && end <= limit,
        Ok(ObservedFrame::Terminal(size)) => tag == TERMINAL_TAG && size == length && length <= 2097152,
        Err(e) => if tag == STDOUT_TAG || tag == STDERR_TAG {
            e@ == "observed output exceeds command bound"@
                && (if tag == STDOUT_TAG { stdout } else { stderr }) as int + length > limit
        } else if tag == TERMINAL_TAG {
            e@ == "oversize observed terminal record"@ && length > 2097152
        } else { e@ == "unknown observed command frame"@ }
    },
{
    match tag {
        STDOUT_TAG if length <= limit - stdout => Ok(ObservedFrame::Stdout(stdout + length)),
        STDERR_TAG if length <= limit - stderr => Ok(ObservedFrame::Stderr(stderr + length)),
        STDOUT_TAG | STDERR_TAG => Err("observed output exceeds command bound"),
        TERMINAL_TAG if length <= 2 * 1024 * 1024 => Ok(ObservedFrame::Terminal(length)),
        TERMINAL_TAG => Err("oversize observed terminal record"),
        _ => Err("unknown observed command frame"),
    }
}

/// The first byte after a terminal record must be EOF. The lengths and SHA-256 strings
/// observed at the reader must agree with those declared by the producer.
pub fn observed_stream_matches(
    trailing: usize, declared_out: usize, actual_out: usize, declared_err: usize, actual_err: usize,
    declared_out_sha: &str, actual_out_sha: &str, declared_err_sha: &str, actual_err_sha: &str,
) -> (ok: bool)
    ensures ok == (trailing == 0 && declared_out == actual_out && declared_err == actual_err
        && declared_out_sha@ == actual_out_sha@ && declared_err_sha@ == actual_err_sha@),
{
    trailing == 0 && declared_out == actual_out && declared_err == actual_err
        && <&str>::exec_eq(declared_out_sha, actual_out_sha)
        && <&str>::exec_eq(declared_err_sha, actual_err_sha)
}

pub fn observed_process_matches(launcher: Option<u32>, expected: u32, child: Option<u32>,
    digest: &str, expected_digest: &str) -> (ok: bool)
    ensures ok == (launcher == Some(expected) && child.is_some() && child != Some(0u32)
        && digest@ == expected_digest@),
{
    matches!(launcher, Some(pid) if pid == expected) && matches!(child, Some(pid) if pid != 0)
        && <&str>::exec_eq(digest, expected_digest)
}

/// Read one extra byte to detect growth after metadata observation.
pub fn original_read_cap(limit: u64) -> (cap: u64)
    ensures cap == if limit < u64::MAX { (limit + 1) as u64 } else { u64::MAX },
{
    if limit == u64::MAX { limit } else { limit + 1 }
}

pub fn original_size_allowed(length: u64, limit: u64) -> (allowed: bool)
    ensures allowed == (length <= limit),
{ length <= limit }

/// Return the unchanged bytes, or the length actually read when the input exceeded the limit.
pub fn bounded_original(bytes: Vec<u8>, limit: u64) -> (result: Result<Vec<u8>, usize>)
    ensures match result {
        Ok(data) => bytes@.len() <= limit && data@ == bytes@,
        Err(size) => bytes@.len() > limit && size == bytes@.len(),
    },
{
    let size = bytes.len();
    if (size as u64) <= limit { Ok(bytes) } else { Err(size) }
}

/// Identifies the exact profile before its bytes can be decoded.
pub fn profile_matches(observed: &str, declared: &str) -> (ok: bool)
    ensures ok == (observed@ == declared@),
{
    <&str>::exec_eq(observed, declared)
}

/// The report must name the path registered for this exact unit.
pub fn report_path_matches(registered: Option<&str>, declared: &str) -> (ok: bool)
    ensures ok == match registered { Some(path) => path@ == declared@, None => false },
{
    match registered {
        Some(path) => <&str>::exec_eq(path, declared),
        None => false,
    }
}

/// Only deliver bytes to the collector after their observed identity matches.
pub fn admitted_report<'a>(bytes: &'a [u8], observed: &str, declared: &str)
    -> (out: Option<&'a [u8]>)
    ensures match out {
        Some(data) => observed@ == declared@ && data == bytes,
        None => observed@ != declared@,
    },
{
    if <&str>::exec_eq(observed, declared) { Some(bytes) } else { None }
}

/// Preserve the receipt and forward its original stdout only when a passed
/// operation declares compiler output. Other present receipts carry no stdout.
pub fn binding_original<'a, T>(
    state: &str, compiler_output: bool, original: Option<(T, &'a [u8])>,
) -> (out: Option<(T, &'a [u8])>)
    ensures match (original, out) {
        (None, None) => true,
        (Some((receipt, bytes)), Some((identity, sent))) => identity == receipt
            && if compiler_output && state@ == "passed"@ { sent == bytes } else { sent.len() == 0 },
        _ => false,
    },
{
    match original {
        None => None,
        Some((receipt, raw)) => {
            let stdout: &[u8] = if compiler_output {
                match admitted_report(raw, state, "passed") { Some(bytes) => bytes, None => &[] }
            } else { &[] };
            Some((receipt, stdout))
        },
    }
}

/// A plan with missing inputs stays blocked before any route query.
pub fn can_validate_selected_plan(blocked: &[String]) -> (ready: bool)
    ensures ready == (blocked.len() == 0),
{
    blocked.is_empty()
}

/// Registered hosts admit action references only from their full configuration.
pub open spec fn authoritative_tool_present(
    registered: bool, registration_contains: bool, profile_contains: bool,
) -> bool {
    if registered { registration_contains } else { profile_contains }
}

pub fn action_tool_allowed(
    registered: bool, registration_contains: bool, profile_contains: bool,
) -> (allowed: bool)
    ensures allowed == authoritative_tool_present(
        registered, registration_contains, profile_contains,
    ),
{
    if registered { registration_contains } else { profile_contains }
}

pub open spec fn operation_status(state: Seq<char>) -> Status {
    if state == "passed"@ { Status::Passed }
    else if state == "blocked"@ { Status::Blocked }
    else { Status::Failed }
}

pub open spec fn operation_exit(state: Seq<char>, observed: Option<i32>) -> Option<i32> {
    match observed {
        Some(code) => if state == "passed"@ || code != 0 { Some(code) } else { None },
        None => None,
    }
}

pub open spec fn operation_cause(error: &Option<String>) -> Seq<char> {
    match error {
        Some(cause) => cause@,
        None => "registered command completed with matching receipt"@,
    }
}

/// 直接生成现役 CheckResult；失败回执中的零退出只留在原回执，不冒充失败退出。
pub fn operation_result(
    operation: &str, state: &str, error: &Option<String>, observed: Option<i32>,
) -> (row: CheckResult)
    ensures row.id@ == operation@,
        row.status == operation_status(state@),
        row.cause@ == operation_cause(error),
        row.exit_code == operation_exit(state@, observed),
{
    let passed = <&str>::exec_eq(state, "passed");
    let status = if passed { Status::Passed }
        else if <&str>::exec_eq(state, "blocked") { Status::Blocked }
        else { Status::Failed };
    let cause = match error {
        Some(cause) => cause.clone(),
        None => "registered command completed with matching receipt".to_string(),
    };
    let exit_code = match observed {
        Some(code) => if passed || code != 0 { Some(code) } else { None },
        None => None,
    };
    CheckResult { id: operation.to_string(), status, cause, exit_code }
}

pub open spec fn source_binding_failed<T>(state: Seq<char>, binding: &Result<T, String>) -> bool {
    state == "passed"@ && binding.is_err()
}

/// 源码绑定失败不能沿用原 passed；原进程及其真实零退出仍保留在原回执。
pub fn operation_result_with_sources<T>(
    operation: &str, state: &str, error: &Option<String>, observed: Option<i32>,
    binding: Result<T, String>,
) -> (row: CheckResult)
    ensures row.id@ == operation@,
        row.status == if source_binding_failed(state@, &binding) { Status::Failed }
            else { operation_status(state@) },
        row.cause@ == match binding {
            Err(cause) => if state@ == "passed"@ { cause@ } else { operation_cause(error) },
            Ok(_) => operation_cause(error),
        },
        row.exit_code == if source_binding_failed(state@, &binding) {
            operation_exit("failed"@, observed)
        } else { operation_exit(state@, observed) },
{
    match binding {
        Err(cause) => {
            if <&str>::exec_eq(state, "passed") {
                proof {
                    reveal_strlit("failed"); reveal_strlit("passed"); reveal_strlit("blocked");
                    assert("failed"@[0] != "passed"@[0]);
                    assert("failed"@.len() != "blocked"@.len());
                }
                operation_result(operation, "failed", &Some(cause), observed)
            } else { operation_result(operation, state, error, observed) }
        },
        Ok(_) => operation_result(operation, state, error, observed),
    }
}

pub fn coverage_cause(cause: String, required: bool) -> (result: String)
    ensures result@ == if required { "E_VERUS_COVERAGE: "@ + cause@ } else { cause@ },
{
    if required { "E_VERUS_COVERAGE: ".to_string().concat(cause.as_str()) } else { cause }
}

/// A required compiler operation that failed or was blocked cannot establish source coverage.
pub fn coverage_result(mut row: CheckResult, required_proof: bool) -> (result: CheckResult)
    ensures result.id@ == row.id@, result.status == row.status, result.exit_code == row.exit_code,
        result.cause@ == if required_proof && (row.status == Status::Failed || row.status == Status::Blocked) {
            "E_VERUS_COVERAGE: "@ + row.cause@
        } else { row.cause@ },
{
    row.cause = coverage_cause(row.cause, required_proof && matches!(row.status, Status::Failed | Status::Blocked));
    row
}

pub open spec fn report_failure(rows: Seq<CheckResult>) -> bool {
    exists|i: int| 0 <= i < rows.len()
        && ((#[trigger] rows[i]).status == Status::Failed || rows[i].status == Status::Blocked)
}

pub open spec fn report_status(rows: Seq<CheckResult>) -> Status {
    if report_failure(rows) { Status::Failed } else { Status::Passed }
}

/// 本地／单元和离线汇总使用同一真实结果列表，失败或阻断均不能生成通过。
pub fn first_failure(rows: &Vec<CheckResult>) -> (found: Option<usize>)
    ensures found.is_some() == report_failure(rows@),
        match found {
            Some(i) => i < rows.len() && (rows[i as int].status == Status::Failed
                    || rows[i as int].status == Status::Blocked)
                && forall|j: int| 0 <= j < i ==> (#[trigger] rows[j]).status != Status::Failed
                    && rows[j].status != Status::Blocked,
            None => true,
        },
{
    let mut i = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] rows[j]).status != Status::Failed
                && rows[j].status != Status::Blocked,
        decreases rows.len() - i,
    {
        if matches!(rows[i].status, Status::Failed | Status::Blocked) {
            assert(report_failure(rows@));
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn response_status(rows: &Vec<CheckResult>) -> (status: Status)
    ensures status == report_status(rows@),
{
    if first_failure(rows).is_some() { Status::Failed } else { Status::Passed }
}

pub open spec fn all_not_required(rows: Seq<CheckResult>) -> bool {
    forall|i: int| 0 <= i < rows.len() ==> (#[trigger] rows[i]).status == Status::NotRequired
}

fn only_not_required(rows: &Vec<CheckResult>) -> (all: bool)
    ensures all == all_not_required(rows@),
{
    let mut index = 0;
    while index < rows.len()
        invariant
            0 <= index <= rows.len(),
            forall|j: int| 0 <= j < index ==> (#[trigger] rows[j]).status == Status::NotRequired,
        decreases rows.len() - index,
    {
        if !matches!(rows[index].status, Status::NotRequired) {
            assert(!all_not_required(rows@));
            return false;
        }
        index += 1;
    }
    true
}

/// The collection reads the recorded status and the exit of the same unit process.
pub fn unit_outcome_matches(rows: &Vec<CheckResult>, reported: Status, exit: i32) -> (ok: bool)
    ensures ok == (rows.len() > 0
        && report_failure(rows@) == (reported == Status::Failed || reported == Status::Blocked)
        && report_failure(rows@) == (exit != 0)
        && all_not_required(rows@) == (reported == Status::NotRequired)),
{
    if rows.is_empty() { return false; }
    let failed = first_failure(rows).is_some();
    failed == matches!(reported, Status::Failed | Status::Blocked)
        && failed == (exit != 0)
        && only_not_required(rows) == matches!(reported, Status::NotRequired)
}

/// 原异常产生唯一库存失败记录；状态与该实际记录同源。
pub fn inventory_failure(cause: String) -> (failure: (Status, Vec<CheckResult>))
    ensures failure.0 == report_status(failure.1@), failure.0 == Status::Failed,
        failure.1.len() == 1, failure.1[0].id@ == "ci.inventory"@,
        failure.1[0].status == Status::Failed, failure.1[0].cause@ == cause@,
        failure.1[0].exit_code.is_none(),
{
    let rows = vec![CheckResult {
        id: "ci.inventory".to_string(), status: Status::Failed, cause, exit_code: None,
    }];
    assert(report_failure(rows@));
    (Status::Failed, rows)
}

/// 诊断失败追加原有列表，保全先前顺序、重复及全部字段，并产生失败终态。
pub fn diagnostic_failure(rows: &mut Vec<CheckResult>) -> (status: Status)
    ensures rows.len() == old(rows).len() + 1,
        rows@.take(old(rows).len() as int) == old(rows)@,
        rows@.last().id@ == "ci.execution-diagnostics"@,
        rows@.last().status == Status::Failed,
        rows@.last().cause@ == "execution diagnostic publication failed; original errors retained"@,
        rows@.last().exit_code.is_none(),
        status == Status::Failed, status == report_status(rows@),
{
    rows.push(CheckResult {
        id: "ci.execution-diagnostics".to_string(), status: Status::Failed,
        cause: "execution diagnostic publication failed; original errors retained".to_string(),
        exit_code: None,
    });
    assert(report_failure(rows@));
    Status::Failed
}

pub open spec fn matching_result(
    values: Seq<CheckResult>, id: Seq<char>, status: Status, exit_code: Option<i32>,
) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i]).id@ == id
        && values[i].status == status && values[i].exit_code == exit_code
}

/// 同一条实际结果必须同时满足精确 ID、状态与退出码。
pub fn has_result(
    values: &Vec<CheckResult>, id: &str, status: &Status, exit_code: Option<i32>,
) -> (found: bool)
    ensures found <==> matching_result(values@, id@, *status, exit_code),
{
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> !(
                (#[trigger] values[j]).id@ == id@ && values[j].status == *status
                    && values[j].exit_code == exit_code),
        decreases values.len() - i,
    {
        let value = &values[i];
        if <&str>::exec_eq(value.id.as_str(), id) && value.status as u8 == *status as u8
            && value.exit_code == exit_code {
            return true;
        }
        i += 1;
    }
    false
}

pub open spec fn valid_result_states(values: Seq<CheckResult>, not_required: Seq<&str>) -> bool {
    matching_result(values, "ci.inventory"@, Status::Passed, None)
        && forall|i: int| 0 <= i < values.len()
            && text_member(not_required, (#[trigger] values[i]).id@)
                ==> values[i].status == Status::NotRequired
}

/// 库存标记要求 Passed/None；未要求结果只约束状态，保留原退出码合同。
pub fn matches_result_states(values: &Vec<CheckResult>, not_required: &Vec<&str>) -> (valid: bool)
    ensures valid <==> valid_result_states(values@, not_required@),
{
    if !has_result(values, "ci.inventory", &Status::Passed, None) { return false; }
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(),
            matching_result(values@, "ci.inventory"@, Status::Passed, None),
            forall|j: int| 0 <= j < i
                && text_member(not_required@, (#[trigger] values[j]).id@)
                    ==> values[j].status == Status::NotRequired,
        decreases values.len() - i,
    {
        let value = &values[i];
        if contains_text(not_required, value.id.as_str())
            && !matches!(value.status, Status::NotRequired) { return false; }
        i += 1;
    }
    true
}

pub open spec fn text_member(values: Seq<&str>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target
}

pub fn contains_text(values: &Vec<&str>, target: &str) -> (found: bool)
    ensures found <==> text_member(values@, target@),
{
    let mut i = 0;
    while i < values.len()
        invariant
            0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] values[j])@ != target@,
        decreases values.len() - i,
    {
        if <&str>::exec_eq(values[i], target) { return true; }
        i += 1;
    }
    false
}

pub open spec fn same_text_membership(actual: Seq<&str>, expected: Seq<&str>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        text_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, (#[trigger] expected[i])@))
}

/// 不依赖顺序。重复 ID 的拒绝归响应协议，不由集合相等推导唯一性。
pub fn matches_text_membership(actual: &Vec<&str>, expected: &Vec<&str>) -> (matches: bool)
    ensures matches <==> same_text_membership(actual@, expected@),
{
    let mut i = 0;
    while i < actual.len()
        invariant 0 <= i <= actual.len(),
            forall|j: int| 0 <= j < i ==>
                text_member(expected@, (#[trigger] actual[j])@),
        decreases actual.len() - i,
    {
        if !contains_text(expected, actual[i]) { return false; }
        i += 1;
    }
    let mut i = 0;
    while i < expected.len()
        invariant 0 <= i <= expected.len(),
            forall|j: int| 0 <= j < actual.len() ==>
                text_member(expected@, (#[trigger] actual[j])@),
            forall|j: int| 0 <= j < i ==>
                text_member(actual@, (#[trigger] expected[j])@),
        decreases expected.len() - i,
    {
        if !contains_text(actual, expected[i]) { return false; }
        i += 1;
    }
    true
}

pub open spec fn text_values(values: Seq<&str>) -> Seq<Seq<char>> {
    values.map(|i: int, text: &str| text@)
}

pub open spec fn character_member(values: Seq<Seq<char>>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i]) == target
}

proof fn text_member_representation(values: Seq<&str>, target: Seq<char>)
    ensures text_member(values, target) <==> character_member(text_values(values), target),
{
    if text_member(values, target) {
        let i = choose|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target;
        assert(text_values(values)[i] == target);
    }
    if character_member(text_values(values), target) {
        let i = choose|i: int| 0 <= i < text_values(values).len()
            && text_values(values)[i] == target;
        assert(values[i]@ == target);
    }
}

pub open spec fn result_membership(actual: Seq<&str>, expected: Seq<Seq<char>>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        character_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, #[trigger] expected[i]))
}

proof fn result_membership_representation(actual: Seq<&str>, expected: Seq<&str>)
    ensures same_text_membership(actual, expected)
        <==> result_membership(actual, text_values(expected)),
{
    if same_text_membership(actual, expected) {
        assert forall|i: int| 0 <= i < actual.len() implies
            character_member(text_values(expected), (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < text_values(expected).len() implies
            text_member(actual, #[trigger] text_values(expected)[i]) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, expected[i]@));
            }
    }
    if result_membership(actual, text_values(expected)) {
        assert forall|i: int| 0 <= i < actual.len() implies
            text_member(expected, (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            text_member(actual, (#[trigger] expected[i])@) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, text_values(expected)[i]));
            }
    }
}

pub open spec fn expected_result_ids(
    operations: Seq<&str>, not_required: Seq<&str>,
) -> Seq<Seq<char>> {
    seq!["ci.inventory"@] + text_values(operations) + text_values(not_required)
}

/// 借用登记列表，逐项保留原顺序、重复及完整字符串身份。
pub fn borrow_texts<'a>(values: &'a Vec<String>) -> (texts: Vec<&'a str>)
    ensures texts.len() == values.len(),
        forall|i: int| 0 <= i < values.len() ==> (#[trigger] texts[i])@ == values[i]@,
{
    let mut texts: Vec<&'a str> = Vec::new();
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(), texts.len() == i,
            forall|j: int| 0 <= j < i ==> (#[trigger] texts[j])@ == values[j]@,
        decreases values.len() - i,
    {
        texts.push(values[i].as_str());
        i += 1;
    }
    texts
}

/// 借用真实报告字段，逐项保留原顺序、重复及完整字符串身份。
pub fn result_ids<'a>(values: &'a Vec<CheckResult>) -> (ids: Vec<&'a str>)
    ensures ids.len() == values.len(),
        forall|i: int| 0 <= i < values.len() ==> (#[trigger] ids[i])@ == values[i].id@,
{
    let mut ids: Vec<&'a str> = Vec::new();
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(), ids.len() == i,
            forall|j: int| 0 <= j < i ==> (#[trigger] ids[j])@ == values[j].id@,
        decreases values.len() - i,
    {
        ids.push(values[i].id.as_str());
        i += 1;
    }
    ids
}

pub open spec fn report_id_member(actual: Seq<CheckResult>, id: Seq<char>) -> bool {
    exists|j: int| 0 <= j < actual.len() && (#[trigger] actual[j]).id@ == id
}

pub open spec fn report_result_membership(
    actual: Seq<CheckResult>, expected: Seq<Seq<char>>,
) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        character_member(expected, (#[trigger] actual[i]).id@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            report_id_member(actual, #[trigger] expected[i]))
}

proof fn report_membership_representation(
    actual: Seq<CheckResult>, ids: Seq<&str>, expected: Seq<Seq<char>>,
)
    requires ids.len() == actual.len(),
        forall|i: int| 0 <= i < actual.len() ==> (#[trigger] ids[i])@ == actual[i].id@,
    ensures result_membership(ids, expected) <==> report_result_membership(actual, expected),
{
    if result_membership(ids, expected) {
        assert forall|i: int| 0 <= i < actual.len() implies
            character_member(expected, (#[trigger] actual[i]).id@) by {
                assert(actual[i].id@ == ids[i]@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            report_id_member(actual, #[trigger] expected[i]) by {
                let j = choose|j: int| 0 <= j < ids.len()
                    && (#[trigger] ids[j])@ == expected[i];
                assert(actual[j].id@ == expected[i]);
            }
    }
    if report_result_membership(actual, expected) {
        assert forall|i: int| 0 <= i < ids.len() implies
            character_member(expected, (#[trigger] ids[i])@) by {
                assert(ids[i]@ == actual[i].id@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            text_member(ids, #[trigger] expected[i]) by {
                let j = choose|j: int| 0 <= j < actual.len()
                    && (#[trigger] actual[j]).id@ == expected[i];
                assert(ids[j]@ == expected[i]);
            }
    }
}

/// 直接从真实报告取得 ID；调用方无需提供未经证明的字段投影。
pub fn matches_report_result_ids(
    actual: &Vec<CheckResult>, operations: &Vec<&str>, not_required: &Vec<&str>,
) -> (matches: bool)
    ensures matches <==> report_result_membership(
        actual@, expected_result_ids(operations@, not_required@)),
{
    let ids = result_ids(actual);
    let matches = matches_result_ids(&ids, operations, not_required);
    proof {
        report_membership_representation(
            actual@, ids@, expected_result_ids(operations@, not_required@));
    }
    matches
}

/// 期望结果恰为库存、登记操作和未要求测试；不从实际结果补期望。
pub fn matches_result_ids(
    actual: &Vec<&str>, operations: &Vec<&str>, not_required: &Vec<&str>,
) -> (matches: bool)
    ensures matches <==> result_membership(
        actual@, expected_result_ids(operations@, not_required@)),
{
    let mut expected = Vec::new();
    expected.push("ci.inventory");
    let mut i = 0;
    while i < operations.len()
        invariant 0 <= i <= operations.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@).take(i as int),
        decreases operations.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(operations[i]);
        proof {
            assert(text_values(expected@) =~= before.push(operations@[i as int]@));
            assert(text_values(operations@).take(i + 1)
                =~= text_values(operations@).take(i as int).push(operations@[i as int]@));
        }
        i += 1;
    }
    let mut i = 0;
    while i < not_required.len()
        invariant 0 <= i <= not_required.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@)
                + text_values(not_required@).take(i as int),
        decreases not_required.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(not_required[i]);
        proof {
            assert(text_values(expected@) =~= before.push(not_required@[i as int]@));
            assert(text_values(not_required@).take(i + 1)
                =~= text_values(not_required@).take(i as int).push(not_required@[i as int]@));
        }
        i += 1;
    }
    let matches = matches_text_membership(actual, &expected);
    proof {
        assert(text_values(expected@) =~= expected_result_ids(operations@, not_required@));
        result_membership_representation(actual@, expected@);
    }
    matches
}

}
