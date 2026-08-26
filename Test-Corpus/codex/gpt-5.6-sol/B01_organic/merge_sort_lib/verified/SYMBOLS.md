# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `merge_sort` | `T` | `merge_sort` | present |

The C shared library exports one public symbol. The Rust shared library exports
the same symbol with the same name.

## Undefined symbols

The C library has one strong undefined libc symbol, `memcpy@GLIBC_2.14`, plus
the usual weak ELF runtime symbols. It has no undefined project symbols.

## Completion

- [x] Every C-defined dynamic symbol is defined by the Rust shared library.
- [x] Rust has zero missing or undefined non-runtime project symbols.
