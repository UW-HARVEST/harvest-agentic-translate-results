# Phase A.1 — Symbol surface

## Source of truth

C shared library built with:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

`CMakeLists.txt` builds exactly one translation unit (`src/lib.c`) into
`libharvest-work-GusaAW.so` (the project name is derived from the *parent*
directory of `c_src`, so the file name is environment dependent — the tests glob
for `c_src/build/lib*.so`).

Compile flags observed in `CMakeFiles/<target>.dir/flags.make`: `-fPIC` only
(`CMAKE_BUILD_TYPE` is empty ⇒ **no optimisation, no `-DNDEBUG`**).  This matters:
at `-O0` GCC does *not* constant-fold the `(int)` casts of `INFINITY`/`NAN`, it
emits a real `cvttsd2si` (verified by `objdump`, see `cvt.rs`).

Rust shared library: `translation/target/{debug,release}/libdoubleneg_lib.so`
(`[lib] name = "doubleneg_lib"`, `crate-type = ["cdylib"]`).

## Defined (exported) symbol parity

`nm -D --defined-only` on both libraries, sorted:

| # | C symbol | C signature (from `src/lib.c` / `include/lib.h`) | in Rust `.so`? | Rust definition site |
|---|----------|--------------------------------------------------|----------------|----------------------|
| 1 | `convert_double_to_int`  | `int convert_double_to_int(double value)`                                | ✅ | `src/cvt.rs` |
| 2 | `find_value_in_buffer`   | `int find_value_in_buffer(const char *buffer, size_t size, int search_val)` | ✅ | `src/buffer.rs` |
| 3 | `process_negation`       | `int process_negation(int var1)`                                         | ✅ | `src/negation.rs` |
| 4 | `create_numeric_buffer`  | `void create_numeric_buffer(char *buffer, int size, int seed)`           | ✅ | `src/buffer.rs` |
| 5 | `calculate_with_doubles` | `double calculate_with_doubles(int a, int b, int c)`                     | ✅ | `src/dmath.rs` |
| 6 | `doubleneg`              | `int doubleneg(int param1, int param2, int param3, int param4)`          | ✅ | `src/doubleneg.rs` |

There are **no** namespace/renaming/`#define` macros in `include/lib.h` or
`src/lib.c`, so there are no macro-generated linker names to account for; the
source-level names are the linker names one-for-one.

`C \ Rust` symbol difference: **empty**.  (Rust additionally exports the usual
`cdylib` bookkeeping symbols such as `_init`/`_fini`/`__rust_*` allocator shims,
which is expected and harmless — the gate is one-directional: every C symbol
must exist in Rust.)

Verified mechanically by `tests/symbols.rs::c_symbols_are_all_exported_by_rust`,
which shells out to `nm -D --defined-only` on both libraries and asserts the
`C \ Rust` set difference is empty, and additionally `dlsym`s every name through
`libloading` (so a symbol that is present but not dynamically resolvable also
fails).

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on `libdoubleneg_lib.so` lists only libc / libm /
libgcc-unwind imports:

`_Unwind_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`,
`close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `pow`, `printf`, `pthread_key_*`, `pthread_setspecific`,
`puts`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `strlen`,
`syscall`, `write`, `writev` (+ weak `__cxa_finalize`, `__gmon_start__`,
`gettid`, `statx`, `_ITM_*`).

**0 missing / undefined non-libc symbols.**

Two of these imports are deliberate and load-bearing for byte-identical output:

* `printf@GLIBC_2.2.5` — the translation calls the *same* libc `printf` the C
  code calls, so `%e`/`%d`/`%ld` formatting, `-0.0`/`inf`/`nan` spelling and
  stdout buffering are literally the same code.
* `pow@GLIBC_2.29` — the translation calls the *same* libm `pow`, so
  `pow(10.0, c % 10)` and `pow(2.0, 40)` are bit-for-bit identical.
* `puts@GLIBC_2.2.5` — LLVM's `SimplifyLibCalls` rewrites
  `printf("literal\n")` (no conversions) into `puts("literal")`. This is a
  behaviour-preserving libcall optimisation: the bytes written to stdout are
  identical, only the (discarded) return value differs. Confirmed by the
  byte-for-byte stdout comparison in `tests/doubleneg.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
possible configuration is the default one. `run_all_features.sh` extracts the
feature list from `Cargo.toml` and, finding none, runs the cross-product of
`{default, --no-default-features}` × `{release, debug}`:

| # | profile | features | result |
|---|---------|----------|--------|
| 1 | release | default                  | PASS (66 tests) |
| 2 | release | `--no-default-features`  | PASS (66 tests) |
| 3 | debug   | default                  | PASS (66 tests) |
| 4 | debug   | `--no-default-features`  | PASS (66 tests) |

The `debug` rows matter independently of features: `debug` enables
`overflow-checks`, so they prove none of the translated wrapping arithmetic
(`seed + i*7`, `param1 + i*param2`, `INT_MIN % 1000`, `INT_MIN % 256`) panics
where the C silently wraps.

## ⚠ Build pitfall found during verification

`cargo test` **never rebuilds a `cdylib`**. With `crate-type = ["cdylib"]`, a
`cargo test` invocation compiles `src/lib.rs` into a *test-harness binary* and
leaves `target/<profile>/libdoubleneg_lib.so` untouched. A `cargo test` run after
editing `src/` therefore silently validates the **previously built** `.so`.

This was observed live: a deliberately broken `convert_double_to_int` still
"passed" every suite because the loaded `.so` was 12 minutes old.

Two mitigations are in place:

1. `run_all_features.sh` runs `cargo build` before every `cargo test`.
2. `tests/common/mod.rs::assert_libraries_are_fresh` refuses to run if the
   Rust `.so` is older than the newest file in `src/`, or if the C `.so` is
   older than `c_src/src`/`c_src/include`. It reports the exact rebuild command.

## Suite integrity (mutation check)

Passing differential tests only mean something if they can fail.
`mutation_check.py` injects 15 deliberate one-line bugs into the Rust source
one at a time, rebuilds, and re-runs every suite:

* **13 caught.**
* **2 provably semantically equivalent** (uncatchable by construction, with
  proofs recorded in the script): C `%` vs `rem_euclid` before an `as i8`
  truncation (they differ by exactly 256, and 256 ≡ 0 mod 256 — verified
  exhaustively in C over 200 001 values, 0 differences), and passing the `%ld`
  offset as 32-bit (the offset is always `0..=255` and x86-64 32-bit moves
  zero-extend into the varargs register; the translation uses the correct
  `c_long` regardless).
* **0 unexplained survivors.**
