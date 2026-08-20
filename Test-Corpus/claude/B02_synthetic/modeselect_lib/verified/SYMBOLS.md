# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so | grep -E ' [TDBRG] '

# Rust
cargo build
nm -D --defined-only target/debug/libmodeselect_lib.so | grep -E ' [TDBRG] ' | grep -v '_ZN'
```

## C `.so` exported (defined, global) symbols

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `apply_multiplier`          | T (text) | YES |
| 2 | `classify_mode`             | T (text) | YES |
| 3 | `convert_negative_overflow` | T (text) | YES |
| 4 | `convert_time_factor`       | T (text) | YES |
| 5 | `get_modified_time`         | T (text) | YES |
| 6 | `hash_time_value`           | T (text) | YES |
| 7 | `modeselect`                | T (text) | YES |

The C `.so` defines **no** exported data objects and no macro-generated
symbols. `c_src/src/lib.c` is the only translation unit and every one of its
seven non-`static` functions is listed above, so the surface is complete.

Note: only `modeselect` is declared in the public header
(`c_src/include/lib.h`), but the other six functions have external linkage and
are therefore reachable through `dlsym`. They are all treated as public API and
tested directly.

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC_2.2.5`, `strcmp@GLIBC_2.2.5`,
`time@GLIBC_2.2.5` (plus the usual weak `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`).

The Rust `.so` imports the same three libc functions (it deliberately calls
glibc `printf`/`strcmp`/`time` so formatting and string comparison are
byte-identical) plus the standard Rust runtime's libc/pthread dependencies.

## Result

**0 missing symbols.** Every symbol exported by the C `.so` is exported by the
Rust `.so` under the exact same name. No stubs were added; every export is a
real translation of the corresponding C function.

Verification helper: `scripts/symbol_diff.sh` (prints nothing on success).

## Verification of the artifact under test (important)

`cargo test` does **not** rebuild a `cdylib`-only library target, so
`target/<profile>/libmodeselect_lib.so` can be arbitrarily stale while the tests
happily report success. This was found by mutation testing: ten deliberately
injected bugs in `src/lib.rs` all "passed". The suite therefore builds the
libraries from `build.rs` (see `CONFIGS.md` rows 75–78) and the tests load those
paths. `scripts/symbol_diff.sh` additionally checks the `cargo build` artifact.

Current state (`./scripts/symbol_diff.sh`):

```
C library:        c_src/build/libtranslated_rust.so (7 public symbols)
OK: target/debug/libmodeselect_lib.so exports all C symbols
OK: .../out/libmodeselect_lib_dbg.so exports all C symbols
OK: .../out/libmodeselect_lib_opt.so exports all C symbols
```

Symbol parity is also asserted from inside the test suite
(`tests/phase_d_parity.rs`): `symbol_parity_c_vs_rust`,
`symbol_parity_optimised_rust_build`, `symbol_parity_cargo_built_artifact`,
`no_unresolved_non_libc_symbols_in_rust_so`,
`every_symbol_resolves_in_both_libraries` — the last two `dlopen` each library
and `dlsym` all seven names, which is the real proof there are no unresolved
non-libc symbols.
