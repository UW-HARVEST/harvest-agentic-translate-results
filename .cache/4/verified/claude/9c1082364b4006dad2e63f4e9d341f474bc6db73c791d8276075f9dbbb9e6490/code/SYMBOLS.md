# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this was produced

The project's own build (`c_src/CMakeLists.txt`) links `src/main.c` into an
**executable** (`driver`), so a shared-library flavour of the very same
translation unit is produced with the same compiler flags CMake uses when no
`CMAKE_BUILD_TYPE` is set (`-fPIC`, no `-O`), plus an `-O2` variant:

```
cc -shared -fPIC -O0 -o c_src/build/libcref.so    c_src/src/main.c
cc -shared -fPIC -O2 -o c_src/build/libcref_o2.so c_src/src/main.c
```

`build.rs` performs exactly these two invocations on every `cargo build` /
`cargo test` and exports the resulting paths to the tests as `C_REF_SO` and
`C_REF_SO_O2`. Nothing under `c_src/` is modified.

The Rust side is built as a `cdylib` (`[lib] crate-type = ["cdylib"]`) →
`target/debug/libfma_array.so`.

Symbol lists come from `nm -D --defined-only <so>`; the comparison is
automated by `tests/symbol_parity.rs` (and `scripts/symdiff.sh`).

## C `.so` exported symbols vs Rust `.so`

`nm -D --defined-only c_src/build/libcref.so`, ignoring the reserved/toolchain
names that begin with `_` (`_init`, `_fini`, `__bss_start`, `_edata`, `_end`,
`_IO_stdin_used`):

| # | C symbol | type | C declaration | present in Rust `.so`? | Rust definition |
|---|----------|------|---------------|------------------------|-----------------|
| 1 | `fma_array` | `T` (global text) | `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)` | YES | `#[no_mangle] pub unsafe extern "C" fn fma_array` in `src/lib.rs` |
| 2 | `call_fma`  | `T` (global text) | `int call_fma(const int *data, int len)` | YES | `#[no_mangle] pub unsafe extern "C" fn call_fma` in `src/lib.rs` |
| 3 | `main`      | `T` (global text) | `int main(void)` | YES | `#[no_mangle] pub extern "C" fn main` in `src/lib.rs` |

**Missing from the Rust `.so`: none.**

No C source file was left untranslated: `c_src/src/main.c` is the only
translation unit in the project (`find c_src -name '*.c' -o -name '*.h'`
returns just that one file), and all three of its externally visible functions
are translated in full — `fma_array_raw`, `call_fma_raw` and
`main_body`/`main_stdio` in `src/fma.rs`. There are no stubs, no
`unimplemented!()`, and no `todo!()` anywhere in `src/`.

### Reserved symbols intentionally not mirrored

These are emitted by the linker / CRT, not by the program, and are not part of
the library's API surface. The Rust `cdylib` emits its own equivalents.

`_init`, `_fini`, `__bss_start`, `_edata`, `_end`, `_IO_stdin_used`.

## Undefined (imported) symbols

C `.so` (`nm -D -u`): `__isoc99_scanf@GLIBC_2.7`, `printf@GLIBC_2.2.5`,
`memset@GLIBC_2.2.5`, plus the weak `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`.

Rust `.so` (`nm -D -u`): glibc imports plus the eleven libgcc stack-unwinder
symbols (`_Unwind_RaiseException@GCC_3.0`, `_Unwind_Resume@GCC_3.0`,
`_Unwind_Backtrace@GCC_3.3`, `_Unwind_GetIP@GCC_3.0`,
`_Unwind_GetIPInfo@GCC_4.2.0`, `_Unwind_SetIP@GCC_3.0`,
`_Unwind_SetGR@GCC_3.0`, `_Unwind_GetRegionStart@GCC_3.0`,
`_Unwind_GetLanguageSpecificData@GCC_3.0`, `_Unwind_GetDataRelBase@GCC_3.0`,
`_Unwind_GetTextRelBase@GCC_3.0`). These come from `libgcc_s.so.1`, which the
Rust `.so` links against, so there are **0 unresolved non-libc/non-libgcc
symbols**; `dlopen` of the Rust `.so` with `RTLD_NOW` succeeds, which the
tests assert.

