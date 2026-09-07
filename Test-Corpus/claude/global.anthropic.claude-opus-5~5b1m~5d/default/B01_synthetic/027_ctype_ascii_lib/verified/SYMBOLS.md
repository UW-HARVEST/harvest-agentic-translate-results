# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

```
0000000000001149 T driver
```

That is the complete public ABI. `c_src/include/driver.h` declares exactly one
function and `c_src/src/driver.c` is the only translation unit in
`CMakeLists.txt`. There are no macro-generated symbol names, no namespace
prefix macros, no exported globals, and no additional C source files (so there
is no "untranslated module" case here).

## Rust `.so` exported (defined, dynamic) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

```
00000000000129e0 T driver
```

## Symbol parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | T (exported) | T (exported) | ✅ present in both |

## Diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
<empty>
```

**Missing from Rust: 0. Undefined non-libc symbols in Rust: 0.**

The Rust `.so`'s undefined symbols are only libc imports (`printf`,
`setlocale`) plus the usual glibc/`compiler_builtins` runtime entries, which
mirror what the C `.so` imports (`printf`, `setlocale`, plus glibc's
`__ctype_b_loc` / `__ctype_tolower_loc` / `__ctype_toupper_loc`, which the Rust
translation replaces with its own in-crate `"C"`-locale tables in
`src/ctype.rs`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** — there are no
default or optional features, so the only build configuration is the default
one. `cargo build --no-default-features` is therefore equivalent to the default
build. (Verified with `cargo read-manifest`/`cargo metadata`: `"features": {}`.)

## Verification result

`tests/phase_d_symbol_parity.rs` enforces the parity above as a test
(`every_c_symbol_is_exported_by_rust`, `rust_so_has_no_unresolvable_non_libc_symbols`),
and `run_tests.sh` re-checks the raw `nm -D` diff for both the release and dev
cdylib profiles:

```
symbol diff: EMPTY (parity OK)
dev symbol diff: EMPTY (parity OK)
```

Nothing was stubbed and no module was missing: `c_src` contains a single
translation unit whose only public function is fully implemented in Rust
(`src/lib.rs` plus the `"C"`-locale `<ctype.h>` tables in `src/ctype.rs` that
replace glibc's `__ctype_b_loc` / `__ctype_tolower_loc` / `__ctype_toupper_loc`).
