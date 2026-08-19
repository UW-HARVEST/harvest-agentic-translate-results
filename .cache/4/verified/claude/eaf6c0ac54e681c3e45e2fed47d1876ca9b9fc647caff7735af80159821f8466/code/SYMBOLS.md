# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust (only one configuration exists: no [features] in Cargo.toml)
cargo build --no-default-features
# -> target/debug/libdriver.so
```

## `nm -D c_src/build/libdriver.so` (raw)

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001109 T driver
                 U printf@GLIBC_2.2.5
```

## `nm -D --defined-only` comparison

| # | symbol | type | C `.so` | Rust `.so` | status |
|---|--------|------|---------|------------|--------|
| 1 | `driver` | `T` (global text) | yes (`0x1109`) | yes (`0x11e00`) | **MATCH** — exported by `#[no_mangle] pub extern "C" fn driver` |

Toolchain-generated weak symbols (`_ITM_registerTMCloneTable`,
`_ITM_deregisterTMCloneTable`, `__cxa_finalize`, `__gmon_start__`) are present as
weak/undefined in BOTH objects; they are not part of the library API surface.

**Symbols exported by the C `.so` but missing from the Rust `.so`: NONE.**
There is exactly one C translation unit (`c_src/src/driver.c`) and exactly one
public declaration in the only public header (`c_src/include/driver.h`:
`void driver(int x);`). No C module was skipped by the translation, so no
missing source had to be translated and no stub was needed.

## Undefined (imported) symbols

C imports `printf@GLIBC_2.2.5` only.
Rust imports `printf@GLIBC_2.2.5` (the translation deliberately calls C
`printf` so stdout buffering matches byte-for-byte) plus the standard
libc/libgcc set pulled in by `std` and the unwinder
(`_Unwind_*` from `libgcc_s.so.1`, `malloc`, `memcpy`, `write`, ... from
`libc.so.6`).

`ldd` resolves every entry for both objects:

* C: `libc.so.6`
* Rust: `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`

**0 missing / unresolvable non-libc symbols in the Rust `.so`.**

## Verified across every artifact

`nm -D --defined-only | awk '{print $NF}'` yields exactly `driver` for all four
builds — so the export surface is identical in every configuration:

| artifact | defined dynamic symbols |
|---|---|
| `c_src/build/libdriver.so` (gcc, default flags) | `driver` |
| `target/c_o2/libdriver.so` (gcc `-O3`) | `driver` |
| `target/debug/libdriver.so` (Rust, dev) | `driver` |
| `target/release/libdriver.so` (Rust, release) | `driver` |

Enforced as a test by `tests/phase_d_symbols.rs`:

* `sym_01_every_c_symbol_is_exported_by_rust` — set difference C \ Rust is empty
* `sym_02_rust_has_no_unresolvable_undefined_symbols` — no undefined non-libc
  symbol; also proven by `dlopen(RTLD_NOW)` succeeding
* `sym_03_both_libraries_expose_a_distinct_callable_driver` — guards against the
  two `.so`s aliasing to one object
* `sym_04_no_extra_public_c_declarations_were_missed` — fails if `driver.h` ever
  declares anything beyond `void driver(int x);`

## Header ↔ export cross-check

| public header decl | C symbol | Rust symbol |
|---|---|---|
| `void driver(int x);` | `driver` | `driver` |

No other declarations, no macro-generated symbol families, no exported data
objects (no globals, no constants) exist in the C sources.
