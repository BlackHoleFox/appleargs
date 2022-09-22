#![doc = include_str!("../README.md")]
#![deny(missing_docs, clippy::undocumented_unsafe_blocks)]

use core::iter::FusedIterator;
use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;

pub mod env;

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos"
))]
mod sys;

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos"
)))]
mod sys {
    #[inline]
    #[cfg(feature = "empty-on-unsupported")]
    pub(super) fn args_slice() -> &'static [&'static [u8]] {
        &[]
    }
    #[cfg(not(feature = "empty-on-unsupported"))]
    compile_error!(
        "The `appleargs` crate is unsupported on this target, \
        and the `\"empty-on-unsupported\"` cargo feature has \
        not been enabled."
    );
}

use sys::args_slice;

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
    let inner = args_slice().iter();

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
    let inner = args_slice().iter();

    AppleArgsOs { inner }
}

fn str_from_slice<'a>(bytes: &&'a [u8]) -> &'a str {
    core::str::from_utf8(bytes).expect("apple argument was not valid UTF-8")
}

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
