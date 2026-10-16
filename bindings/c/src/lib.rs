//! C ABI over the cadetta CLI entry point (docs/spec/bindings.md 4).
//! frob:ticket 01M4SV2T5ACY1EFVCXF3JQ1E7R
//!
//! The surface is deliberately one call: `cadetta_run` executes the same
//! `run_with_logging` the CLI and the Python binding use, so every query's
//! `--format json` document is identical by construction. Strings cross the
//! boundary as NUL-terminated UTF-8 and are released with `cadetta_free`.

use std::ffi::{CStr, CString, c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// Exit value for unusable arguments (mirrors `CADETTA_E_ARGS` in cadetta.h).
pub const E_ARGS: c_int = -1;
/// Exit value for a core panic (mirrors `CADETTA_E_PANIC` in cadetta.h).
pub const E_PANIC: c_int = -2;

/// The core version string, e.g. `cadetta 0.1.0`, as a NUL-terminated static.
#[unsafe(no_mangle)]
pub extern "C" fn cadetta_version() -> *const c_char {
    version_cstr().as_ptr()
}

fn version_cstr() -> &'static CStr {
    use std::sync::OnceLock;
    static FULL: OnceLock<CString> = OnceLock::new();
    FULL.get_or_init(|| CString::new(cadetta::VERSION).unwrap_or_default())
}

/// Turn captured bytes into an owned C string (lossy UTF-8, NULs dropped).
fn to_c(bytes: &[u8]) -> *mut c_char {
    let text: String = String::from_utf8_lossy(bytes).replace('\0', "");
    CString::new(text).map_or(ptr::null_mut(), CString::into_raw)
}

/// Store `bytes` into `slot` when the caller asked for the stream.
///
/// # Safety
/// `slot` must be null or valid for a pointer write.
unsafe fn put(slot: *mut *mut c_char, bytes: &[u8]) {
    if !slot.is_null() {
        // SAFETY: caller guarantees `slot` is writable when non-null.
        unsafe { *slot = to_c(bytes) };
    }
}

/// Run the CLI with `argc` arguments from `argv`; see `cadetta.h`.
///
/// # Safety
/// `argv` must point to `argc` valid NUL-terminated strings (or be null
/// with `argc == 0`); `out` and `err` must each be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cadetta_run(
    argc: c_int,
    argv: *const *const c_char,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> c_int {
    // SAFETY: forwarded caller contract.
    unsafe {
        put(out, b"");
        put(err, b"");
    }
    let Some(args) = (unsafe { collect_args(argc, argv) }) else {
        tracing::warn!(argc, "cadetta_run: unusable argv");
        return E_ARGS;
    };
    tracing::debug!(?args, "cadetta_run");
    let mut so: Vec<u8> = Vec::new();
    let mut se: Vec<u8> = Vec::new();
    let ran = catch_unwind(AssertUnwindSafe(|| {
        cadetta::run_with_logging(&args, &mut so, &mut se)
    }));
    // SAFETY: forwarded caller contract; previous empty strings are replaced
    // after being freed.
    unsafe {
        free_slot(out);
        free_slot(err);
    }
    match ran {
        Ok(code) => {
            // SAFETY: as above.
            unsafe {
                put(out, &so);
                put(err, &se);
            }
            c_int::from(code)
        }
        Err(_) => {
            tracing::error!("cadetta_run: core panicked");
            // SAFETY: as above.
            unsafe {
                put(out, &so);
                put(err, b"cadetta: internal error (panic)");
            }
            E_PANIC
        }
    }
}

/// Free and null the string in `slot`, when present.
///
/// # Safety
/// `slot` must be null or point to a pointer from `to_c`.
unsafe fn free_slot(slot: *mut *mut c_char) {
    if !slot.is_null() {
        // SAFETY: `slot` holds a pointer produced by `to_c`.
        unsafe {
            cadetta_free(*slot);
            *slot = ptr::null_mut();
        }
    }
}

/// Copy `argv` into owned strings; `None` on null or non-UTF-8 entries.
///
/// # Safety
/// See [`cadetta_run`].
unsafe fn collect_args(argc: c_int, argv: *const *const c_char) -> Option<Vec<String>> {
    let n = usize::try_from(argc).ok()?;
    if n == 0 {
        return Some(Vec::new());
    }
    if argv.is_null() {
        return None;
    }
    (0..n)
        .map(|i| {
            // SAFETY: caller guarantees `argc` valid entries.
            let p = unsafe { *argv.add(i) };
            if p.is_null() {
                return None;
            }
            // SAFETY: non-null NUL-terminated per the contract.
            unsafe { CStr::from_ptr(p) }
                .to_str()
                .ok()
                .map(str::to_owned)
        })
        .collect()
}

/// Release a string returned through `cadetta_run`; null is a no-op.
///
/// # Safety
/// `s` must be null or a pointer returned by `cadetta_run`, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cadetta_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: `s` came from `CString::into_raw` in `to_c`.
        drop(unsafe { CString::from_raw(s) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_core() {
        // SAFETY: static NUL-terminated string.
        let v = unsafe { CStr::from_ptr(cadetta_version()) };
        assert_eq!(v.to_str().unwrap(), cadetta::VERSION);
    }

    #[test]
    fn run_version_flag_and_free() {
        let a = CString::new("--version").unwrap();
        let argv = [a.as_ptr()];
        let (mut o, mut e) = (ptr::null_mut(), ptr::null_mut());
        // SAFETY: argv has one valid entry; out/err are writable.
        let code = unsafe { cadetta_run(1, argv.as_ptr(), &mut o, &mut e) };
        assert_eq!(code, 0);
        // SAFETY: `o` was set by cadetta_run.
        let text = unsafe { CStr::from_ptr(o) }.to_str().unwrap().to_owned();
        assert!(text.contains("cadetta"), "{text}");
        // SAFETY: pointers from cadetta_run, freed once.
        unsafe {
            cadetta_free(o);
            cadetta_free(e);
            cadetta_free(ptr::null_mut());
        }
    }

    #[test]
    fn null_argv_is_rejected() {
        // SAFETY: argc > 0 with null argv is the case under test.
        let code = unsafe { cadetta_run(1, ptr::null(), ptr::null_mut(), ptr::null_mut()) };
        assert_eq!(code, E_ARGS);
    }
}
