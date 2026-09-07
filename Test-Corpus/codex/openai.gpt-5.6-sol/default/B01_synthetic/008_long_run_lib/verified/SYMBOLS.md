# Exported Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/liblong.so
nm -D --defined-only target/release/liblong.so
```

| C symbol | kind | Rust export | status |
|----------|------|-------------|--------|
| `array` | `B` (global data, `int[256 * 1024]`) | `array` (`B`) | present |
| `long_exec` | `T` (function) | `long_exec` (`T`) | present |
| `perform_expensive_operations` | `T` (function) | `perform_expensive_operations` (`T`) | present |

Missing C-defined symbols in Rust: **0**.

The C library's undefined strong symbols are `printf@GLIBC_2.2.5`,
`rand@GLIBC_2.2.5`, and `srand@GLIBC_2.2.5`; all are libc dependencies rather
than library API symbols.

Phase D recheck: the sorted C-minus-Rust symbol diff is empty, there are no
extra Rust exports, and `ldd -r target/release/liblong.so` reports no unresolved
relocations.