## Feature combinations

`Cargo.toml` declares **no `[features]` table**, so the crate has exactly one
build configuration and the set of valid feature combinations is:

| # | combination | command |
|---|-------------|---------|
| 1 | *(none — default == no-default-features)* | `cargo check --no-default-features --all-targets` |
| 1 | *(none — default)* | `cargo check --all-targets` |

`c_src/CMakeLists.txt` defines no `option()`, no `add_definitions`, no
`target_compile_definitions` and no `CMAKE_BUILD_TYPE`, and `main.c` contains
no `#ifdef`/`#if` at all — so there is likewise a single C configuration. Both
`cargo check` invocations above pass with **zero warnings** (and `cargo clippy
--all-targets` is clean), and the full test suite is run under both, in both the
`dev` and the `release` profile, by `scripts/verify_all.sh`.

`scripts/verify_all.sh` enumerates the feature combinations *from `Cargo.toml`
itself* (full power set of the `[features]` table), so adding a feature later
automatically widens the sweep instead of silently skipping it.

## Crate layout changes needed to make the surface comparable

The translation originally produced only `src/main.rs` (a `[[bin]]`), which
exports **no** C symbols at all, so nothing could be compared through the FFI
boundary. The crate is now:

| file | role |
|------|------|
| `src/fma.rs` | the translation itself (`fma_array_raw`, `call_fma_raw`, the `scanf("%d")` scanner, `main_body`) — compiled into **both** targets below, so the binary and the library can never drift apart |
| `src/lib.rs` | `[lib] crate-type = ["cdylib"]`; nothing but the three `#[no_mangle] extern "C"` wrappers |
| `src/main.rs` | `[[bin]] driver`, the counterpart of the CMake `driver` executable; pulls in `fma.rs` with `#[path]` rather than through the library crate, so its entry point cannot collide with the library's exported `main` |

`[lib] test = false, bench = false` is required: the library exports a
`#[no_mangle] main` (because the C `.so` does), which would otherwise collide
with the entry point libtest generates for a lib unit-test harness
(`error: entry symbol main declared multiple times`).

Because the integration tests reach the library only through `dlopen`, cargo has
no dependency edge from the tests to the `cdylib` and will **not** rebuild it
when `src/` changes. `tests/common/mod.rs::rust_so()` therefore hard-fails on a
stale `.so` (mtime comparison against every `.rs` file plus `Cargo.toml` and
`build.rs`) so that a stale artifact can never masquerade as a passing run.

## Tests enforcing this file

| test (`tests/symbol_parity.rs`) | what it enforces |
|---|---|
| `sym_every_c_symbol_is_exported_by_rust` | the symbol diff against **both** C variants (`-O0`, `-O2`) is empty |
| `sym_expected_symbol_set` | the C's API surface is exactly `{call_fma, fma_array, main}` and the Rust's is identical — so a future C addition that goes untranslated fails loudly instead of silently shrinking the comparison |
| `sym_no_unresolved_symbols_in_rust_so` | `dlopen(RTLD_NOW)` of the Rust `.so` succeeds (i.e. nothing is unresolved) and all three symbols `dlsym` |
| `sym_all_symbols_resolve_out_of_process_in_both` | the same resolution check inside a fresh child process, for both libraries, with identical output |

`scripts/symdiff.sh` performs the same diff from the shell (used by
`scripts/verify_all.sh` for the `dev` and `release` `.so`s).

## Status

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (3/3 C symbols exported
      with byte-identical names), for both the `-O0` and the `-O2` C build, and
      for both the `dev` and the `release` Rust build.
- [x] `nm -D` shows 0 undefined non-libc/non-libgcc symbols in the Rust `.so`.
- [x] No C source went untranslated: `c_src/src/main.c` is the only translation
      unit and all three of its functions are fully translated (no stubs).
