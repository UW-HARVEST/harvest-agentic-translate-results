# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Source of truth: `nm -D` on both shared objects.

* C:    `c_src/build/libdriver.so`            (cmake, `CMAKE_BUILD_TYPE=""` → `-O0`, no extra flags)
* Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

Regenerate with:

```sh
nm -D --defined-only c_src/build/libdriver.so            | awk '$2=="T"{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # MUST be empty
```

`tests/symbols.rs::symbol_parity_c_subset_of_rust` performs exactly this diff
programmatically (it `dlsym`s every C-exported name out of the Rust `.so`), so
the gate is enforced by the test suite, not only by this document.

## Exported (dynamic, `T`) symbols

The C library has exactly one translation unit (`src/goto.c`); nothing else in
`c_src/` is compiled, so there is no un-translated module. All three of its
external definitions are present in the Rust `.so`:

| # | C symbol                | C `.so` | Rust `.so` | Declared in `include/goto.h` | Rust definition (`src/lib.rs`) |
|---|-------------------------|---------|------------|------------------------------|--------------------------------|
| 1 | `driver`                | `T`     | `T`        | yes (`int driver(int, const char*)`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |
| 2 | `forward_goto_example`  | `T`     | `T`        | no (external linkage, not in header) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn forward_goto_example` |
| 3 | `open_with_cleanup`     | `T`     | `T`        | no (external linkage, not in header) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn open_with_cleanup` |

`comm -23` result: **empty** — 0 symbols missing from the Rust `.so`.

No macro-generated symbols exist in this library (no function-defining macros in
`goto.c` / `goto.h`), and there are no exported data objects.

## Rust `.so` extra exports

`nm -D --defined-only` on the Rust `.so` yields exactly 3 `T` symbols — the same
3 names, no extras. (Rust's own runtime symbols such as `rust_eh_personality`
are local, not dynamic.)

## Undefined (imported) symbols

C imports: `fclose fclose ferror fgets fopen fprintf fwrite printf stderr`
(`fwrite` appears because gcc rewrites the constant-format
`fprintf(stderr, "Error: negative input\n")` into `fwrite`; the bytes written are
identical).

Rust imports the same set (`fclose ferror fgets fopen fprintf fwrite printf
stderr`) plus only libc/`libgcc_s` runtime symbols pulled in by `std`
(`malloc`, `memcpy`, `__errno_location`, `_Unwind_*`, `dl_iterate_phdr`, …).

**0 missing / undefined non-libc symbols in the Rust `.so`.** ✔

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the
complete set of feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | default (empty feature set) | `cargo test` |
| 2 | `--no-default-features` (identical to #1 — no `default` feature exists) | `cargo test --no-default-features` |

Both are exercised by `run_all.sh`; the symbol table and every test result are
identical under each because no `#[cfg(feature = ...)]` exists in the crate.

`run_all.sh` enumerates the powerset of `[features]` generically (so the matrix
stays correct if features are added later) and additionally crosses it with the
two build profiles, because `release` and `debug` are genuinely different code
paths here — `[profile.release]` sets `panic = "abort"`, and `debug` enables
arithmetic overflow checks, which matters for the wrapping `x * 2`:

| # | feature combo | profile | symbol parity | tests |
|---|---------------|---------|---------------|-------|
| 1 | default | release | 3/3, 0 missing | 48 passed |
| 2 | default | debug | 3/3, 0 missing | 48 passed |
| 3 | `--no-default-features` | release | 3/3, 0 missing | 48 passed |
| 4 | `--no-default-features` | debug | 3/3, 0 missing | 48 passed |
