# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-VpWgci.so`
* Rust`.so`: `translation/target/release/libhdr_bitrate_lib.so`

## C `.so` defined symbols (`nm -D | grep -v ' U '`)

| symbol | type | in C `.so` | in Rust `.so` | notes |
|--------|------|-----------|---------------|-------|
| `hdr_bitrate`                 | `T` (global text) | yes | yes | the only real API symbol; declared in `c_src/include/lib.h` |
| `_ITM_deregisterTMCloneTable` | `w` (weak undef)  | yes | yes | toolchain/CRT artifact, not library API |
| `_ITM_registerTMCloneTable`   | `w` (weak undef)  | yes | yes | toolchain/CRT artifact |
| `__cxa_finalize@GLIBC_2.2.5`  | `w` (weak undef)  | yes | yes | libc |
| `__gmon_start__`              | `w` (weak undef)  | yes | yes | toolchain/CRT artifact |

## Rust-only symbols

The Rust `.so` additionally carries the weak libc references
`__cxa_thread_atexit_impl@GLIBC_2.18`, `gettid@GLIBC_2.30`,
`statx@GLIBC_2.28`. These are weak *undefined* imports pulled in by the Rust
standard library, not exported API, so they do not affect parity.

## Missing-symbol analysis

The whole C library is one translation unit (`c_src/src/lib.c`, 14 lines) with a
single external definition. Nothing is macro-generated, nothing is `static`
and exported, and no C source file went untranslated.

**Symbol diff (C-defined symbols absent from the Rust `.so`): EMPTY.**

Verification command (must print nothing):

```sh
comm -23 \
  <(nm -D c_src/build/libharvest-work-VpWgci.so   | grep -v ' U ' | awk '{print $NF}' | sort -u) \
  <(nm -D translation/target/release/libhdr_bitrate_lib.so | grep -v ' U ' | awk '{print $NF}' | sort -u)
```

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
