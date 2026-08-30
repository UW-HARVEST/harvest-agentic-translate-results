# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

The C library's undefined dynamic entries (`printf`, `puts`, and weak ELF
runtime hooks) are external runtime imports, not library-owned API symbols.

| symbol | C definition | Rust definition | status |
|--------|--------------|-----------------|--------|
| `bad` | `src/driver.c:42` | `src/lib.rs:29` | present |
| `driver` | `src/driver.c:106` | `src/lib.rs:78` | present |
| `good` | `src/driver.c:100` | `src/lib.rs:72` | present |
| `printIntLine` | `src/driver.c:37` | `src/lib.rs:22` | present |
| `printLine` | `src/driver.c:29` | `src/lib.rs:13` | present |

- [x] Every library-owned C dynamic definition is exported by Rust with the
      exact name.
- [x] Missing library-owned symbols: 0.
- [x] Undefined non-runtime/non-libc C symbols: 0.
