# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported (defined, dynamic) symbols

| addr | type | symbol |
|------|------|--------|
| `000000000000122b` | `T` | `driver` |
| `0000000000001129` | `T` | `fma_array` |

Note: `inner` is `static` in `c_src/src/driver.c` and therefore has internal
linkage — it is deliberately NOT in the dynamic symbol table and must NOT be
exported by Rust either. Confirmed absent from `nm -D` on the C `.so`.

## Rust `.so` exported (defined, dynamic) symbols

| addr | type | symbol |
|------|------|--------|
| `0000000000011760` | `T` | `driver` |
| `0000000000011910` | `T` | `fma_array` |

## Parity diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

- Symbols in C but missing from Rust: **0**
- Extra non-libc/non-runtime symbols in Rust: **0**
- No module of the C source was skipped: `c_src` contains exactly one
  translation unit (`src/driver.c`, 46 lines) and one header
  (`include/driver.h`); both functions with external linkage in it are
  implemented and exported by `translation/src/lib.rs`. The one `static`
  function (`inner`) is translated as a private Rust `unsafe fn`.

## Undefined (imported) symbols in the Rust `.so`

Only libc / Rust-runtime imports. The translation deliberately imports the
platform `printf` so that stdio formatting and buffering are byte-identical
to the C library and interleave with C stdio in the same process:

- `printf` (libc) — used by `inner`
- allocator / unwind / `memcpy`-class symbols from libc and the Rust runtime

0 missing/undefined **non-libc** symbols.

## Checklist

- [x] Every C `.so` symbol is exported by the Rust `.so` with the exact name.
- [x] No stubs / `unimplemented!()` were introduced.
- [x] `static` C function `inner` is correctly NOT exported by either `.so`.
