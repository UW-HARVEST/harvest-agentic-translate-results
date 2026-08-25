# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | C source | Rust export | Status |
|----------|----------|-------------|--------|
| `bad` | `c_src/src/driver.c:42` | `src/lib.rs:18` | [x] |
| `driver` | `c_src/src/driver.c:58` | `src/lib.rs:30` | [x] |
| `good` | `c_src/src/driver.c:53` | `src/lib.rs:24` | [x] |
| `printLine` | `c_src/src/driver.c:28` | `src/lib.rs:9` | [x] |

The mechanically generated C-to-Rust missing-symbol set is empty.

The C library's only strong undefined symbol is `puts@GLIBC_2.2.5`; its other
undefined entries are weak ELF/libc runtime hooks. The Rust library's undefined
entries are libc, GCC unwinding, pthread, and ELF runtime symbols; it has no
undefined project-library symbol.
