# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-87rYkA.so
nm -D --defined-only translation/target/release/libpow43_lib.so
```

## C `.so` exported (defined) dynamic symbols

```
0000000000001109 T pow43
```

(address is build-dependent; the name is what matters for ABI parity)

Weak/undefined entries emitted by the C toolchain itself (not part of the
library ABI, ignored for parity):

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

## Rust `.so` exported (defined) dynamic symbols

```
0000000000011a60 T pow43
```

## Parity table

| # | C symbol | kind | C source declaration | present in Rust `.so` | note |
|---|----------|------|----------------------|-----------------------|------|
| 1 | `pow43`  | `T` (global text) | `float pow43(int x);` (`c_src/include/lib.h:1`) | YES | `#[unsafe(no_mangle)] pub extern "C" fn pow43(x: c_int) -> f32` |

## Symbols NOT exported by C (must also not be part of the compared ABI)

| C entity | storage | exported? | Rust counterpart |
|----------|---------|-----------|------------------|
| `g_pow43[129 + 16]` | `static const float` | no (internal) | `static G_POW43: [f32; 145]` (private) |

## Diff

`comm -23` of the two defined-symbol name lists (C minus Rust): **empty**.
`comm -13` (Rust minus C): **empty** — Rust exports no extra ABI symbols.

Undefined (`U`) symbols in the Rust `.so` are all libc / libgcc-unwind imports
(`memcpy`, `malloc`, `_Unwind_*`, `pthread_*`, …) pulled in by the Rust runtime;
there are **0 missing/undefined non-libc symbols**.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
- [x] Every C-exported symbol is exported by the Rust `.so` under the exact same name.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default (empty) feature set. The feature-combo
sweep therefore consists of exactly:

| # | cargo invocation | symbols match |
|---|------------------|---------------|
| 1 | `cargo build --release` (default) | yes |
| 2 | `cargo build --release --no-default-features` (identical, no features exist) | yes |
