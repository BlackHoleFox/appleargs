#![doc = include_str!("../README.md")]
#![deny(missing_docs, clippy::undocumented_unsafe_blocks)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

// todo: (target_os = "tvos", target_os = "watchos") after testing
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
compile_error!("appleargs is not supported on this platform");

/// Returns the Apple arguments of the current process.
///
/// The order of the arguments returned is not guaranteed, nor is the count, or the presence any specific item.
///
/// See the top-level documentation's example of what this could return.
pub fn apple_args(
) -> impl Iterator<Item = &'static CString> + ExactSizeIterator + DoubleEndedIterator + Clone {
    // This synchronizes with the `Release` store and acts as a fence.
    let data = ARGS_DATA.load(Ordering::Acquire);
    NonNull::new(data)
        .map(|ptr| {
            // `Relaxed` is fine because it is fenced by the `Acquire` used
            // for `data` and `len` is written prior to storing `data`.
            let len = ARGS_LEN.load(Ordering::Relaxed);
            // Safety: `ptr` is always a valid slice and `len` always matches
            // because of the orderings.
            unsafe { std::slice::from_raw_parts(ptr.as_ptr(), len) }
        })
        .unwrap_or(&[])
        .iter()
}

static ARGS_DATA: AtomicPtr<CString> = AtomicPtr::new(ptr::null_mut());
static ARGS_LEN: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn init_function(
    _argc: c_int,
    _argv: *const *const c_char,
    _envp: *const *const c_char,
    mut applep: *const *const c_char,
) {
    let mut v: Vec<CString> = Vec::new();

    // Safety: `applep` is not null, so its valid to read another pointer from.
    while !applep.is_null() && !applep.read().is_null() {
        // Safety: See above
        let p: *const i8 = applep.read();

        // Safety: `applep` was pointing at a valid nul-terminated
        // string.
        let s = CStr::from_ptr(p);

        if !s.to_bytes().is_empty() {
            v.push(s.to_owned());
        }

        // Safety: This will never wrap and after incrementing
        // past the last array element, the loop will stop.
        applep = applep.add(1);
    }

    // `Relaxed` is fine because the store of `data` with
    // `Release` acts as a fence, and `len` is always loaded
    // after `data`.
    ARGS_LEN.store(v.len(), Ordering::Relaxed);
    ARGS_DATA.store(
        Box::into_raw(v.into_boxed_slice()).cast::<CString>(),
        Ordering::Release,
    );
}

#[used]
#[cfg_attr(
    any(target_os = "macos", target_os = "ios"),
    link_section = "__DATA,__mod_init_func"
)]
static CTOR: unsafe extern "C" fn(
    argc: c_int,
    argv: *const *const c_char,
    envp: *const *const c_char,
    applep: *const *const c_char,
) = init_function;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_check() {
        let args = apple_args();
        assert_ne!(args.clone().count(), 0);

        for arg in args {
            println!("Arg: {arg:?}");
        }
    }
}
