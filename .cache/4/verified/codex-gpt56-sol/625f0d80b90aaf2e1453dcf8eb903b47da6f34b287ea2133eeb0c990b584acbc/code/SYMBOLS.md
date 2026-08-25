# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`

Generated from:

```text
$ nm -D --defined-only c_src/build/libdriver_c.so
0000000000001159 T main
0000000000001139 T static_sum
```

Only globally defined dynamic symbols are public library exports. Undefined
entries in the unfiltered `nm -D` output are libc/toolchain imports, not symbols
implemented by this source.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `main` | `T` | `main` | [x] |
| `static_sum` | `T` | `static_sum` | [x] |

Parity check:

```text
$ comm -23 \
    <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

Result: empty (zero C exports missing from Rust).
