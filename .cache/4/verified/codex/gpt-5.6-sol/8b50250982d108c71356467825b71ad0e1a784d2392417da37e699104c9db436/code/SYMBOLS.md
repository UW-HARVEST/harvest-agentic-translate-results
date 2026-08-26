# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so
```

Undefined and weak runtime imports shown by unfiltered `nm -D` are not library
exports. The C library has one defined public dynamic symbol.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `get_predict_func` | `T` | `get_predict_func` | [x] |

The defined-symbol comparison is empty:

```text
comm -23 <(C defined symbols) <(Rust defined symbols)
```

Missing C exports in Rust: **0**.
