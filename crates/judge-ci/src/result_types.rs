//! Rows produced by the CI judge and read back from unit reports.
use vstd::prelude::*;

verus! {
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Status { Passed, Failed, NotRequired, Blocked }

pub struct CheckResult {
    pub id: String,
    pub status: Status,
    pub cause: String,
    pub exit_code: Option<i32>,
}
}

#[cfg(not(verus_keep_ghost))]
impl From<chrono_harness::CheckResult> for CheckResult {
    fn from(row: chrono_harness::CheckResult) -> Self {
        Self {
            id: row.id,
            status: match row.status {
                chrono_harness::Status::Passed => Status::Passed,
                chrono_harness::Status::Failed => Status::Failed,
                chrono_harness::Status::NotRequired => Status::NotRequired,
                chrono_harness::Status::Blocked => Status::Blocked,
            },
            cause: row.cause,
            exit_code: row.exit_code,
        }
    }
}

#[cfg(not(verus_keep_ghost))]
impl From<Status> for chrono_harness::Status {
    fn from(status: Status) -> Self {
        match status {
            Status::Passed => Self::Passed,
            Status::Failed => Self::Failed,
            Status::NotRequired => Self::NotRequired,
            Status::Blocked => Self::Blocked,
        }
    }
}

#[cfg(not(verus_keep_ghost))]
impl From<CheckResult> for chrono_harness::CheckResult {
    fn from(row: CheckResult) -> Self {
        Self {
            id: row.id,
            status: row.status.into(),
            cause: row.cause,
            exit_code: row.exit_code,
        }
    }
}
