# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

The C project (`c_src/CMakeLists.txt`) builds **one** translation unit,
`c_src/src/main.c`, as the executable `driver`. For differential testing through
the FFI boundary the very same translation unit is also compiled as a shared
library (no source change, `c_src/` untouched):

```sh
gcc -shared -fPIC -o libcdriver.so c_src/src/main.c        # C .so
cargo build --lib                                          # -> target/<prof>/libdriver.so
```

The Rust crate mirrors this: `src/logic.rs` holds the translated code,
`src/main.rs` is the executable, `src/lib.rs` is a `cdylib` that re-exports the
same C-ABI symbols.

## `nm -D --defined-only` comparison

Command used (see `tests/common/mod.rs`, which regenerates both libraries, and
`tests/symbol_parity.rs`, which asserts the diff is empty):

```sh
nm -D --defined-only <lib> | awk '{print $2" "$3}'
```

| # | C symbol (`libcdriver.so`) | type | present in Rust `libdriver.so` | Rust definition |
|---|----------------------------|------|--------------------------------|-----------------|
| 1 | `static_sum`               | `T`  | **yes** (`T static_sum`)       | `#[no_mangle] pub extern "C" fn static_sum` in `src/lib.rs` |
| 2 | `main`                     | `T`  | **yes** (`T main`)             | `#[no_mangle] pub unsafe extern "C" fn main` in `src/lib.rs` |

Raw output (verbatim):

```
$ nm -D libcdriver.so                        $ nm -D --defined-only libdriver.so
                 w _ITM_deregisterTMCloneTable   00000000000127f0 T main
                 w _ITM_registerTMCloneTable     0000000000012d30 T static_sum
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001159 T main
                 U printf@GLIBC_2.2.5
                 U puts@GLIBC_2.2.5
0000000000001139 T static_sum
                 U strtol@GLIBC_2.2.5
```

(`puts` appears because gcc rewrites `printf("...\n")` with a constant,
newline-terminated format string into `puts`; it is a libc import, not API.)

* Symbols only in C, missing from Rust: **none** (0).
* The C `.so` defines no other non-weak global symbols: the remaining `nm -D`
  entries are the libc imports `printf`, `puts`, `strtol` (`U`) and the weak
  (`w`) toolchain hooks `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`, which are
  linker/compiler artefacts rather than API.
* The Rust `.so` exports exactly those two symbols and nothing else
  (`nm -D --defined-only | wc -l` == 2), so there are no extra/renamed exports
  either.
* Nothing is stubbed: both exports call the real translated code in
  `src/logic.rs`.

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u target/<prof>/libdriver.so` lists only libc/ld.so imports (`write`,
`memcpy`, pthread/dl stubs pulled in by `std`, …). There are **0 unresolved
non-libc symbols**: `ldd -r target/<prof>/libdriver.so` prints no "undefined
symbol" line, which `tests/symbol_parity.rs::rust_so_has_no_unresolved_symbols`
asserts automatically.

## Automated checks

| check | test |
|-------|------|
| every C export is exported by the Rust `.so` (set difference empty) | `tests/symbol_parity.rs::rust_so_exports_every_c_symbol` |
| the C export set is still exactly `{main, static_sum}` (fails if the C grows API) | same test |
| no unresolved symbols in the Rust `.so` | `tests/symbol_parity.rs::rust_so_has_no_unresolved_symbols` |
| `dlsym` resolves both names in both libraries | `tests/symbol_parity.rs::both_libraries_resolve_the_documented_entry_points` |
| symbol diff for every feature combination × profile | `run_diff_tests.sh` (`nm -D` + `comm -23`) |

## Feature / configuration matrix

`Cargo.toml` declares **no `[features]` table**, therefore the only valid
feature combination is the (empty) default one:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo check/build/test --no-default-features` | identical to the default build; there are no optional features and no `cfg(feature = …)` in the crate |

`c_src/CMakeLists.txt` likewise defines no options, no `#ifdef`-driven variants
and no `CMAKE_BUILD_TYPE`-dependent code (the single target is
`add_executable(driver src/main.c)`), so there is exactly one C configuration
too. `src/main.rs` contains the only `cfg` in the crate (`cfg(unix)` /
`cfg(not(unix))` for `OsStr` bytes and for restoring the default `SIGPIPE`
disposition), which is target-driven, not feature-driven.
