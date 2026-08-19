# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Mechanically derived, not assumed.

## How the two shared objects were produced

```sh
# C
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> translated_rust/c_src/build/libdriver.so

# Rust (crate-type = ["cdylib"])
cd translated_rust && cargo build --offline
# -> translated_rust/target/debug/libdriver.so
```

## Complete C source surface

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/driver.c` | `src/lib.rs` (`driver`) | TRANSLATED |

`c_src/include/driver.h` declares exactly one public prototype: `void driver(int x);`
There are **no** other `.c`/`.h` files in `c_src`, so no module was skipped.

## `nm -D --defined-only` on the C `.so`

```
0000000000001109 T driver
```

Exactly **one** exported symbol: `driver`.

## `nm -D --defined-only` on the Rust `.so`

```
0000000000011e50 T driver
```

## Symbol diff (C exports that the Rust `.so` does not export)

| # | C symbol | type | exported by Rust `.so`? | note |
|---|----------|------|-------------------------|------|
| 1 | `driver` | `T` (global text) | **YES** — `T driver` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int)` |

**MISSING SYMBOLS: 0** — the diff is empty. No stubs were added; the single
symbol is a real translation of the real C body.

The C `.so` exports no data symbols, no weak symbols of its own, and no
macro-generated symbols (there are no function-generating macros in the C
source), so there is nothing else to match.

## Undefined (imported) symbols

C `.so` non-libc undefined symbols: **none**.

```
w _ITM_deregisterTMCloneTable   (weak, toolchain)
w _ITM_registerTMCloneTable     (weak, toolchain)
w __cxa_finalize@GLIBC_2.2.5    (libc)
w __gmon_start__                (weak, toolchain)
U printf@GLIBC_2.2.5            (libc)
```

Rust `.so` non-libc undefined symbols: **none**. Every `U`/`w` entry resolves to
`libc.so.6` (`printf`, `malloc`, `memcpy`, `write`, `open64`, ...) or to
`libgcc_s.so.1` (`_Unwind_*`), both of which appear in `ldd`:

```
libgcc_s.so.1 => /lib64/libgcc_s.so.1
libc.so.6     => /lib64/libc.so.6
```

Note that the Rust `.so` imports `printf@GLIBC_2.2.5`, exactly like the C `.so`:
the translation deliberately calls the C runtime's `printf` so that the emitted
bytes *and* the `stdout` buffering / flush-at-exit behaviour are bit-identical
to the original library rather than merely similar.

- [x] `nm -D` shows 0 missing / 0 undefined non-libc symbols in Rust.

## Build-time configuration surface (Phase A enumeration)

`Cargo.toml` has **no `[features]` table**, therefore the complete set of valid
feature combinations is:

| # | combination | cargo invocation | equals default? |
|---|-------------|------------------|-----------------|
| 1 | *(empty set)* | `cargo check/test --no-default-features` | yes — there is no `default` feature and no optional dependency |

`c_src/CMakeLists.txt` declares no `option()`, no `add_definitions`, no
`target_compile_definitions` and the C source contains no `#ifdef`/`#if`
(verified by grep: 0 hits), so the C side likewise has exactly one
configuration. Phases B–C therefore need to be run once, under
`--no-default-features` (which is byte-identical to the default build).
