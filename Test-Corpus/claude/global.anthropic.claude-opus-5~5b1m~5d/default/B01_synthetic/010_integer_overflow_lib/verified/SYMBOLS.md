# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

* C  `.so`: `c_src/build/libdriver.so`
* Rust `.so`: `translation/target/release/libdriver.so`

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```cmake
add_library(driver SHARED
    src/driver.c)
```

so the whole library is `c_src/src/driver.c` (+ the public header
`c_src/include/driver.h`). There is no un-translated module: `driver.c` defines
exactly two functions, `printHexCharLine` and `driver`, and both are translated
in `translation/src/lib.rs`. Nothing is stubbed or `unimplemented!()`.

## Exported (defined, dynamic) symbols

| # | symbol | C signature | in C `.so` | in Rust `.so` | status |
|---|--------|-------------|------------|---------------|--------|
| 1 | `printHexCharLine` | `void printHexCharLine(char)` | `T` yes | `T` yes | OK |
| 2 | `driver`           | `void driver(char)`           | `T` yes | `T` yes | OK |

`nm -D --defined-only` raw output:

```
=== C ===
0000000000001143 T driver
0000000000001119 T printHexCharLine

=== Rust ===
0000000000011710 T driver
0000000000011730 T printHexCharLine
```

Symbol diff (C-exported minus Rust-exported): **empty**.

Note: `printHexCharLine` is *not* declared in `driver.h`, but it has external
linkage in `driver.c` and therefore appears in the C `.so`'s dynamic symbol
table. It is consequently part of the ABI and is exported (and tested) from the
Rust `.so` as well.

## Undefined (imported) symbols

The C `.so` imports only `printf` (plus the usual weak CRT/ITM hooks). The Rust
`.so` imports `printf` too, plus libc/`libgcc` unwinder symbols pulled in by the
Rust standard library (`malloc`, `memcpy`, `write`, `_Unwind_*`, `pthread_*`,
…). All Rust-side undefined symbols are libc / unwinder / weak-CRT symbols:

* **0 missing or undefined non-libc symbols in the Rust `.so`.**

Verification command:

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $NF}' | sort) \
  <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
# -> no output
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one (`--no-default-features` is equivalent).
`cdylib` is the only crate type. There is no binary target in either project
(`CMakeLists.txt` builds only `add_library(... SHARED ...)`, and the crate has
no `[[bin]]` / `src/main.rs`), so the "compare binary stdout" gate is
vacuously satisfied — instead the library's stdout is compared through the FFI
boundary in Phases B/C.
