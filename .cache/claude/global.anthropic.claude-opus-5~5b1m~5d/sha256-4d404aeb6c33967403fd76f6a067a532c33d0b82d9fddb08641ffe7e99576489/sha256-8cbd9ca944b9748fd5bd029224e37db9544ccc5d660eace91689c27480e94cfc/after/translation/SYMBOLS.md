# SYMBOLS.md — Phase A symbol surface

C `.so`: `c_src/build/libharvest-work-Iq61VS.so`
Rust `.so`: `translation/target/release/libbetagamma_lib.so`

Command used:
```
nm -D --defined-only <so> | grep ' T '
nm -D --undefined-only <so>
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `create_block`   | T | T | returns `DataBlock` **by value** (sret, 40 bytes) |
| 2 | `allocate_block` | T | T | returns `MemoryBlock*` (heap) |
| 3 | `free_block`     | T | T | void |
| 4 | `compute_hash`   | T | T | pointer-comparison based, no NULL checks |
| 5 | `betagamma`      | T | T | the only symbol declared in `include/lib.h` |

**Missing from Rust `.so`: NONE (0).** The whole C translation unit
(`c_src/src/lib.c`, the only source file in `CMakeLists.txt`) is translated in
`translation/src/lib.rs`; no module was skipped and no symbol is stubbed.

## Undefined symbols

C: `calloc`, `free`, `malloc`, `strcpy` (all glibc) + weak ITM/`__cxa_finalize`/`__gmon_start__`.

Rust: superset of the above; every extra entry is glibc or the language runtime
(`libunwind` `_Unwind_*`, `memcpy`, `mmap64`, `dl_iterate_phdr`, `pthread_key_*`,
… — pulled in by `std`/panic machinery). **0 missing/undefined non-libc symbols.**

## Types crossing the FFI boundary

```c
typedef struct { int id; char name[32]; uint8_t flags; } DataBlock;   /* size 40, align 4 */
typedef struct { int *data; size_t size; }              MemoryBlock; /* size 16, align 8 */
```
Both are `#[repr(C)]` in Rust with identical layout (verified by
`size_of`/`offset_of` assertions in the differential tests).
Bytes 37..40 of `DataBlock` are padding and are **uninitialised in both
implementations**; tests compare `id`/`name`/`flags`, never the padding.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** ⇒ the only
configuration is the default one. `--no-default-features` is therefore
equivalent to the default build (verified by running the full test suite under
both, see `FEATURES` section of the test run log).
