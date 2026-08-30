# Dynamic Symbol Surface

Source binary: `../c_src/build/libdriver.so`

Command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `driver` | `T`  | `driver`    | Present |

The C shared object has no undefined non-libc library symbols. Its only
undefined function import is `printf@GLIBC_2.2.5`; the remaining undefined
entries are standard weak toolchain/runtime symbols.
