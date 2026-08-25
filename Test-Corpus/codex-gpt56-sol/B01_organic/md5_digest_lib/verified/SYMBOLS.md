# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/deps/libmd5_digest_lib.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `md5_digest` | `T` | `md5_digest` | present |

The unfiltered C dynamic table also contains the undefined weak runtime imports
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`. These are toolchain imports,
not symbols defined or implemented by this library.

Missing defined C symbols in Rust: **0**
