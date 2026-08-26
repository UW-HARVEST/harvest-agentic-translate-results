# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

## Surface of the C library

The whole C library is one translation unit:

* `c_src/CMakeLists.txt` → `add_library(translated_rust SHARED src/lib.c)`
  (no other sources, no `-D` defines, no `option()`/`#ifdef` configuration).
* `c_src/include/lib.h` declares exactly one function:
  `int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src);`
* `c_src/src/lib.c` defines exactly that one function. There are no static
  helpers, no global data, no constructors/destructors, and no macro-generated
  symbol families.

So there is **no untranslated C module**: `src/lib.rs` covers 100 % of
`c_src/src/lib.c`.

## `nm -D --defined-only` — C `.so`

Command:

```
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Output (global, defined):

| # | symbol   | type | present in Rust `.so`? |
|---|----------|------|------------------------|
| 1 | `wcscat` | `T`  | **yes** (`T wcscat`)   |

## `nm -D --defined-only` — Rust `.so`

Command:

```
cargo build --offline --no-default-features
nm -D --defined-only target/debug/libwcscat_lib.so
```

Output (global, defined):

| # | symbol   | type | in C `.so`? |
|---|----------|------|-------------|
| 1 | `wcscat` | `T`  | yes         |

Exported via `#[unsafe(no_mangle)] pub unsafe extern "C" fn wcscat(...)` in
`src/lib.rs`.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only target/debug/libwcscat_lib.so   | awk '{print $NF}' | sort -u)
```

Result: **empty** — 0 symbols exported by the C `.so` are missing from the
Rust `.so`. See `tests/symbols.rs`, which re-runs this diff as an automated
test (`c_symbols_are_all_exported_by_rust`).

## Undefined (imported) symbols

`nm -D -u` on the C `.so`:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

`nm -D -u` on the Rust `.so` lists only:

* the same four weak CRT/ITM stubs,
* glibc functions (`malloc`, `free`, `memcpy`, `memset`, `open64`, `read`,
  `write`, `pthread_key_*`, `syscall`, …) pulled in by Rust `std`,
* `_Unwind_*` from `libgcc_s` (Rust `std` panic machinery).

There are **no undefined non-libc / non-libgcc symbols** — nothing that would
indicate a missing Rust implementation. Verified by
`tests/symbols.rs::rust_so_has_no_undefined_non_system_symbols`.

## Target ABI notes

Verified on this host (`gcc`, `x86_64-unknown-linux-gnu`):

* `sizeof(wchar_t) == 4`, and `wchar_t` is **signed** →
  `pub type wchar_t = c_int` (= `i32`) in `src/lib.rs` is correct.
* `size_t` is 64-bit → `usize`.
* The C library is compiled with `C_FLAGS = -fPIC` and an empty
  `CMAKE_BUILD_TYPE`, i.e. **`-O0`**, so pointer arithmetic in the C is
  emitted literally (wrapping `imul`/`lea`), which is what the Rust models
  with `wrapping_add`.

## Feature combinations

`Cargo.toml` declares only `default = []`; the C build system has no
configuration axes at all (no `option()`, no `add_definitions`, no `#ifdef` in
`lib.c` / `lib.h`). The complete set of valid combinations is therefore:

| # | cargo invocation                                        |
|---|---------------------------------------------------------|
| 1 | `cargo test --offline --no-default-features`             |
| 2 | `cargo test --offline --no-default-features --features default` |
| 3 | `cargo test --offline` (implicit default — same as #2)   |

All three are exercised by `run_all_features.sh`.

## How this is verified automatically

`tests/symbols.rs` re-derives everything above at test time:

| test | what it proves |
|------|----------------|
| `both_shared_objects_exist_and_load` | both `.so` files load and expose `wcscat` |
| `c_symbols_are_all_exported_by_rust` | `nm -D` diff C → Rust is empty |
| `rust_so_has_no_undefined_non_system_symbols` | no missing Rust implementations |
| `c_so_has_no_undefined_non_system_symbols` | baseline for the above |
| `rust_so_exports_no_unexpected_extra_symbols` | no leaked Rust-mangled internals |
| `neither_entry_point_is_glibc_wcscat` | `dlsym` did not silently resolve either side to glibc's unrelated 2-argument `wcscat` |

Mechanical re-check (run from the crate root, after `./run_all_features.sh`):

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only target/difftest/default/debug/libwcscat_lib.so | awk '{print $NF}' | sort -u)
                                    # -> no output: 0 missing symbols
```

## Harness note (important)

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]`-only library,
because integration tests cannot link it and it is therefore not a dependency
of the test targets. Loading `target/debug/libwcscat_lib.so` directly would
silently test a **stale** artifact. `tests/common/mod.rs` therefore builds the
cdylib itself (into `target/difftest/<features>/`) with the same feature set as
the running test binary, and `DIFFTEST_PROFILE=release` selects the optimized,
`panic = "abort"` artifact. This was validated by mutation testing: seven
deliberate defects injected into `src/lib.rs` (`22`→`23`, `34`→`35`, `0`→`1`,
dropping the `numElem == 0` guard, dropping either `dst[0] = 0` store,
reordering the `!dst`/`!src` guards, and an off-by-one in each loop bound) each
made the suite fail, and reverting restored a clean run.
