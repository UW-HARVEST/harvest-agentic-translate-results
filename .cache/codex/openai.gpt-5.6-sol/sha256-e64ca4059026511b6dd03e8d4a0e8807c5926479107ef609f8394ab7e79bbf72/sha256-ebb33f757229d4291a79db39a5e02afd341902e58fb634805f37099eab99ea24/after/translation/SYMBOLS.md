# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libharvest-work-5ZTW5F.so
nm -D --defined-only ../c_src/build/libharvest-work-5ZTW5F.so
nm -D --defined-only target/release/libbin2hex_lib.so
```

## C `nm -D` output

```text
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U abort@GLIBC_2.2.5
0000000000001109 T bin2hex
```

The weak/undefined entries are compiler/runtime imports, not public definitions
provided by the C library.

## Defined public symbol parity

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `bin2hex` | `T` | `bin2hex` | `T` | [x] exact match |

Missing C-defined symbols in Rust: **0**.

This parity check passed after both the default build and the explicit
`--no-default-features` build. The Rust library has no undefined project/API
symbol matching `bin2hex`, `harvest`, or `translation`.
