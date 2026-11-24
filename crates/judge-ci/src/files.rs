//! File and executable identities used to bind the CI judge inputs.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn copy_hashed(mut input: impl Read, mut output: impl Write) -> Result<(String, u64), String> {
    let mut buffer = [0; 65536];
    let mut hash = Sha256::new();
    let mut total = 0u64;
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        hash.update(&buffer[..n]);
        total = total.checked_add(n as u64).ok_or("stream size overflow")?;
    }
    Ok((format!("{:x}", hash.finalize()), total))
}
pub fn file_identity(path: &Path) -> Result<(String, u64), String> {
    if !fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .is_file()
    {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    let input = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !input.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    copy_hashed(input, std::io::sink())
}
pub fn resolve_program(
    root: &Path,
    program: &str,
    search: Option<&str>,
) -> Result<PathBuf, String> {
    let root = std::path::absolute(root).map_err(|e| e.to_string())?;
    let error = || format!("executable not found: {program}");
    if program.contains('/') {
        let candidate = root.join(program);
        return candidate.is_file().then_some(candidate).ok_or_else(error);
    }
    let path = search
        .map(std::ffi::OsString::from)
        .or_else(|| std::env::var_os("PATH"))
        .ok_or("PATH is missing")?;
    std::env::split_paths(&path)
        .map(|dir| root.join(dir).join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(error)
}
/// Check the runner wire's lexical relative path before using native host filesystem APIs.
pub fn relative_path(relative: &str) -> Result<(), String> {
    if relative.is_empty()
        || relative == "."
        || relative.starts_with("./")
        || relative.starts_with('/')
        || relative.split('/').any(|part| part == "..")
        || relative
            .bytes()
            .any(|b| matches!(b, 0 | b'\n' | b'\r' | b'\\'))
    {
        return Err(format!("invalid relative path: {relative:?}"));
    }
    Ok(())
}
pub fn no_symlink_parents(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let mut path = root.to_owned();
    for part in Path::new(relative).components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("symlink path is not allowed: {}", path.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}
