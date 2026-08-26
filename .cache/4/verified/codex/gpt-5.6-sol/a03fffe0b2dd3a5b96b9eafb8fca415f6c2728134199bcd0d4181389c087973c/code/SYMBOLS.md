# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver.so`, built from the unmodified
`c_src/src/main.c` with position-independent code.

Command:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C symbol | Type | Rust status | Reason/action |
|----------|------|-------------|---------------|
| `driver` | `T` | Matched | Exported by `src/lib.rs` with the exact C ABI and symbol name. |
| `main` | `T` | Matched | Exported by `src/lib.rs` with the exact C ABI and symbol name. |

Undefined symbols in the C shared object are C runtime symbols only:
`__isoc99_scanf`, `printf`, and `puts`, plus standard weak toolchain symbols.

Final symbol-set comparison:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

Result: empty (zero C symbols missing from Rust).
