# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-mpLI0z.so   (name = parent dir name, see CMakeLists.txt)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libbitwriter_add_lib.so   ([lib] name in Cargo.toml)
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libharvest-work-mpLI0z.so`

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `bitwriter_add` | `T` (global text) | YES |

## Rust `.so` exported (defined) dynamic symbols

`nm -D --defined-only translation/target/release/libbitwriter_add_lib.so`

| # | symbol | type |
|---|--------|------|
| 1 | `bitwriter_add` | `T` (global text) |

## Symbol diff

```
comm -23 <(c_syms) <(rust_syms)   # in C but not Rust
=> (empty)
```

**0 missing symbols.** The C translation unit (`c_src/src/lib.c`, the only source
file listed in `CMakeLists.txt`) defines exactly one external function, and it is
implemented and exported by the Rust crate via `#[unsafe(no_mangle)] pub unsafe
extern "C" fn bitwriter_add`. No module of the C source was skipped; there is no
second `.c` file. No stubs are present — the Rust body is a statement-for-statement
translation.

`c_src/include/lib.h` additionally declares data *types* only
(`tflac_u8`, `tflac_u32`, `tflac_u64`, `tflac_uint`, `struct tflac_bitwriter`);
types produce no dynamic symbols. Their ABI (size 32, align 8, field offsets
0/8/12/16/20/24) is verified behaviourally in Phase B instead of via `nm`.

Undefined (imported) symbols in the Rust `.so` are libc/runtime only
(`memcpy`, `__cxa_*`-class runtime helpers, etc.) — no unresolved project symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `--no-default-features` is still exercised in
Phase D for completeness and produces an identical symbol set.
