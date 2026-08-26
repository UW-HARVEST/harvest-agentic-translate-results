# SYMBOLS.md — Phase A: exported symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so   (name comes from the parent dir via
#    cmake_path(GET parent FILENAME project_name) in CMakeLists.txt)

# Rust
cargo build --release
# -> target/release/libhdr_bitrate_lib.so  ([lib] name = "hdr_bitrate_lib",
#    crate-type = ["cdylib"])
```

## C `.so` exported (defined) dynamic symbols

```
00000000000010f9 T hdr_bitrate
```

## Rust `.so` exported (defined) dynamic symbols

```
0000000000011ca0 T hdr_bitrate
```

## Parity table

| # | symbol | declared in | in C `.so` | in Rust `.so` | status |
|---|--------|-------------|-----------|--------------|--------|
| 1 | `hdr_bitrate` | `c_src/include/lib.h` | yes (`T`) | yes (`T`) | **MATCH** |

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so         | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libhdr_bitrate_lib.so | awk '{print $3}' | sort)
(empty)
```

**Result: 0 missing symbols. 0 extra symbols.**

## Completeness of the translation

`c_src/` contains exactly one translation unit (`src/lib.c`, 14 lines) and one
header (`include/lib.h`, 3 lines). `CMakeLists.txt` compiles only `src/lib.c`.
The single declared and defined entry point is `hdr_bitrate`. There is no
untranslated C source file, no macro-generated symbol family, and no
conditionally-compiled symbol: nothing was skipped by the translate step, so no
additional translation work was required for symbol parity.

No stubs / `unimplemented!()` / fake exports were added — the sole export is a
real translation of the C body.

## Undefined (imported) symbols

Verified with `scripts/symbol_parity.sh`. The Rust `.so` imports **49** dynamic
symbols, every one of which is glibc, the GCC unwinder, or a weak
linker/instrumentation stub. There are **0 undefined non-libc symbols**:

| class | symbols |
|-------|---------|
| weak linker/instrumentation stubs | `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__` |
| GCC unwinder (`panic`/landing pads) | `_Unwind_Backtrace`, `_Unwind_Resume`, `_Unwind_Get{IP,IPInfo,DataRelBase,TextRelBase,RegionStart,LanguageSpecificData}`, `_Unwind_Set{GR,IP}` |
| glibc C runtime | `__cxa_finalize`, `__cxa_thread_atexit_impl`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_{create,delete}`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev` |

For comparison the C `.so` imports only 4 (`_ITM_*`, `__cxa_finalize`,
`__gmon_start__`). The extra Rust imports are the Rust standard library's
runtime (allocator, TLS, panic machinery, backtrace support) that `libstd`
pulls in for any `cdylib`; **none** of them is reached by `hdr_bitrate`, which
is a leaf function that calls nothing and allocates nothing.

## Anti-vacuity: the exports are genuinely exercised

A matching `nm -D` proves only that a name exists. To prove the exported
`hdr_bitrate` is a faithful implementation and that the differential tests can
actually observe a difference, 13 deliberate mutations were applied to
`src/lib.rs` and the suite re-run against each:

| mutation | result |
|----------|--------|
| table entry `224 -> 225` | KILLED (8 tests) |
| padding byte value `0 -> 1` | KILLED (8) |
| drop the `-1` on `j` | KILLED (10) |
| `i` reads bit 2 instead of bit 3 | KILLED (11) |
| `k = h2 & 0x0F` instead of `h2 >> 4` | KILLED (14) |
| drop the `* 2` multiplier | KILLED (14) |
| multiplier `2 -> 3` | KILLED (14) |
| read `h[0]` instead of `h[1]` | KILLED (14) |
| read `h[3]` instead of `h[2]` | KILLED (15) |
| off-by-one on the read offset | KILLED (15) |
| clamp negative `j` to `0` (naive "fix" of the C's UB) | KILLED (7) |
| clamp `k` to `14` (naive "fix" of the C's UB) | KILLED (6) |
| `PAD 15 -> 16` | **SURVIVED — provably equivalent mutant** |

`PAD` appears in *both* the table-write offset (`flat[PAD + i*45 + j*15 + k]`)
and the table-read offset (`PAD + i*45 + j*15 + k`), so changing it shifts the
table and every access consistently; the padding stays large enough for the
extreme offsets (`PAD-15 >= 0` and `PAD+90 < PAD+90+PAD`). The mutant is
semantically identical, so surviving is correct, not a test gap.

**12 of 13 mutants killed, 1 provably equivalent → no test gap.**

### Note on artifact freshness (a real bug this caught)

`cargo test` does **not** rebuild the `cdylib`, because the integration-test
binaries do not link against it. An initial version of the harness merely
*located* `target/debug/libhdr_bitrate_lib.so`, and consequently loaded a stale
`.so`: **all 9 mutations initially SURVIVED**, i.e. the whole suite was
vacuous while reporting 28 passes. `tests/harness/mod.rs::ensure_rust_so()` now
invokes `cargo build --lib` into a separate `--target-dir` (a distinct build
lock, so no deadlock with the outer `cargo test`) and asserts the resulting
`.so` is no older than `src/lib.rs`.
