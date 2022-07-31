# `appleargs` 

[![CI Status](https://github.com/BlackHoleFox/appleargs/workflows/CI/badge.svg)](https://github.com/BlackHoleFox/appleargs/actions)
[![Docs](https://docs.rs/appleargs/badge.svg)](https://docs.rs/appleargs)
[![Latest Version](https://img.shields.io/crates/v/appleargs.svg)](https://crates.io/crates/appleargs)
<!-- Add when `CStr` in core stabilizes ![MSRV](https://img.shields.io/badge/MSRV%201.64-blue.svg) -->

A smol crate to grab your process' "apple arguments"

## What are apple arguments?

They are an extra set of strings optionally passed to an executable by the kernel on Darwin-based operating systems. They are entirely undocumented (Open an issue if you find it :D), and as far as anyone can tell, solely intended to store or hint precomputed information about the running process for `dyld` to use. 

The values are set during the [exec sequence] of a process and subsequently read by `dyld` at various points when it starts an executable. While these can easily change, `dyld` is open source so it can be referenced for good examples like...

- [building executable launch caches]
- [checking pointer auth configuration]
- [determine platform binary support]

`kern_exec.c` seems to have the [full list of parameters] that could appear. This crate doesn't attempt to document or parse them because of their amazingly unstable nature.

### Example
```text
"executable_path=/Users/person/dev/project/target/debug/bin"
"ptr_munge="
"main_stack="
"executable_file=0x1a0100000f,0x71b112"
"dyld_file=0x1a0100000f,0xfffffff000dc897"
"executable_cdhash=acd984a2fa40d1b36ba71094e7c0318a6bf15084"
"executable_boothash=cd0228d404782f85c4ef3d65dc2ae92aaa66578b"
"arm64e_abi=os"
"th_port="
```

## Supported Operating Systems
This crate should on most macOS versions (but is not explictly tested). Automated testing occurs on the latest version of macOS. It also should work on iOS.

[exec sequence]: https://github.com/apple-oss-distributions/xnu/blob/e7776783b89a353188416a9a346c6cdb4928faad/bsd/kern/kern_exec.c#L5508

[building executable launch caches]: https://github.com/apple-oss-distributions/dyld/blob/3a0a4f7221ce977f01c90b50bb48b7c9406c8589/dyld/DyldRuntimeState.cpp#L2211

[checking pointer auth configuration]: https://github.com/apple-oss-distributions/dyld/blob/3a0a4f7221ce977f01c90b50bb48b7c9406c8589/dyld/DyldProcessConfig.cpp#L466

[determine platform binary support]: https://github.com/apple-opensource/dyld/blob/e3f88907bebb8421f50f0943595f6874de70ebe0/src/dyld2.cpp#L6653

[full list of parameters]: https://github.com/apple-oss-distributions/xnu/blob/e7776783b89a353188416a9a346c6cdb4928faad/bsd/kern/kern_exec.c#L5399-L5456
