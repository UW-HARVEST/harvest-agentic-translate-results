# Dynamic Symbol Surface

Reference library: `c_src/build/libdriver.so`

Command used:

```text
nm -D --defined-only --format=posix c_src/build/libdriver.so | sort
```

| C symbol | Type | C source | Rust export |
|----------|------|----------|-------------|
| `FIO_createFilename_fromOutDir` | `T` | `src/lib.c` | [x] exact-name export present |
| `extractFilename` | `T` | `src/lib.c` | [x] exact-name export present |

The C dynamic symbol table has no other defined global symbols. Its undefined
symbols are libc/toolchain dependencies (`__errno_location`, `calloc`, `exit`,
`fprintf`, `memcpy`, `stderr`, `strerror`, `strlen`, `strrchr`, and weak ELF
runtime hooks), not library API symbols requiring Rust implementations.

