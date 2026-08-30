# Dynamic Symbol Surface

Source command:

```text
nm -D ../c_src/build/libdriver.so
```

`driver` is the only symbol defined by the C library. The other entries are
undefined dynamic imports or weak toolchain hooks. All seven C entries are also
present in the Rust dynamic symbol table.

| symbol | C type | role | Rust `.so` |
|---|---:|---|:---:|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain hook | [x] |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain hook | [x] |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak libc hook | [x] |
| `__gmon_start__` | `w` | weak toolchain hook | [x] |
| `driver` | `T` | public API export | [x] |
| `printf@GLIBC_2.2.5` | `U` | libc import | [x] |
| `strcspn@GLIBC_2.2.5` | `U` | libc import | [x] |

Defined-symbol parity command:

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

Expected output: empty.
