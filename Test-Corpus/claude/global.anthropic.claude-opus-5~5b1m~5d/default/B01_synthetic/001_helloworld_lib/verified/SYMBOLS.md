# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libhello.so

cd translation && cargo build --release --offline
# -> translation/target/release/libhello.so
```

## C source inventory

The whole library is two files (`c_src/CMakeLists.txt` compiles exactly one
translation unit, `src/hello.c`):

| C file | translated to | status |
|--------|---------------|--------|
| `c_src/src/hello.c` | `translation/src/hello.rs` | translated |
| `c_src/include/hello.h` (public header) | — (declares the single symbol below) | covered |

No C source file is missing from the translation, so no Phase-A "translate the
skipped module" work applies.

## Exported (defined) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | C decl | Rust definition |
|---|--------|---------|-----------|--------|-----------------|
| 1 | `helloworld` | `T` (0x1109) | `T` (0x11c70) | `int helloworld();` — `c_src/include/hello.h:27` | `#[unsafe(no_mangle)] pub extern "C" fn helloworld() -> c_int` — `translation/src/hello.rs:25` |

There are no macro-generated symbols, no exported data symbols, no versioned
symbols, and no aliases in the C `.so`.

## Symbol diff

```
C defined symbols   : 1
Rust defined symbols: 1

comm -23 c_syms rust_syms   # in C, missing from Rust
(empty)

comm -13 c_syms rust_syms   # extra in Rust
(empty)
```

**Result: 0 missing symbols. Symbol parity is exact.**

## Undefined (imported) symbols

The C `.so` imports one non-weak libc symbol: `puts@GLIBC_2.2.5`.
(The C compiler lowers `printf("Hello World!\n")` to `puts("Hello World!")`;
`puts` appends the newline, so the emitted bytes are `Hello World!\n`.)

The Rust `.so` imports `puts@GLIBC_2.2.5` as well — rustc/LLVM performs the same
`printf` → `puts` lowering on the `c_printf` call in `hello.rs`. Its remaining
imports are all libc / libgcc runtime symbols pulled in by the Rust standard
library (`malloc`, `memcpy`, `write`, `_Unwind_*`, `dl_iterate_phdr`, …).

**Result: 0 undefined non-libc / non-libgcc symbols in the Rust `.so`.**
Verified with `ldd`: the Rust `.so` needs only `libc.so.6` and `libgcc_s.so.1`.

## ⚠ Build pitfall that invalidates naive test runs

`cargo test` **does not build a `cdylib`-only library target.** Only
`cargo build` produces `<target>/<profile>/libhello.so`. A bare `cargo test`
therefore runs every differential test against whatever `.so` an earlier build
left behind, so source changes — including regressions — are silently not under
test. This was observed here: an early mutation run had *all* mutants survive
purely because the `.so` was stale.

Two safeguards are in place:

* `tests/common/mod.rs::assert_so_is_fresh` compares the `.so`'s mtime against
  every `src/**/*.rs` and `Cargo.toml`, and fails loudly with a `STALE ARTIFACT`
  message rather than passing vacuously.
* `verify.sh` runs the matching `cargo build` before every `cargo test`.

A second, related requirement: the tests capture output by redirecting the
process-wide fd 1, and libtest writes its own progress lines there too, so the
suite must run serially. `.cargo/config.toml` sets `RUST_TEST_THREADS=1`, and
`assert_serial_execution` refuses to run (with an explanatory message) if that
is not in effect.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one configuration (the default). `--no-default-features` and any
`--features <combo>` reduce to the same single build. The
`tests/feature_matrix.sh` helper enumerates the (single) combination and runs
the full test suite under it, so the Phase-D "every feature combination"
requirement is satisfied by construction.
