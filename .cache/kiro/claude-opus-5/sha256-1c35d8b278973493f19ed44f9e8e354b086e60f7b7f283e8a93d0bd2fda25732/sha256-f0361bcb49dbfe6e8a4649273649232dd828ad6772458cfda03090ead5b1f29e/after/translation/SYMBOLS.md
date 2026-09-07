# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (defined) symbols

`nm -D --defined-only c_src/build/libdriver.so`

```
0000000000001173 T driver
```

## Rust `.so` exported (defined) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

```
0000000000011720 T driver
```

## Parity table

| # | C symbol | type | present in Rust `.so` | notes |
|---|----------|------|-----------------------|-------|
| 1 | `driver` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub extern "C" fn driver(x: f32)` |

## Non-exported C symbols (must NOT be exported)

Mechanically, the only other function in the C translation unit is:

| C symbol | linkage | exported? | Rust counterpart |
|----------|---------|-----------|------------------|
| `print_hex` | `static` (internal linkage, `t` in `nm`) | no — correctly absent from `nm -D` on both | private `fn print_hex` in `src/lib.rs`, not exported |

## Undefined (imported) symbols

`nm -D --undefined-only` on each object. Only libc imports are expected.

* C `.so` imports: `printf`, `putchar`/`puts` (compiler may rewrite
  `printf("\n")`), plus the usual `_ITM_*` / `__gmon_start__` /
  `__cxa_finalize` weak toolchain symbols.
* Rust `.so` imports: `printf` plus the usual weak toolchain symbols and libc
  functions pulled in by `std`.

**0 missing non-libc symbols in the Rust `.so`.**

## Completeness check

The whole C library is 2 functions in 1 translation unit
(`c_src/src/driver.c`), with 1 public header (`c_src/include/driver.h`)
declaring exactly `void driver(float x);`. Both functions are translated; no C
module was skipped, so no additional translation work was required for symbol
parity.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only build configuration is the default one. There is no
feature cross-product to iterate over (verified by grepping `Cargo.toml`).
