# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-kFULdk.so
cd translation && cargo build --release
# -> translation/target/release/libfloat2half_lib.so
```

## C `.so` defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `float2half` | `T` (global text) | YES — `T float2half` |

That is the complete list. The C translation unit's only other file-scope
objects are `static uint16_t m__base[512]` and `static uint8_t m__shift[512]`;
`static` gives them internal linkage, so they are **not** in the dynamic symbol
table and must not be exported by Rust either. Verified: they do not appear in
`nm -D` on the C `.so`, and the Rust `M__BASE` / `M__SHIFT` are private
`static`s (also absent from `nm -D` on the Rust `.so`).

There are no macro-generated symbols in this library (no function-defining
macros in `src/lib.c` or `include/lib.h`).

## Symbol diff

```
comm -3 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
        <(nm -D --defined-only RUST.so| awk '{print $NF}' | sort)
```

Result: **empty**. 0 symbols exported by C and missing from Rust.
0 symbols exported by Rust that C does not export.

## Undefined symbols in the Rust `.so`

All undefined (`U` / `w`) entries in the Rust `.so` are libc / libgcc-unwind
imports pulled in by the Rust standard library and panic runtime
(`malloc`, `memcpy`, `abort`, `_Unwind_*`, `pthread_key_*`, `dl_iterate_phdr`,
…). There are **0 undefined non-libc symbols**, i.e. nothing from this crate is
left unresolved. The library loads and resolves `float2half` successfully via
`dlopen`/`libloading` in the integration tests, which is the operational proof.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are therefore the same build, and the symbol table above holds
for every configuration.
