# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the built shared libraries.

Build commands:

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust (only one valid feature combination; see "Feature combinations" below)
cargo build --no-default-features
# -> target/debug/libdriver.so
```

## Exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | C symbol | type | present in Rust `.so`? | Rust source |
|---|----------|------|------------------------|-------------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_float)` |

**Missing symbols: 0.** The C `.so` exports exactly one non-libc, non-toolchain
symbol (`driver`) and the Rust `.so` exports it with the identical name.

### Deliberately NOT exported (parity requirement in the other direction)

| C declaration | linkage | exported by C `.so`? | exported by Rust `.so`? |
|---------------|---------|----------------------|-------------------------|
| `static void print_hex(unsigned char *p, int len)` | `static` (internal) | NO | NO (plain `unsafe fn print_hex`, no `#[no_mangle]`) |

`print_hex` is `static` in `c_src/src/driver.c`, so it is not part of the ABI.
The Rust translation keeps it private, so `dlsym` fails for it in BOTH libraries.
This is asserted as a differential test (`ERRORS.md` row 8), because exporting it
from the Rust side would itself be a symbol-surface divergence.

## Toolchain / libc symbols (not part of the comparison)

Both libraries additionally reference undefined libc symbols. These are resolved
by the dynamic loader and are not part of the exported API surface:

* C `.so` undefined: `printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5`,
  `__cxa_finalize`, `__gmon_start__`, `_ITM_{de,}registerTMCloneTable` (weak).
* Rust `.so` undefined: the same `printf@GLIBC_2.2.5` and `putchar@GLIBC_2.2.5`,
  plus the standard `std`/`libstd` support set (`_Unwind_*`, `malloc`, `free`,
  `memcpy`, `mmap64`, `pthread_key_create`, ...). All are libc/`libgcc_s`
  symbols, i.e. **0 missing/undefined non-libc symbols**.

Note that the Rust translation intentionally calls libc `printf`/`putchar`
rather than `std::io::stdout`, so both libraries write into the *same* glibc
`stdout` FILE object with the same buffering. That is what makes the byte-level
differential comparison in `tests/` meaningful.

`gcc` lowers the C `printf("\n")` call to `putchar('\n')` even at the default
(unoptimized) CMake setting — visible in the C `.so`'s undefined-symbol list
above — and the Rust translation calls `putchar` for that same statement, so the
emitted byte streams and the libc call sequences match.

## Feature combinations

`Cargo.toml` has **no `[features]` section**, so `default` is empty and there is
exactly **one** valid feature combination: the empty set.

`c_src/CMakeLists.txt` defines no `option()`, no `target_compile_definitions`,
and no `#ifdef`-driven variants (its only flag is `-fno-strict-aliasing`, which
has no observable API effect). There are therefore no build-time configurations
on the C side either.

| # | feature combination | profile | `cargo check --all-targets` | `cargo test` |
|---|---------------------|---------|----------------------------|--------------|
| 1 | (empty — the only valid combo) | dev | PASS (no errors, no warnings) | PASS — 40 differential + 2 symbol-parity |
| 1 | (empty — the only valid combo) | release | PASS | PASS — 40 differential + 2 symbol-parity |

Because the single combination is also the default, `cargo check` and
`cargo check --no-default-features` are equivalent here; both are run by
`run_all_features.sh`, which enumerates the power set of `[features]` from
`Cargo.toml` so the matrix widens automatically if a feature is ever added.

The `release` profile is included deliberately: it is the configuration where
LLVM is free to reorder/reassociate float moves, so it is where a
NaN-canonicalizing translation bug would be most likely to surface. Both profiles
agree with the C reference byte-for-byte.

## Symbol-parity checks are automated

`tests/symbol_parity.rs` shells out to `nm -D --defined-only` on both `.so`s and
asserts, as ordinary tests:

* `phase_d_every_c_symbol_is_exported_by_rust` — the C set minus the Rust set is
  empty (the Phase D gate);
* `phase_d_rust_exports_no_extra_public_api` — the Rust set minus the C set is
  also empty, so the translation does not widen the ABI (in particular the
  `static` helper `print_hex` stays internal).

Both run in every feature/profile combination, and `run_all_features.sh` prints
the raw `nm -D` output plus the `diff` of the two symbol sets at the end of the
run for manual inspection.

## Verification status

| gate | result |
|------|--------|
| `nm -D`: symbols missing from the Rust `.so` | 0 |
| `nm -D`: extra public symbols in the Rust `.so` | 0 |
| undefined non-libc / non-toolchain symbols in the Rust `.so` | 0 |
| `CONFIGS.md` rows passing (Phase B) | 23 / 23 |
| `ERRORS.md` rows passing (Phase C) | 16 / 16 |
| feature combinations verified (Phase D) | 1 / 1, in both dev and release |
| exhaustive sweep of the entire 2^32 input domain | PASS in both dev and release |

The test suite was mutation-checked to confirm it is not vacuous: five separate
injected defects in `src/lib.rs` — reversed byte order (`to_be_bytes`), NaN
canonicalization, uppercase `%02X`, sign-extending the byte instead of
zero-extending it, and a missing trailing newline — were each detected by the
suite (34, 15, 32, 32 and 35 failing tests respectively). Two harness self-checks
(`harness_00_capture_mechanism_is_clean`, `harness_01_two_distinct_libraries_loaded`)
guard against the comparison silently degenerating, and `assert_fresh` refuses to
run against a stale `.so`.
