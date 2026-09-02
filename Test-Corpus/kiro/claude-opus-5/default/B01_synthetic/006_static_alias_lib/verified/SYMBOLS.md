# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libStaticAlias.so                (cmake, gcc, no -O flags)
RUST: translation/target/release/libStaticAlias.so (cdylib)
```

## C `.so` — defined dynamic symbols

`nm -D --defined-only c_src/build/libStaticAlias.so`

```
0000000000001168 T driver
0000000000001119 T static_alias
```

That is the complete public surface: the C library is a single translation unit
(`src/staticalias.c`) with exactly two external definitions, both declared in
`include/staticalias.h`:

```c
int *static_alias(int *outer);
void driver(int initial_value, int iterations);
```

There are no macro-generated symbols, no exported data objects (`inner` is a
function-local `static`, so it has internal linkage and is not exported), and no
`#ifdef`-gated alternative definitions.

## Rust `.so` — defined dynamic symbols

`nm -D --defined-only translation/target/release/libStaticAlias.so`

```
0000000000011740 T driver
00000000000117c0 T static_alias
```

## Parity table

| # | C symbol | type | Rust exports it? | notes |
|---|----------|------|------------------|-------|
| 1 | `static_alias` | `T` (text, global) | YES — `T static_alias` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn static_alias` |
| 2 | `driver`       | `T` (text, global) | YES — `T driver`       | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated — `src/staticalias.c` is the only C source file in
`c_src/CMakeLists.txt`, and both of its external functions are present in the
Rust `cdylib`.

Neither object exports anything beyond those two entries: `nm -D --defined-only`
returns exactly 2 lines for the C `.so` and exactly 2 for the Rust `.so`, so the
dynamic surfaces are not merely compatible but identical. This is enforced by
the `phase_d_symbol_parity` test in `tests/differential.rs` and by
`tests/all_features.sh`.

## Undefined (imported) symbols

`nm -D -u` on the C `.so`:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U printf@GLIBC_2.2.5
```

`nm -D -u` on the Rust `.so` resolves to the same class of symbols — glibc
(`printf`, `malloc`, `memcpy`, `write`, …), the pthread TLS helpers and the
`_Unwind_*` family pulled in by Rust's `std` panic machinery. **0 undefined
non-libc / non-runtime symbols**, i.e. nothing from the library's own surface is
left dangling. Both objects import `printf@GLIBC_2.2.5`, confirming the Rust
translation prints through the *same* libc `stdout` stream as the C original
rather than through `std::io::stdout` (which would buffer separately).

Verification command used (must print nothing):

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libStaticAlias.so     | awk '{print $NF}' | sort -u) \
  <(nm -D --defined-only translation/target/release/libStaticAlias.so | awk '{print $NF}' | sort -u)
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one (empty feature set).
`--no-default-features` and the default build are the same object, so the symbol
table and every Phase B / Phase C result below hold for *all* feature
combinations that exist (see `tests/all_features.sh`).
