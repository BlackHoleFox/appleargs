use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::os::raw::{c_char, c_int};

pub fn args_slice() -> &'static [&'static [u8]] {
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
            v.push(Box::leak(s.into()));
        }

        // Safety: This will never wrap and after incrementing
        // past the last array element, the loop will stop.
        applep = applep.add(1);
    }

    let vslice = v.leak::<'static>();
    // `Relaxed` is fine because the store of `data` with
    // `Release` acts as a fence, and `len` is always loaded
    // after `data`.
    ARGS_LEN.store(vslice.len(), Ordering::Relaxed);
    ARGS_DATA.store(vslice.as_mut_ptr(), Ordering::Release);
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
