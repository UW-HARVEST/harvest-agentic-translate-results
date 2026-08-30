# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated in Rust? | where |
|---|---|---|
| `c_src/src/driver.c` | yes | `translation/src/lib.rs` |
| `c_src/include/driver.h` (declares `void driver(int)`) | yes | `driver` export |

There are no other `.c` files in the project, so no module was skipped.

## Symbol table

`T` = exported text symbol. Filtered to non-libc, defined, dynamic symbols.

| # | symbol | C `.so` | Rust `.so` | linkage in C | notes |
|---|--------|---------|------------|--------------|-------|
| 1 | `driver` | T | T | `extern` (public, from `driver.h`) | `void driver(int floors)`; Rust: `#[unsafe(no_mangle)] pub extern "C" fn driver(floors: c_int)` |
| — | `print_hex` | absent (static) | absent (private `unsafe fn`) | `static` — not part of the ABI | correctly NOT exported by Rust; exporting it would be a parity failure in the other direction |

## Diff result

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

- Missing from Rust: **none**
- Undefined non-libc symbols in Rust `.so`: **none** (only `printf` from `libc`,
  which the C `.so` also imports).

Automated in `tests/differential.rs::symbol_parity_c_vs_rust`, which shells out to
`nm -D` on both objects at test time and asserts the C set is a subset of the Rust set.

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
