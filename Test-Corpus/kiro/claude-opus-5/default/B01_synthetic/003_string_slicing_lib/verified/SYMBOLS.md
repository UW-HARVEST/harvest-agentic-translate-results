# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libString_Slice.so

cd translation && cargo build --release
# -> translation/target/release/libString_Slice.so
```

## C source inventory

`c_src` contains exactly one translation unit and one public header:

| file | public declarations |
|------|---------------------|
| `c_src/include/slicing.h` | `int slice(char *mystr, int *start_ptr, int *stop_ptr);` |
| `c_src/src/slicing.c` | definition of `slice` (only function in the file) |

There are no namespace/renaming macros, no macro-generated symbol families, no
`#ifdef`-gated alternate entry points, and no second module. So the complete
public surface is a single function.

## `nm -D --defined-only` — C `.so`

```
0000000000001129 T slice
```

(One global text symbol. `nm -D` on the C library also lists the usual
libc imports as *undefined* — `printf`, `strlen`, `_ITM_*`, `__gmon_start__`,
`__cxa_finalize` — which are not part of the exported surface.)

## `nm -D --defined-only` — Rust `.so`

```
00000000000117a0 T slice
```

## Symbol diff

| symbol | in C `.so` | in Rust `.so` | status |
|--------|-----------|---------------|--------|
| `slice` | yes (`T`) | yes (`T`) | OK — exported via `#[unsafe(no_mangle)] pub unsafe extern "C" fn slice` |

**Missing from Rust: none.** No implementation gaps, no un-exported
implementations, no untranslated C modules. No stubs were added.

## Undefined (imported) non-libc symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists 52 entries, every one of
which is glibc or the platform unwinder / GCC runtime:

* the two the translation actually calls: `printf@GLIBC_2.2.5`,
  `strlen@GLIBC_2.2.5`
* `puts@GLIBC_2.2.5` — LLVM's standard `printf("literal\n")` -> `puts("literal")`
  peephole applied to the three fixed error messages. Behaviourally identical
  (same stream, same bytes, same buffering); confirmed byte-for-byte by the
  Phase C tests and by the multi-call stream-ordering test C14.
* allocator / memory / syscall wrappers pulled in by the Rust standard library
  (`malloc`, `calloc`, `realloc`, `free`, `memcpy`, `memmove`, `memset`, `bcmp`,
  `posix_memalign`, `mmap64`, `munmap`, `read`, `write`, `writev`, `open64`,
  `close`, `lseek64`, `stat64`, `fstat64`, `statx`, `getcwd`, `getenv`,
  `readlink`, `realpath`, `syscall`, `abort`, `__errno_location`)
* thread/TLS and unwinder symbols (`pthread_key_*`, `pthread_setspecific`,
  `__tls_get_addr`, `gettid`, `dl_iterate_phdr`, `_Unwind_*@GCC_*`)
* the usual weak link-editor hooks (`_ITM_*`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__gmon_start__`)

**There are no unresolved project-level symbols.**

**Gate: 0 missing symbols, 0 undefined non-libc symbols. PASS.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one (`--no-default-features` is equivalent
and also builds/tests clean — verified by
`run_tests.sh`). Phases B and C therefore cover every feature
combination that exists.

## Harness self-validation (negative controls)

Passing differential tests only mean something if the harness can actually see
a divergence. Three bugs were injected into `src/lib.rs`, rebuilt, and the
suite re-run; each was caught, and the original file was restored afterwards:

| injected bug | detected by |
|--------------|-------------|
| `start > len` done as a **signed** comparison instead of C's `int`→`size_t` promotion (so a negative start is accepted) | `phase_b_valid` aborted with SIGSEGV on the resulting out-of-bounds read |
| `stop <= start` weakened to `stop < start` (off-by-one; an empty explicit slice wrongly accepted) | `c13_full_cross_product_fuzz`, `c14_sequential_calls_stream_order`, `c15_aliased_index_pointers`, `exhaustive_small_domain` |
| the two `stop` checks swapped, so `stop <= start` runs before `stop > len` | `e5_stop_negative_precedence`, `e6_stop_int_extremes`, `generic_boundary_sweep` |

A **staleness guard** was also added to the harness
(`tests/common/mod.rs::assert_not_stale`) after discovering that `cargo test`
alone does **not** emit the `cdylib` — without it the suite silently tested a
previously built `.so`. The harness now refuses to run if the `.so` it would
load is older than anything in `src/`.
