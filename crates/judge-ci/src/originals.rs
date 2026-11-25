//! Published references and bounded original data used by CI collection.
use crate::{runtime_session::Session, storage::Storage, wire::Scope};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Original {
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransport {
    pub source_directory: String,
    pub directory: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportReference {
    pub schema: String,
    pub original: Original,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub reports: Vec<ReportInput>,
}

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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullManifest {
    pub schema: String,
    pub reports: Vec<FullReportInput>,
}

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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub path: String,
    pub raw: Vec<u8>,
    pub sha256: String,
    pub semantic_digest: String,
}

pub(super) fn original_path(
    root: &Path,
    original: &Original,
    transport: Option<&ArtifactTransport>,
) -> Result<String, String> {
    Storage::open(root)?
        .call(json!({"operation":"resolve-original","original":original,"transport":transport}))
}

pub(super) fn read_bounded(root: &Path, path: &str, limit: u64) -> Result<Vec<u8>, String> {
    crate::units::artifact_path(path)?;
    let storage = Storage::open(root)?;
    storage.call::<()>(json!({"operation":"retain-read","path":path}))?;
    let full: std::path::PathBuf = storage.call(json!({"operation":"checked-path","path":path}))?;
    let meta = fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
    if !meta.is_file() || !crate::verified_collection::original_size_allowed(meta.len(), limit) {
        return Err(format!("bounded collection input rejected: {path}"));
    }
    let mut bytes = Vec::new();
    fs::File::open(full)
        .map_err(|e| e.to_string())?
        .take(crate::verified_collection::original_read_cap(limit))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    crate::verified_collection::bounded_original(bytes, limit)
        .map_err(|_| format!("bounded collection input exceeds limit: {path}"))
}

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
