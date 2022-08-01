//! Inspection of the "apple" arguments as if it were another set of environment
//! variables.
//!
//! While the format of the apple arguments is not documented and may be
//! unstable at the moment it is generally identical to the syntax used by
//! strings in the environment[^1]. This seems deliberate, as the apple
//! arguments are passed into the process in a similar manner to `envp` as well.
//!
//! This module provides an interface similar to the environment-reading (but
//! not writing) functions in [`std::env`], but which use the apple arguments
//! instead.
//!
//! The used for the apple arguments may be unstable, so the functions in this
//! module ignore arguments which cannot be parsed as an environment variable
//! (that is, ones which do not contain the `'='` character), rather than
//! producing an error or even panicking.
//!
//! In the future, if the apple arguments change to include strings which do not
//! conform to the environment variable syntax, this module will continue
//! working. If this happens, you can access the "complete" argument set using
//! iterator functions in the crate root, such as [`appleargs::apple_args_os`].
//!
//! [^1]: that is, `"$key=$value"` where `$key` does not contain the `'='`
//!     character, and neither `$key` nor `$value` contain `'\0'`.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;

/// An iterator over the "apple" arguments parsed into UTF-8 "env var"-style
/// key/value pairs.
///
/// This type is most similar to [`std::env::Vars`], but uses the pseudo-env
/// made up of the apple arguments, rather than the "real" environment.
///
/// This struct is returned by [`appleargs::env::apple_vars()`](apple_vars), see
/// it and the [module documentation](crate::env) for more information.
#[derive(Clone)]
#[must_use]
pub struct AppleVars {
    inner: SplitArgsIter,
}

/// An iterator over the "apple" arguments parsed as "env var"-style key/value
/// pairs.
///
/// This type is most similar to [`std::env::VarsOs`], but uses the pseudo-env
/// made up of the apple arguments, rather than the "real" environment.
///
/// This struct is returned by [`env::apple_vars_os()`](apple_vars_os), see it
/// and the [module documentation](crate::env) for more information.
#[derive(Clone)]
#[must_use]
pub struct AppleVarsOs {
    inner: SplitArgsIter,
}

/// Returns an iterator over the key/value pairs in the pseudo-environment
/// provided as apple arguments.
///
/// This is a tuple of `(&str, &str)`. Currently we panic if we encounter
/// invalid UTF-8 is encountered. You should use [`apple_vars_os`] if this is
/// undesirable.
#[inline]
pub fn apple_vars() -> AppleVars {
    AppleVars {
        inner: split_args_iter(),
    }
}

/// Returns an iterator over the key/value pairs in the pseudo-environment
/// provided as apple arguments.
///
/// It is essentially equivalent to [`std::env::vars_os`], but uses apple
/// arguments rather than the process environment.
///
/// This is a tuple of `(&OsStr, &OsStr)`. These are not guaranteed to be UTF-8.
/// If this is undesirable, you should use the [`apple_vars()`] function instead.
#[inline]
pub fn apple_vars_os() -> AppleVarsOs {
    AppleVarsOs {
        inner: split_args_iter(),
    }
}

impl Iterator for AppleVarsOs {
    type Item = (&'static OsStr, &'static OsStr);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (OsStr::from_bytes(k), OsStr::from_bytes(v)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    // Can't provide more efficient impl of anything else. (Note that our inner
    // iterator is not an `ExactSizeIterator`)
}

impl DoubleEndedIterator for AppleVarsOs {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|(k, v)| (OsStr::from_bytes(k), OsStr::from_bytes(v)))
    }
}

impl core::iter::FusedIterator for AppleVarsOs {}

impl core::fmt::Debug for AppleVarsOs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl Iterator for AppleVars {
    type Item = (&'static str, &'static str);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (super::str_from_slice(&k), super::str_from_slice(&v)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    // Can't provide more efficient impl of anything else. (Note that our inner
    // iterator is not an `ExactSizeIterator`)
}

impl DoubleEndedIterator for AppleVars {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|(k, v)| (super::str_from_slice(&k), super::str_from_slice(&v)))
    }
}

impl core::iter::FusedIterator for AppleVars {}

impl core::fmt::Debug for AppleVars {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

type SplitArgsIter = core::iter::FilterMap<
    core::iter::Copied<core::slice::Iter<'static, &'static [u8]>>,
    fn(&[u8]) -> Option<(&[u8], &[u8])>,
>;

#[inline]
fn split_args_iter() -> SplitArgsIter {
    super::args_slice().iter().copied().filter_map(split_kv)
}

#[inline]
fn split_kv<'a>(s: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    debug_assert!(!s.contains(&b'\0'));
    let equals = s.iter().position(|&b| b == b'=')?;
    Some((&s[..equals], &s[(equals + 1)..]))
}

/// Searches the apple argument pseudo-env for a variable with the name `s`, and
/// returns it, if one is found.
///
/// It is analogous to [`std::env::var`], but treats the apple arguments as
/// an environment, rather than using the "real" environment.
///
/// This method returns an error if the value of the variable is not valid
/// UTF-8. See [`apple_var_os`] for a similar function without this requirement.
pub fn apple_var(s: impl AsRef<[u8]>) -> Result<&'static str, VarError> {
    fn apple_var_impl(s: &[u8]) -> Result<&'static str, VarError> {
        if let Some((_, v)) = split_args_iter().find(|&(k, _)| k == s) {
            core::str::from_utf8(v).map_err(|_| VarError::NotUnicode(v))
        } else {
            Err(VarError::NotPresent)
        }
    }
    apple_var_impl(s.as_ref())
}

/// Searches the apple argument pseudo-env for a variable with the name `s`, and
/// returns it, if one is found.
///
/// It is analogous to [`std::env::var_os`], but treats the apple arguments as
/// an environment, rather than using the "real" environment.
///
/// This method returns an [`OsStr`], which may not be valid UTF-8. If this is
/// undesirable, see [`apple_var_os`], which returns an error if the value is
/// not valid UTF-8.
pub fn apple_var_os(s: impl AsRef<OsStr>) -> Option<&'static OsStr> {
    fn apple_var_os_impl(s: &OsStr) -> Option<&'static OsStr> {
        split_args_iter()
            .find(|&(k, _)| k == s.as_bytes())
            .map(|(_, v)| OsStr::from_bytes(v))
    }
    apple_var_os_impl(s.as_ref())
}

/// The error type returned by [`appleargs::env::apple_var`](apple_var).
///
/// Essentially equivalent to [`std::env::VarError`], but uses a static
/// reference (and not a `Vec`) in the `NotUnicode` variant, as we operate on a
/// static copy of the apple arguments.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VarError {
    /// The specified environment variable was not present.
    NotPresent,
    /// The specified environment variable was found, but was not valid UTF-8.
    NotUnicode(&'static [u8]),
}

// #[cfg(test)]
// mod test {
//     static FAKE_ARGS = &[];
//     #[test]
//     fn fake_apple_args()
// }
