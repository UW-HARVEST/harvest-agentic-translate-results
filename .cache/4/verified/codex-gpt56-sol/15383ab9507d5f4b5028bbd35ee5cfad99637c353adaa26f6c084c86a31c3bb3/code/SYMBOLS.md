# Exported Symbol Surface

Derived from:

```text
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001159 T searchAndReplace
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `searchAndReplace` | global text (`T`) | `searchAndReplace` | [x] |

The C library's remaining dynamic symbols are undefined libc references or weak
toolchain hooks, not public definitions supplied by this library.

The sorted `nm -D --defined-only` C-minus-Rust symbol diff is empty.
