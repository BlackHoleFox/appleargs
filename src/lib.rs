#![doc = include_str!("../README.md")]
#![deny(missing_docs, clippy::undocumented_unsafe_blocks)]

use core::iter::FusedIterator;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ffi::OsStr;
use std::os::raw::{c_char, c_int};
use std::os::unix::prelude::OsStrExt;

// todo: (target_os = "tvos", target_os = "watchos") after testing
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
compile_error!("appleargs is not supported on this platform");

/// An iterator over the process' apple arguments.
///
/// This iterator will panic if any of the arguments are not
/// valid UTF-8.
#[derive(Clone)]
pub struct AppleArgs {
    inner: core::slice::Iter<'static, &'static [u8]>,
}

impl core::fmt::Debug for AppleArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.inner.clone().map(str_from_slice))
            .finish()
    }
}

impl Iterator for AppleArgs {
    type Item = &'static str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(str_from_slice)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.len()
    }
}

impl ExactSizeIterator for AppleArgs {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl DoubleEndedIterator for AppleArgs {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(str_from_slice)
    }
}

impl FusedIterator for AppleArgs {}

/// Returns the Apple arguments of the current process as UTF-8 strings.
///
/// The order of the arguments returned is not guaranteed, nor is the count, or the presence any specific item.
///
/// See the top-level documentation's example of what this could return.
#[inline]
pub fn apple_args() -> AppleArgs {
    let inner = args_slice_iter();

    AppleArgs { inner }
}

/// An iterator over the process' apple arguments.
///
/// This iterator does not check that any argument is a valid UTF-8 string.
#[derive(Clone)]
pub struct AppleArgsOs {
    inner: core::slice::Iter<'static, &'static [u8]>,
}

impl core::fmt::Debug for AppleArgsOs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.inner.clone().map(|v| OsStr::from_bytes(v)))
            .finish()
    }
}

impl Iterator for AppleArgsOs {
    type Item = &'static OsStr;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|v| OsStr::from_bytes(v))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.len()
    }
}

impl ExactSizeIterator for AppleArgsOs {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl DoubleEndedIterator for AppleArgsOs {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|v| OsStr::from_bytes(v))
    }
}

impl FusedIterator for AppleArgsOs {}

/// Returns the Apple arguments of the current process.
///
/// The order of the arguments returned is not guaranteed, nor is the count, or the presence any specific item.
///
/// See the top-level documentation's example of what this could return.
#[inline]
pub fn apple_args_os() -> AppleArgsOs {
    let inner = args_slice_iter();

    AppleArgsOs { inner }
}

fn str_from_slice<'a>(bytes: &&'a [u8]) -> &'a str {
    core::str::from_utf8(bytes).expect("apple argument was not valid UTF-8")
}

fn args_slice_iter() -> core::slice::Iter<'static, &'static [u8]> {
    // This synchronizes with the `Release` store and acts as a fence.
    let data = ARGS_DATA.load(Ordering::Acquire);

    NonNull::new(data)
        .map(|ptr| {
            // `Relaxed` is fine because it is fenced by the `Acquire` used
            // for `data` and `len` is written prior to storing `data`.
            let len = ARGS_LEN.load(Ordering::Relaxed);
            // Safety: `ptr` is always a valid slice and `len` always matches
            // because of the orderings.
            unsafe { core::slice::from_raw_parts(ptr.as_ptr(), len) }
        })
        .unwrap_or(&[])
        .iter()
}

static ARGS_DATA: AtomicPtr<&'static [u8]> = AtomicPtr::new(ptr::null_mut());
static ARGS_LEN: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn init_function(
    _argc: c_int,
    _argv: *const *const c_char,
    _envp: *const *const c_char,
    mut applep: *const *const c_char,
) {
    // Set up an abort guard. It's likely to be extremely bad for us to panic
    // inside a `__mod_init_func`, even more than unwinding across C code
    // normally would be. Eventually rustc will set an abort guard up for us in
    // `extern "C" fn`, but for now it doesn't, so we do it manually.
    let panic_in_static_ctor_sounds_bad = AbortGuard;
    let mut v: Vec<&'static [u8]> = Vec::new();

    // Safety: `applep` is not null, so its valid to read another pointer from.
    while !applep.is_null() && !applep.read().is_null() {
        // Safety: See above
        let p: *const c_char = applep.read();

        // Safety: `applep` was pointing at a valid nul-terminated
        // string.
        let len = strlen(p);
        let ptr = p as *const u8;
        let s = core::slice::from_raw_parts(ptr, len); // Explicit nul skip.

        if !s.is_empty() {
            v.push(Box::leak(s.to_owned().into_boxed_slice()));
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
        Box::into_raw(v.into_boxed_slice()).cast::<&'static [u8]>(),
        Ordering::Release,
    );
    // Disarm the abort guard.
    core::mem::forget(panic_in_static_ctor_sounds_bad);
}

extern "C" {
    /// Provided by libc or compiler_builtins.
    fn strlen(s: *const c_char) -> usize;
}

struct AbortGuard;
impl Drop for AbortGuard {
    #[cold]
    #[inline(never)]
    fn drop(&mut self) {
        // It would be better to use the real `abort`, but the only way for this
        // struct to have `Drop` run is if the dtor is run during unwinding of
        // some other panic, which means this will be a double panic (which
        // aborts). That said, this should ever happen unless an allocator
        // panics (and they shouldn't), so whatever.
        panic!("Triggering abort via double-panic");
    }
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

        let args = apple_args_os();
        assert_ne!(!args.count(), 0);
    }
}
