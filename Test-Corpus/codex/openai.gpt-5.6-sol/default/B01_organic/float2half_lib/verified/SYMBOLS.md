# Dynamic Symbol Surface

Derived from:

```text
nm -D ../c_src/build/libharvest-work-PGeIlH.so
nm -D target/release/libfloat2half_lib.so
```

## C dynamic symbols

| symbol | C kind | Rust `nm -D` status |
|--------|--------|---------------------|
| `_ITM_deregisterTMCloneTable` | weak undefined toolchain hook | present (weak undefined) |
| `_ITM_registerTMCloneTable` | weak undefined toolchain hook | present (weak undefined) |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined libc ABI | present (weak undefined) |
| `__gmon_start__` | weak undefined toolchain hook | present (weak undefined) |
| `float2half` | defined public function | present (defined public function) |

## Public API parity

The only defined public C symbol is `float2half`. The Rust shared library
exports it with the exact same name. Missing public C symbols: **0**.

