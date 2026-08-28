# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-ktJGSr.so
```

| C symbol | Kind | Rust export present |
|----------|------|---------------------|
| `next_double` | `T` | [x] |

The C shared object has one defined dynamic symbol. Weak runtime symbols and
undefined libc/toolchain symbols are not library API definitions.

Final `comm` comparison of sorted `nm -D --defined-only` symbol names produced
no missing C symbols.
