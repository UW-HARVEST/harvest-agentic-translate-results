# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --offline --release
```

## C `.so` exported (defined, dynamic) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `driver` | `T` (global text) | YES (`translation/src/lib.rs`, `#[unsafe(no_mangle)] pub extern "C" fn driver`) |

## Rust `.so` exported (defined, dynamic) symbols

`nm -D --defined-only translation/target/release/libdriver.so`

| # | symbol | type | in C `.so`? |
|---|--------|------|-------------|
| 1 | `driver` | `T` (global text) | YES |

## Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Result: **empty** — 0 symbols missing from the Rust `.so`, 0 extra.

## Source-file completeness

The C library is a single translation unit:

* `c_src/include/driver.h` — declares exactly one function: `void driver(int x, int y);`
* `c_src/src/driver.c` — defines exactly that one function. No `static` helpers,
  no macro-generated symbols, no global data, no constructors/destructors.

`grep -n '^[a-zA-Z_].*(' c_src/src/driver.c` yields only the `driver` definition,
so no C source/module was skipped by the translation. Nothing to translate
additionally, and nothing is stubbed in Rust.

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only translation/target/release/libdriver.so` lists only
libc / libgcc-unwind / glibc-pthread imports (`puts`, `memcpy`, `malloc`,
`_Unwind_*`, `__errno_location`, ...). LLVM lowers the crate's
`printf("loop\n")`-style calls to `puts`, exactly as gcc does for the C source
(`nm -D --undefined-only c_src/build/libdriver.so` also imports `puts`), so the
emitted bytes are identical.

**0 missing / unresolved non-libc symbols.**

## Cargo feature surface

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, therefore the only feature combination that exists is the default
(empty) one. `cargo test --no-default-features` is equivalent to `cargo test`.
Verified in Phase D.
