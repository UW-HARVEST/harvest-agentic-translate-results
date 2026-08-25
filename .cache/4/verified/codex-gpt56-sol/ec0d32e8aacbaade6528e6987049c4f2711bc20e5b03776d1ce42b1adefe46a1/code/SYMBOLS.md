# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver_c.so
```

## Public Defined Symbols

| symbol | C | Rust | status |
|--------|---|------|--------|
| `helloworld` | `T` | `T` | present |
| `main` | `T` | `T` | present |

The mechanically computed defined-symbol difference is empty:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

## Complete C `nm -D` Output

| symbol | kind | classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | weak undefined | toolchain runtime; also present in Rust |
| `_ITM_registerTMCloneTable` | weak undefined | toolchain runtime; also present in Rust |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined | libc runtime; also present in Rust |
| `__gmon_start__` | weak undefined | toolchain runtime; also present in Rust |
| `helloworld` | defined text | public C API; exported by Rust |
| `main` | defined text | program entry point; exported by Rust |
| `puts@GLIBC_2.2.5` | undefined | libc import, not an exported C definition |

Result: **0 missing defined symbols** and **0 undefined non-libc API
symbols**.
