# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libSieve.so
```

The C shared library has one public defined dynamic symbol. Undefined runtime
symbols such as `printf` are imports, not library API exports.

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `sieve` | `T` | `sieve` | [x] present |

The defined-symbol diff is empty:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libSieve.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libSieve.so | awk '{print $3}' | sort -u)
```

