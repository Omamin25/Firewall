//! Win32 attribution: connection tables → PID, PID → image path, PID → token → user.
//!
//! Every `unsafe` block here is a direct FFI call whose invariants are stated inline.
//! Nothing in this file is called per packet — the resolver in `mod.rs` caches results.

mod tables;

use std::{
    collections::HashMap,
    ffi::c_void,
    mem,
    sync::{Mutex, MutexGuard},
};

use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL},
        Security::{
            Authorization::ConvertSidToStringSidW, GetTokenInformation, LookupAccountSidW,
            TokenUser, PSID, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER,
        },
        System::Threading::{
            OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

use super::{AttributionBackend, AttributionError, ConnectionOwner, ProcessInfo};

/// Wide-char buffer for image paths — comfortably past the 260-char legacy `MAX_PATH`.
const PATH_BUFFER_LEN: usize = 1024;
/// `LookupAccountSidW` output buffers. Account and domain names are far shorter.
const NAME_BUFFER_LEN: usize = 256;
/// Cap on memoised SID→account entries. A host sees a handful of accounts, not many.
const MAX_CACHED_ACCOUNTS: usize = 256;

/// Windows implementation of [`AttributionBackend`].
#[derive(Default)]
pub struct WindowsAttribution {
    /// String SID → `domain\user`. `LookupAccountSidW` can touch a domain controller,
    /// so the answer is kept for the life of the process.
    accounts: Mutex<HashMap<String, String>>,
}

impl AttributionBackend for WindowsAttribution {
    fn connections(&self) -> Result<Vec<ConnectionOwner>, AttributionError> {
        let mut owners = tables::tcp_v4()?;
        owners.extend(tables::tcp_v6()?);
        owners.extend(tables::udp_v4()?);
        owners.extend(tables::udp_v6()?);
        Ok(owners)
    }

    fn process(&self, pid: u32) -> Result<ProcessInfo, AttributionError> {
        // System (4) is a kernel pseudo-process: no image file, no openable token.
        if pid == 4 {
            return Ok(ProcessInfo {
                name: Some("System".to_owned()),
                user_account: Some(r"NT AUTHORITY\SYSTEM".to_owned()),
                ..ProcessInfo::default()
            });
        }

        // SAFETY: no pointer arguments; a failure returns Err rather than a null handle.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .map_err(|error| AttributionError::Process {
                pid,
                message: error.message(),
            })?;
        let handle = OwnedHandle(handle);

        let path = image_path(handle.0);
        let (user_account, user_sid) = self.token_identity(handle.0);
        Ok(ProcessInfo {
            name: path.as_deref().map(file_name),
            path,
            user_account,
            user_sid,
        })
    }
}

impl WindowsAttribution {
    /// `(domain\user, string SID)` for the token that owns `process`. Either half can be
    /// missing: some tokens are readable but their SID does not resolve to a name.
    fn token_identity(&self, process: HANDLE) -> (Option<String>, Option<String>) {
        let mut token = HANDLE::default();
        // SAFETY: `token` is a live out-parameter for the duration of the call.
        if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.is_err() {
            return (None, None);
        }
        let token = OwnedHandle(token);

        let mut needed = 0_u32;
        // The sizing call is expected to fail with ERROR_INSUFFICIENT_BUFFER.
        // SAFETY: passing a null buffer with length 0 is the documented sizing form.
        let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut needed) };
        if needed == 0 {
            return (None, None);
        }

        // `TOKEN_USER` embeds a pointer, so the buffer must be pointer-aligned; a
        // `Vec<u8>` would not guarantee that.
        let words = (needed as usize).div_ceil(mem::size_of::<u64>());
        let mut buffer = vec![0_u64; words];
        let capacity = (words * mem::size_of::<u64>()) as u32;
        // SAFETY: buffer and capacity describe the same allocation.
        let filled = unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                capacity,
                &mut needed,
            )
        };
        if filled.is_err() || (needed as usize) < mem::size_of::<TOKEN_USER>() {
            return (None, None);
        }

        // SAFETY: the call above wrote a TOKEN_USER into `buffer`; the SID it points at
        // lives in the same allocation, which outlives this borrow.
        let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
        if sid.0.is_null() {
            return (None, None);
        }

        let sid_text = sid_string(sid);
        let account = self.account_name(sid, sid_text.as_deref());
        (account, sid_text)
    }

    fn account_name(&self, sid: PSID, sid_text: Option<&str>) -> Option<String> {
        if let Some(text) = sid_text {
            if let Some(cached) = self.lock_accounts().get(text) {
                return Some(cached.clone());
            }
        }

        let mut name = [0_u16; NAME_BUFFER_LEN];
        let mut name_len = name.len() as u32;
        let mut domain = [0_u16; NAME_BUFFER_LEN];
        let mut domain_len = domain.len() as u32;
        let mut kind = SID_NAME_USE::default();
        // SAFETY: each buffer pointer is paired with its own capacity, which the API
        // overwrites with the number of characters actually written.
        unsafe {
            LookupAccountSidW(
                PCWSTR::null(),
                sid,
                PWSTR(name.as_mut_ptr()),
                &mut name_len,
                PWSTR(domain.as_mut_ptr()),
                &mut domain_len,
                &mut kind,
            )
        }
        .ok()?;

        let account = qualify(
            &wide_to_string(&domain, domain_len),
            &wide_to_string(&name, name_len),
        );
        if let Some(text) = sid_text {
            let mut accounts = self.lock_accounts();
            if accounts.len() >= MAX_CACHED_ACCOUNTS {
                accounts.clear();
            }
            accounts.insert(text.to_owned(), account.clone());
        }
        Some(account)
    }

    fn lock_accounts(&self) -> MutexGuard<'_, HashMap<String, String>> {
        self.accounts.lock().unwrap_or_else(|error| {
            tracing::warn!("account cache lock was poisoned; recovering");
            error.into_inner()
        })
    }
}

/// Closes a Win32 handle on drop so every early return stays leak-free.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: the handle came from OpenProcess/OpenProcessToken and is closed once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

fn image_path(process: HANDLE) -> Option<String> {
    let mut buffer = [0_u16; PATH_BUFFER_LEN];
    let mut length = buffer.len() as u32;
    // SAFETY: `buffer`/`length` describe the same array; the API writes at most `length`
    // wide characters and updates `length` with the count written.
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .ok()?;
    let path = wide_to_string(&buffer, length);
    (!path.is_empty()).then_some(path)
}

fn sid_string(sid: PSID) -> Option<String> {
    let mut text = PWSTR::null();
    // SAFETY: the API allocates the string with LocalAlloc and stores it in `text`.
    unsafe { ConvertSidToStringSidW(sid, &mut text) }.ok()?;
    if text.is_null() {
        return None;
    }
    // SAFETY: on success `text` points at a null-terminated wide string.
    let value = unsafe { text.to_string() }.ok();
    // SAFETY: `text` was allocated by ConvertSidToStringSidW and is freed exactly once.
    unsafe {
        let _ = LocalFree(HLOCAL(text.0.cast::<c_void>()));
    }
    value
}

fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(path)
        .to_owned()
}

fn qualify(domain: &str, name: &str) -> String {
    if domain.is_empty() {
        name.to_owned()
    } else {
        format!("{domain}\\{name}")
    }
}

fn wide_to_string(buffer: &[u16], length: u32) -> String {
    let end = (length as usize).min(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
        .trim_end_matches('\0')
        .to_owned()
}
