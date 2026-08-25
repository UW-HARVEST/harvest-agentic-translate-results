# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The default CMake configuration has one source file, no options, and no
conditional compilation. Symbols supplied by the ELF runtime are excluded;
the command above reported only the library API below.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `memchra2` | `T` | `memchra2` | [x] |

