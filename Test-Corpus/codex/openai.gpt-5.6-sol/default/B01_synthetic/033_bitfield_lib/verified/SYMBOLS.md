# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

Only externally callable `T` symbols are part of the library API. ELF
toolchain bookkeeping symbols are not exported as public `T` symbols by
`nm -D --defined-only`.

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `driver` | `driver` | [x] exact export present |
| `print_foo` | `print_foo` | [x] exact export present |

Completion:

- [x] Every public C dynamic symbol is exported by the Rust shared library.
- [x] Missing-symbol diff is empty.
