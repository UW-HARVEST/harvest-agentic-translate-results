# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-haTboz.so`
* Rust `.so`: `translation/target/release/libupdate_md5_lib.so`

## C translation units

The whole library is two files; both are fully translated (no module skipped):

| C file | status |
|--------|--------|
| `c_src/include/lib.h` | translated (types `tflac_u8/s32/u32/u64`, `struct tflac_md5`, `struct tflac`) |
| `c_src/src/lib.c` | translated (all 3 functions) |

## Exported (defined) dynamic symbols

`nm -D --defined-only <so>`, C side verbatim:

```
00000000000011c7 T tflac_md5_addsample
0000000000001119 T tflac_pack_u64le
0000000000001286 T update_md5
```

Rust side (Rust-internal `_ZN…` mangled symbols elided; they are not part of the
compared surface):

```
00000000000118d0 T tflac_md5_addsample
00000000000119a0 T tflac_pack_u64le
00000000000119b0 T update_md5
```

| # | symbol | in C `.so` | in Rust `.so` | note |
|---|--------|-----------|---------------|------|
| 1 | `tflac_pack_u64le`    | T | T | not declared in `lib.h`, but non-`static` in `lib.c`, so it is part of the ABI |
| 2 | `tflac_md5_addsample` | T | T | likewise not in the header, but exported |
| 3 | `update_md5`          | T | T | the only header-declared entry point |

**Missing from Rust `.so`: 0.**

## Undefined symbols

C `.so` undefined: `_ITM_deregisterTMCloneTable` (w), `_ITM_registerTMCloneTable` (w),
`__cxa_finalize@GLIBC_2.2.5` (w), `__gmon_start__` (w) — all weak CRT stubs.

Rust `.so` undefined: the same weak CRT stubs plus libc (`malloc`, `free`,
`memcpy`, `memmove`, `memset`, `realloc`, `calloc`, `posix_memalign`, `abort`,
`bcmp`, `strlen`, `getenv`, `getcwd`, `open64`/`read`/`write`/`close`/`lseek64`,
`stat64`/`fstat64`/`statx`, `mmap64`/`munmap`, `readlink`, `realpath`,
`syscall`, `writev`, `dl_iterate_phdr`, `__errno_location`, `__tls_get_addr`,
`pthread_key_*`, `pthread_setspecific`, `gettid`, `__cxa_thread_atexit_impl`)
and the `libgcc` unwinder (`_Unwind_*`). These come from `core`/`std`'s panic
machinery and allocator shims, not from untranslated C.

**Undefined non-libc / non-unwinder symbols in Rust `.so`: 0.**

## ABI layout parity (measured, not assumed)

`gcc` on `lib.h` vs. the `#[repr(C)]` Rust structs:

| item | C | Rust |
|------|---|------|
| `sizeof(tflac_md5)` / align | 88 / 8 | 88 / 8 |
| `offsetof(tflac_md5, pos)` | 0 | 0 |
| `offsetof(tflac_md5, total)` | 8 | 8 |
| `offsetof(tflac_md5, buffer)` | 16 | 16 (`MD5_BUFFER_OFFSET`) |
| `sizeof(tflac)` / align | 96 / 8 | 96 / 8 |
| `offsetof(tflac, md5_ctx)` | 0 | 0 |
| `offsetof(tflac, cur_blocksize)` | 88 | 88 |
| `offsetof(tflac, channels)` | 92 | 92 |

Confirmed in the C disassembly: `update_md5` loads `0x58(%rax)` (=88,
`cur_blocksize`) and `0x5c(%rax)` (=92, `channels`); `tflac_md5_addsample`
addresses the buffer as `0x10(%rax,%rdx,1)` (=16, `buffer`).

## Cargo features

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. There is no non-default feature combination to
re-run Phases B–C under (verified: `grep -n '\[features\]' Cargo.toml` → no match).

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — no
`add_executable`. `Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` —
no `[[bin]]` and no `src/main.rs`. There is **no driver binary**, so the
"compare stdout of the two binaries" clause of Phase B does not apply.
