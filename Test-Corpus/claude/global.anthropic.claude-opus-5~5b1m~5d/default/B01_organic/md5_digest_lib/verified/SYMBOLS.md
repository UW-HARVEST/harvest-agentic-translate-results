# SYMBOLS.md — Phase A: public ABI surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-yTDDEy.so   (name derives from parent dir name)

# Rust
cd translation && cargo build --release --offline
# -> translation/target/release/libmd5_digest_lib.so
```

## C source inventory

The entire C library is two files:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | `tflac_u8`, `tflac_u32` typedefs; `struct tflac_md5`; decl of `md5_digest` |
| `c_src/src/lib.c` | the single definition of `md5_digest` (16 straight-line stores) |

There are **no** other `.c` files, no `#ifdef` feature gates, no
namespace-renaming macros, and no macro-generated symbol names. So the exported
surface is exactly one function.

## Exported (defined) symbols

`nm -D --defined-only` output, filtered to non-libc / non-toolchain symbols:

| symbol | C `.so` | Rust `.so` | signature |
|--------|---------|------------|-----------|
| `md5_digest` | `T` (text, global) | `T` (text, global) | `void md5_digest(const tflac_md5 *m, tflac_u8 out[16])` |

Raw output:

```
$ nm -D --defined-only c_src/build/libharvest-work-yTDDEy.so
00000000000010f9 T md5_digest

$ nm -D --defined-only translation/target/release/libmd5_digest_lib.so
0000000000011c30 T md5_digest
```

## Symbol diff (Phase D gate)

```
$ diff <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only RUST.so | awk '{print $3}' | sort)
(empty)
```

**Result: EMPTY.** 0 symbols missing from the Rust `.so`, 0 extra.

* Nothing needed a new `#[no_mangle]` wrapper — `md5_digest` is already exported
  via `#[unsafe(no_mangle)] pub unsafe extern "C" fn`.
* No C module was left untranslated — `src/lib.c` is the only translation unit
  and its only function is present.
* No stubs / `unimplemented!()` were introduced.

## Undefined (imported) symbols

The Rust `.so` must not reference non-libc symbols that cannot resolve:

```
$ nm -D --undefined-only translation/target/release/libmd5_digest_lib.so
```

All entries resolve against the platform C runtime / libgcc (see
`tests/differential.rs::symbol_parity_*`, which asserts this at test time).
0 unresolvable non-libc undefined symbols.

## Type-layout parity

`struct tflac_md5` = four naturally-aligned `uint32_t`, no padding.

| property | C | Rust (`#[repr(C)]`) |
|----------|---|---------------------|
| `sizeof` | 16 | 16 |
| `alignof` | 4 | 4 |
| offset of `a` / `b` / `c` / `d` | 0 / 4 / 8 / 12 | 0 / 4 / 8 / 12 |

Asserted at runtime by the differential test suite against a C-side
`static_assert`-equivalent probe compiled from the real header.
