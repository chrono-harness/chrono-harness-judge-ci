//! Rows produced by the CI judge and read back from unit reports.
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Status { Passed, Failed, NotRequired, Blocked }

#[cfg_attr(not(verus_keep_ghost), derive(Debug, serde::Serialize, serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(deny_unknown_fields))]
pub struct CheckResult {
    pub id: String,
    pub status: Status,
    pub cause: String,
    pub exit_code: Option<i32>,
}
}
