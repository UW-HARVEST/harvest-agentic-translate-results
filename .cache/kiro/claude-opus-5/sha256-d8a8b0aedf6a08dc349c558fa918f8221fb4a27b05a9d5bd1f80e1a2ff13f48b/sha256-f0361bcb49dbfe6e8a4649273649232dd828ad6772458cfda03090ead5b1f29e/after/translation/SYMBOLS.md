# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-tyyXk5.so
nm -D --defined-only translation/target/release/libbetagamma_lib.so
```

The C library is built from exactly one translation unit (`c_src/src/lib.c`,
per `c_src/CMakeLists.txt`). `c_src/include/lib.h` declares only `betagamma`,
but the other four functions in `lib.c` are non-`static`, so they are exported
too and are part of the ABI surface that must be reproduced.

## Exported (defined, global text) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|--------------|-------|
| 1 | `create_block`   | yes (T) | yes (T) | returns `DataBlock` (40 bytes) by value — MEMORY-class, hidden sret pointer |
| 2 | `allocate_block` | yes (T) | yes (T) | returns `MemoryBlock*` |
| 3 | `free_block`     | yes (T) | yes (T) | void |
| 4 | `compute_hash`   | yes (T) | yes (T) | reads raw pointer values out of the two structs |
| 5 | `betagamma`      | yes (T) | yes (T) | the only symbol declared in the public header |

**Missing symbols: 0.** No implementation was absent, so no C source needed to
be translated in this phase and no stubs were introduced.

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports, i.e. 0 missing non-libc symbols:

* allocator + string: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`,
  `strcpy`, `strlen`, `memcpy`, `memmove`, `memset`, `bcmp`
* Rust std/panic runtime support: `_Unwind_*`, `abort`, `__errno_location`,
  `__tls_get_addr`, `pthread_key_*`, `dl_iterate_phdr`
* std file/IO support pulled in by the std prelude: `open64`, `read`, `write`,
  `writev`, `close`, `lseek64`, `stat64`, `fstat64`, `statx`, `getcwd`,
  `readlink`, `realpath`, `getenv`, `mmap64`, `munmap`, `syscall`
* weak: `_ITM_*TMCloneTable`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
  `__gmon_start__`, `gettid`

The extra std-related imports are inert (the C `.so` needs none of them); they
do not add or remove any *defined* symbol, so ABI parity holds.

Critically, the Rust `.so` imports `malloc` / `calloc` / `free` from libc
rather than shipping its own allocator. That is required for fidelity: see
`CONFIGS.md` axis **H** — `compute_hash` observes raw heap addresses, so the
Rust build must drive the *same* allocator with the *same* request sizes in the
*same* order as the C build.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
configuration is the default one. The Phase D "repeat for every feature combo"
requirement collapses to a single combination; `--no-default-features` is still
exercised by `scripts/check_features.sh` for completeness.
