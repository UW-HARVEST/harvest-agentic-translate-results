# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported (dynamic, defined) symbols

| # | symbol | C type | source of truth | present in Rust `.so`? |
|---|--------|--------|-----------------|------------------------|
| 1 | `driver` | `T` (global text) | `c_src/src/driver.c:63` — `void driver(int x)`; declared in `c_src/include/driver.h:26` | YES |
| 2 | `run`    | `T` (global text) | `c_src/src/driver.c:52` — `void run(int extra_bedrooms)`; **not** declared in the public header, but has external linkage and is therefore part of the ABI surface | YES |

Missing from Rust `.so`: **none**. No translation gaps, no export wrappers to add,
no stubs.

## C symbols that are deliberately NOT exported by either `.so`

These are `static` (internal linkage) in the C translation unit, so they appear
in `nm` but never in `nm -D`. The Rust translation correctly keeps them private
(`fn` / `static` without `#[no_mangle]`), so the dynamic surfaces match.

| symbol | `nm` class in C | Rust counterpart |
|--------|-----------------|------------------|
| `add_bedrooms`           | `t` | `unsafe fn add_bedrooms` (private) |
| `add_floor`              | `t` | `unsafe fn add_floor` (private) |
| `add_floor_to_the_house` | `t` | `unsafe fn add_floor_to_the_house` (private) |
| `print_the_house`        | `t` | `unsafe fn print_the_house` (private) |
| `the_house`              | `d` | `static THE_HOUSE: Global` (private) |

Note: the C `house_t` struct is a file-local `typedef` and never crosses the ABI
boundary (both public functions take a single `int`), so no `#[repr(C)]` layout
compatibility is observable externally. The Rust translation marks it `#[repr(C)]`
anyway.

## Undefined / imported symbols

| `.so` | non-libc undefined symbols |
|-------|----------------------------|
| C     | none (`printf@GLIBC_2.2.5`, `__cxa_finalize@GLIBC_2.2.5`, plus weak `__gmon_start__`, `_ITM_*` — all libc/toolchain) |
| Rust  | none (libc + Rust `std` internals only) |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(empty)
```

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one. There are no `#[cfg(feature = ...)]` gates in
`src/lib.rs` and no `#ifdef`-gated code in the C source other than the
`DRIVER_H_` include guard. Phase D's "every feature combination" therefore
collapses to a single combination, which is verified explicitly by
`translation/check_all_features.sh`.

## How to reproduce

```
cd translation && ./run_verification.sh
```

That script builds the C `.so` with CMake, runs `cargo check`, builds the Rust
`cdylib` in release mode (the artifact the tests `dlopen`), diffs the exported
symbol sets, and then runs the whole differential suite under every feature
combination via `check_all_features.sh`.

## Test layout

| test binary | phase | covers |
|-------------|-------|--------|
| `tests/symbol_parity.rs` | D | `nm -D` set equality both directions + `dlsym` resolvability of every C symbol in both `.so` files |
| `tests/valid_paths.rs` | B | one test per `CONFIGS.md` row (17) |
| `tests/error_paths.rs` | C | one test per `ERRORS.md` row (12 tests covering rows 1–12; rows 13/14 are unreachable and documented; row 15 lives in `printf_format.rs`) |
| `tests/printf_format.rs` | B/C | `ERRORS.md` row 15 — `%d` and `%.1f` field-width growth from pristine state |
| `tests/independence.rs` | — | harness integrity: proves the two `.so` files are distinct implementations with independent `static house_t the_house` state, so no differential assertion can pass vacuously |
| `tests/harness/mod.rs` | — | shared harness: `dlopen`s both `.so` files, captures fd 1 per call, drives C then Rust in lock-step under a global mutex |

Both public functions return `void` and report exclusively through `printf`, so
"comparing outputs" means capturing the bytes each `.so` writes to file
descriptor 1 and comparing them byte-for-byte. That redirection is
process-global, so the suite requires single-threaded execution;
`translation/.cargo/config.toml` sets `RUST_TEST_THREADS=1`, and the harness
refuses to run (with instructions) if the effective `--test-threads` is not 1
rather than failing flakily.

## Negative controls performed

The suite was validated by injecting faults into `src/lib.rs`, rebuilding the
release `cdylib`, and confirming the tests fail — then reverting:

| injected fault | detected by |
|----------------|-------------|
| `wrapping_add` → `saturating_add` in `add_bedrooms` | 11 of 12 `error_paths` tests |
| `driver` calls `run` once instead of twice | 11 of 17 `valid_paths` tests |
| `#[unsafe(no_mangle)]` removed from `run` | `symbols_01`, `symbols_04` |
| glibc `printf("%.1f")` → Rust `format!("{:.1}")` | **not** detected — and correctly so: `bathrooms` is always an exactly representable `n + 0.5` for every reachable state, so the two formatters agree on the entire reachable value set. This is a genuine equivalence, not a gap. |

`src/lib.rs` was restored byte-identically after each control (verified with
`diff`), and no file under `c_src/` was modified (verified with `md5sum`).
