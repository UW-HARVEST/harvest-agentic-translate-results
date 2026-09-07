# Dynamic symbol parity

Derived mechanically from the complete output of:

```text
nm -D ../c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

| C dynamic symbol | C type | Rust entry present | Status |
|------------------|--------|--------------------|--------|
| `_ITM_deregisterTMCloneTable` | `w` | yes | [x] exact match |
| `_ITM_registerTMCloneTable` | `w` | yes | [x] exact match |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | yes | [x] exact match |
| `__gmon_start__` | `w` | yes | [x] exact match |
| `driver` | `T` | yes (`T`) | [x] exact match |
| `printf@GLIBC_2.2.5` | `U` | yes | [x] exact match |
| `puts@GLIBC_2.2.5` | `U` | yes | [x] exact match |

`driver` is the only symbol defined and publicly exported by the C library.
Missing C dynamic symbol entries in Rust: **0**.
