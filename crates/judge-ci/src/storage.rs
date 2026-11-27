//! Live capability for the CI acquisition or inventory. Commands inherit the exact retained descriptors.
#[cfg(not(verus_keep_ghost))]
use crate::runtime_session::Session;
#[cfg(not(verus_keep_ghost))]
use serde::de::DeserializeOwned;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    cell::RefCell,
    collections::BTreeMap,
    os::fd::{AsFd, AsRawFd, OwnedFd},
    path::{Path, PathBuf},
    process::Command,
    rc::{Rc, Weak},
};
#[cfg(not(verus_keep_ghost))]
thread_local! { static ACTIVE:RefCell<BTreeMap<(PathBuf, PathBuf),Weak<RefCell<Session>>>> = RefCell::new(BTreeMap::new()); }
#[cfg(not(verus_keep_ghost))]
#[derive(Clone)]
pub(crate) struct Storage(Rc<RefCell<Session>>);
#[cfg(not(verus_keep_ghost))]
impl Storage {
    pub fn open(root: &Path) -> Result<Self, String> {
        let program = crate::observed::installed(root)?;
        let key = (root.to_owned(), program.to_owned());
        if let Some(session) = ACTIVE.with(|all| all.borrow().get(&key).and_then(Weak::upgrade)) {
            return Ok(Self(session));
        }
        let session = Rc::new(RefCell::new(Session::open(
            root,
            &program,
            "retained-artifacts-v1",
        )?));
        ACTIVE.with(|all| {
            let mut all = all.borrow_mut();
            all.retain(|_, weak| weak.strong_count() > 0);
            all.insert(key, Rc::downgrade(&session));
        });
        Ok(Self(session))
    }
    pub fn with<T>(
        root: &Path,
        use_storage: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let storage = Self::open(root)?;
        let result = use_storage();
        close_result(result, storage.close()).map_err(|failure| match failure {
            CloseFailure::Both(error, close) => {
                format!("{error}; closing retained originals: {close}")
            }
            CloseFailure::Single(error) => error,
        })
    }
    pub fn close(self) -> Result<(), String> {
        match Rc::try_unwrap(self.0) {
            Ok(session) => session.into_inner().close(),
            Err(_) => Ok(()),
        }
    }
    pub fn call<T: DeserializeOwned>(&self, op: Value) -> Result<T, String> {
        self.0.borrow_mut().call(op)
    }
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn command(runner: &Path, program: &Path) -> Result<Command, String> {
    let mut descriptors = Vec::new();
    ACTIVE.with(|all| -> Result<(), String> {
        for session in all.borrow().values().filter_map(Weak::upgrade) {
            session.borrow_mut().append_descriptors(&mut descriptors)?;
        }
        Ok(())
    })?;
    let mut command = Command::new(runner);
    command.arg("exec-with-descriptors-v1").arg(program);
    if !descriptors.is_empty() {
        use std::os::unix::process::CommandExt;
        let mut carrier = Vec::new();
        if let Some(inherited) = std::env::var_os("CHRONO_PROCESS_FDS") {
            carrier.push(
                inherited
                    .into_string()
                    .map_err(|_| "non-UTF8 ownership descriptors")?,
            );
        }
        carrier.extend(descriptors.iter().map(|fd| fd.as_raw_fd().to_string()));
        command.env("CHRONO_PROCESS_FDS", carrier.join(","));
        // The child duplicates the descriptors in its own exec wrapper. The parent keeps close-on-exec set.
        unsafe {
            command.pre_exec(move || {
                for fd in &descriptors {
                    rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::empty())
                        .map_err(std::io::Error::from)?;
                }
                Ok(())
            });
        }
    }
    Ok(command)
}

#[cfg(not(verus_keep_ghost))]
impl Session {
    fn append_descriptors(&mut self, result: &mut Vec<OwnedFd>) -> Result<(), String> {
        use rustix::net::{
            RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, recvmsg,
        };
        use std::{io::IoSliceMut, mem::MaybeUninit};
        const BATCH: usize = 32;
        let mut offset = 0;
        loop {
            let n: usize = self.call(json!({"operation":"scoped-descriptors","offset":offset}))?;
            if n == 0 {
                return Ok(());
            }
            if n > BATCH {
                return Err("invalid retained descriptor count".into());
            }
            let mut marker = [0u8];
            let mut buffer = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(BATCH))];
            let mut controls = RecvAncillaryBuffer::new(&mut buffer);
            #[cfg(target_os = "linux")]
            let flags = RecvFlags::CMSG_CLOEXEC;
            #[cfg(not(target_os = "linux"))]
            let flags = RecvFlags::empty();
            let msg = recvmsg(
                self.socket.as_ref().ok_or("closed retained session")?,
                &mut [IoSliceMut::new(&mut marker)],
                &mut controls,
                flags,
            )
            .map_err(|e| e.to_string())?;
            if msg.bytes != 1 || marker != [1] || msg.flags.contains(ReturnFlags::CTRUNC) {
                return Err("invalid retained descriptor transfer".into());
            }
            let before = result.len();
            for control in controls.drain() {
                if let RecvAncillaryMessage::ScmRights(fds) = control {
                    for fd in fds {
                        rustix::io::fcntl_setfd(fd.as_fd(), rustix::io::FdFlags::CLOEXEC)
                            .map_err(|e| e.to_string())?;
                        result.push(fd);
                    }
                }
            }
            if result.len() - before != n {
                return Err("incomplete retained descriptor transfer".into());
            }
            if n < BATCH {
                return Ok(());
            }
            offset += n;
        }
    }
}

use vstd::prelude::*;
verus! {
pub enum CloseFailure<E> { Single(E), Both(E,E) }

pub fn close_result<T,E>(work: Result<T,E>, close: Result<(),E>) -> (result: Result<T,CloseFailure<E>>)
    ensures result == match (work,close) {
        (Err(first),Err(last)) => Err(CloseFailure::Both(first,last)),
        (Err(error),_) | (_,Err(error)) => Err(CloseFailure::Single(error)),
        (Ok(value),Ok(())) => Ok(value),
    },
{
    match (work,close) {
        (Err(first),Err(last)) => Err(CloseFailure::Both(first,last)),
        (Err(error),_) | (_,Err(error)) => Err(CloseFailure::Single(error)),
        (Ok(value),Ok(())) => Ok(value),
    }
}
}
