//! Published references and bounded original data used by CI collection.
#[cfg(not(verus_keep_ghost))]
use crate::{runtime_session::Session, storage::Storage, wire::Scope};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, io::Read, path::Path};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub path: String,
    pub sha256: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransport {
    pub source_directory: String,
    pub directory: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportReference {
    pub schema: String,
    pub original: Original,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub reports: Vec<ReportInput>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportInput {
    pub unit: String,
    pub path: String,
    pub sha256: String,
    pub runner_sha256: String,
    pub judge_sha256: String,
    pub artifacts: Option<ArtifactTransport>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullManifest {
    pub schema: String,
    pub reports: Vec<FullReportInput>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullReportInput {
    pub unit: String,
    pub path: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<ArtifactTransport>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCheck {
    pub schema: String,
    pub request_sha256: String,
    pub source: String,
    pub profile: String,
    pub base: Option<String>,
    pub candidate: String,
    pub initial: bool,
    pub context: Option<Context>,
    pub scope: Option<Scope>,
    pub evidence: Value,
    pub originals: Vec<Original>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub path: String,
    pub raw: Vec<u8>,
    pub sha256: String,
    pub semantic_digest: String,
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn original_path(
    root: &Path,
    original: &Original,
    transport: Option<&ArtifactTransport>,
) -> Result<String, String> {
    Storage::open(root)?
        .call(json!({"operation":"resolve-original","original":original,"transport":transport}))
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn read_bounded(root: &Path, path: &str, limit: u64) -> Result<Vec<u8>, String> {
    crate::units::artifact_path(path)?;
    let storage = Storage::open(root)?;
    storage.call::<()>(json!({"operation":"retain-read","path":path}))?;
    let full: std::path::PathBuf = storage.call(json!({"operation":"checked-path","path":path}))?;
    let meta = fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
    if !meta.is_file() || !original_size_allowed(meta.len(), limit) {
        return Err(format!("bounded collection input rejected: {path}"));
    }
    let mut bytes = Vec::new();
    fs::File::open(full)
        .map_err(|e| e.to_string())?
        .take(original_read_cap(limit))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    bounded_original(bytes, limit)
        .map_err(|_| format!("bounded collection input exceeds limit: {path}"))
}

#[cfg(not(verus_keep_ghost))]
pub(super) fn with_originals<T: DeserializeOwned>(
    root: &Path,
    operation: Value,
    mut read: impl FnMut(&Original) -> Result<Vec<u8>, String>,
) -> Result<T, String> {
    let runner = crate::observed::installed(root)?;
    let mut session = Session::open(root, &runner, "retained-artifacts-v1")?;
    let result = session.originals(&operation, &mut read);
    let closed = session.close();
    result.and_then(|value| closed.map(|()| value))
}

use vstd::prelude::*;

verus! {
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

}
