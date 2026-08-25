# Dynamic Symbol Surface

Generated from the default C build:

```text
nm -D --defined-only c_src/build/libdriver.so
0000000000001139 T parse_number
```

| symbol | C type | C source | Rust export | status |
|---|---|---|---|---|
| `parse_number` | `T` | `c_src/src/lib.c:13` | `src/lib.rs:32` | [x] exact name present |

The mechanically sorted defined-symbol difference is empty:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

The C library's undefined dynamic symbols are the libc functions `free`,
`malloc`, `memcpy`, and `strtod`, plus weak ELF runtime hooks. It has no
undefined non-libc library symbols.

