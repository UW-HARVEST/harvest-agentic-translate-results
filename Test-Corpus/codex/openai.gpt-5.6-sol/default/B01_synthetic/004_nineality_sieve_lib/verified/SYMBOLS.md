# Dynamic Symbol Surface

Derived from:

```text
nm -D ../c_src/build/libSieve.so
```

Undefined C runtime/toolchain imports are not library API exports. The complete
set of globally defined symbols in the C shared object is:

| symbol | C `nm -D` type | Rust `nm -D` type | status |
|--------|----------------|-------------------|--------|
| `sieve` | `T` | `T` | present |

Missing C exports in Rust: **0**.

