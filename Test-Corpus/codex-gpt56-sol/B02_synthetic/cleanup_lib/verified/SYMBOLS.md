# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Toolchain-generated weak/runtime symbols are absent from the defined-only C
table. The complete C library API surface is:

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `cleanup` | `T` | `cleanup` | present |
| `cleanup_resources` | `T` | `cleanup_resources` | present |
| `print_result` | `T` | `print_result` | present |

Missing C symbols in Rust: **0**.

Undefined non-libc symbols in Rust: **0**. Rust runtime and toolchain symbols
are runtime dependencies of the Rust `cdylib`, not missing C API symbols.

- [x] C-to-Rust exported-symbol diff is empty.
