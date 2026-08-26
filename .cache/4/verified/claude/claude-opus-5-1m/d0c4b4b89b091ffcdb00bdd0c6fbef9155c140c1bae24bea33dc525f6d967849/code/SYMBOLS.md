# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libarrayfunc_lib.so
```

* C shared library: `c_src/build/libtranslated_rust.so`
  (CMake names the target after the *parent* directory, hence the
  `translated_rust` name for the **C** library.)
* Rust shared library: `target/debug/libarrayfunc_lib.so`
  (`[lib] name = "arrayfunc_lib"`, `crate-type = ["cdylib"]`)

## Translation unit inventory

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source | translated to | status |
|----------|---------------|--------|
| `c_src/src/lib.c` | `src/lib.rs` | fully translated (11/11 non-`static` functions) |

`c_src/include/lib.h` declares only `arrayfunc`; the other ten functions have
external linkage but no public prototype. All eleven are exported from the C
`.so` and therefore all eleven must be exported from the Rust `.so`.

## Exported symbol parity

| # | C symbol (`nm -D`, type `T`) | signature | in Rust `.so` | Rust item |
|---|------------------------------|-----------|---------------|-----------|
| 1 | `add_operation`            | `int (int,int,int,int)`             | YES | `add_operation` |
| 2 | `multiply_operation`       | `int (int,int,int,int)`             | YES | `multiply_operation` |
| 3 | `subtract_operation`       | `int (int,int,int,int)`             | YES | `subtract_operation` |
| 4 | `modulo_operation`         | `int (int,int,int,int)`             | YES | `modulo_operation` |
| 5 | `safe_double_to_int`       | `int (double)`                     | YES | `safe_double_to_int` |
| 6 | `compute_scaled_value`     | `int (int,double)`                 | YES | `compute_scaled_value` |
| 7 | `compare_results_in_array` | `int (ResultArray*,int,int)`       | YES | `compare_results_in_array` |
| 8 | `init_result_array`        | `void (ResultArray*,int[],int)`    | YES | `init_result_array` |
| 9 | `process_with_foreach`     | `int (ResultArray*,operation_func)`| YES | `process_with_foreach` |
| 10 | `compute_weighted_sum`    | `int (ResultArray*)`               | YES | `compute_weighted_sum` |
| 11 | `arrayfunc`               | `int (int,int,int,int)`            | YES | `arrayfunc` |

**Missing from Rust: 0. Extra stubs / `unimplemented!()`: 0.**

Measured export counts (`nm -D --defined-only | awk '$2 ~ /^[A-Z]$/'`):
**C = 11, Rust = 11** — a `cdylib` hides Rust-internal symbols, so the two
dynamic symbol tables are exactly the same set. `comm -23` of the two sorted
lists is empty. The diff is enforced by
`tests/phase_d_parity.rs::phase_d_symbol_parity` (which also asserts the C `.so`
exports nothing this file fails to list) and, independently of `nm`, by
`phase_d_symbol_parity_via_dlsym`, which resolves all 11 names out of the Rust
`.so` and calls every one of them.

## Undefined (imported) symbols

C `.so` weak/undefined:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

All are toolchain-generated. The C library imports **no** `libm`/`libc` function
(`math.h` is included but never used), so the Rust translation needs no external
dependency either — it uses only core arithmetic and `f64 as i32` casts.

The Rust `.so` imports only `libc`/`libgcc` symbols supplied by `std`. There are
**0 missing/undefined non-libc symbols** in the Rust `.so`
(verified with `ldd -r target/debug/libarrayfunc_lib.so`).

## Type layout parity (checked by `phase_d_layout_parity`)

| C type | size | align | field offsets | Rust `#[repr(C)]` equivalent |
|--------|------|-------|---------------|------------------------------|
| `Result`      | 24  | 8 | `value` 0, `scaled` 8, `rank` 16 (padding 20..24) | `Result` |
| `ResultArray` | 248 | 8 | `data` 0..240, `count` 240 (padding 244..248)      | `ResultArray` |

## Build configurations

`Cargo.toml` has **no `[features]` table**, and `src/lib.rs` contains no
`#[cfg(feature = …)]`. `c_src/CMakeLists.txt` has no options, `#ifdef` switches
or conditional sources. Therefore the complete set of valid feature
combinations is a single one: the empty set.

| # | combination | command |
|---|-------------|---------|
| 1 | *(none — default == no-default)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`verify_all.sh` derives this list mechanically from `Cargo.toml` (power set of
`[features]` minus `default`) and runs `cargo check --all-targets`, `cargo build`
and `cargo test` for each entry, so it keeps working if features are ever added.

## Harness note: `cargo test` does not build `cdylib` targets

A `crate-type = ["cdylib"]` artifact is **not** produced by `cargo test` — only by
`cargo build`. A `libloading`-based differential suite therefore risks silently
`dlopen`-ing a stale `.so` left behind by an earlier `cargo build`, reporting
success for code that is no longer in `src/`. This was observed here: three
deliberately broken versions of `src/lib.rs` all "passed" before the harness was
fixed.

`tests/common/mod.rs::rust_so_path()` now guards against it:

1. it uses `target/<profile>/libarrayfunc_lib.so` only if that file is at least
   as new as `Cargo.toml` and every `src/**/*.rs`;
2. otherwise it shells out to `cargo build --lib` with a private
   `CARGO_TARGET_DIR` (`target/dylib-under-test`, which avoids contending for the
   build lock the outer `cargo test` holds) and loads that guaranteed-fresh copy;
3. if neither is possible it panics rather than testing a stale artifact.

### Mutation-testing evidence

The suite was validated by breaking `src/lib.rs` one change at a time and
confirming the tests fail:

| mutation | detected by |
|----------|-------------|
| `process_with_foreach`: `* 0.75` -> `* 0.7500000001` | 5 tests (C21-C25) |
| `compare_results_in_array`: `idx >= count` -> `idx > count` | 2 tests (C15, C16) |
| `compute_weighted_sum`: index-0 weight `1` -> `i` (i.e. "fixing" the quirk) | 4 tests (C17-C20) |
| `safe_double_to_int`: high clamp -> `INT_MAX - 1` | 8 tests (C5, C6, E2, E5, E23, …) |
| `init_result_array`: clamp `10` -> `9` | 8 tests (C10, C11, C13, E13, layout parity) |
| `FOREACH`: `count_iter != size` -> `count_iter < size` | 1 test (E18 — only the negative-`count` runaway distinguishes these) |
| reverting the raw-pointer field reads to `(*arr).field` | 2 tests (E28 — SIGSEGV vs SIGABRT) |

Two further mutations were confirmed *semantically equivalent* rather than
undetected: `d >= INT32_MAX` -> `d > INT32_MAX` (Rust's `as` cast saturates to the
same value) and `count < 10` -> `count <= 10` (identical at `count == 10`).
