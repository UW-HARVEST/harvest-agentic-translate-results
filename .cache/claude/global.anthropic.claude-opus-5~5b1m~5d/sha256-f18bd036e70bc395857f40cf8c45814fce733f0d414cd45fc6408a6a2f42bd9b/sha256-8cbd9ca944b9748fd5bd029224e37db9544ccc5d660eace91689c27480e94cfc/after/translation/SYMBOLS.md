# SYMBOLS.md — Phase A symbol map

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported symbols (`c_src/build/libdriver.so`)

```
0000000000001192 T bad
00000000000012e8 T driver
0000000000001239 T good
000000000000116b T printIntLine
0000000000001149 T printLine
```

## Rust `.so` exported symbols (`translation/target/release/libdriver.so`)

```
00000000000117e0 T bad
0000000000011800 T driver
00000000000117e0 T good
0000000000011820 T printIntLine
0000000000011840 T printLine
```

(`bad` and `good` share an address in the Rust build because the optimizer
performs identical-code folding; both symbols are still exported and both are
independently resolvable via `dlsym`.)

## Parity table

| # | symbol | C signature (from `c_src/src/driver.c` / `include/driver.h`) | in C `.so` | in Rust `.so` | status |
|---|--------|--------------------------------------------------------------|-----------|---------------|--------|
| 1 | `printLine`    | `void printLine(const char *line)` | yes | yes | OK |
| 2 | `printIntLine` | `void printIntLine(int intNumber)` | yes | yes | OK |
| 3 | `bad`          | `void bad(void)`                   | yes | yes | OK |
| 4 | `good`         | `void good(void)`                  | yes | yes | OK |
| 5 | `driver`       | `void driver(int useGood)`         | yes | yes | OK |

`bad`/`good` are not declared in `driver.h` but have external linkage in
`driver.c`, so they are part of the exported ABI surface and are tested directly.

## Undefined (imported) symbols

C `.so` imports: `printf`, `puts`, `__stack_chk_fail` (libc / compiler runtime).
Rust `.so` imports: `printf` plus the usual libc/`std` set. No non-libc
undefined symbols in the Rust `.so`.

## Result

* Missing from Rust `.so`: **none**
* Extra non-libc symbols in Rust `.so`: **none**
* Symbol diff: **empty** ✅

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the only
configuration is the default (empty) feature set. Phase B/C therefore have a
single feature combination to cover; this is verified by
`tests/feature_matrix.rs` / the `scripts` loop described in `CONFIGS.md`.
