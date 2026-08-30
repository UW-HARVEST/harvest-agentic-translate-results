# Dynamic Symbol Surface

Derived from:

```text
$ nm -D --defined-only ../c_src/build/libdriver.so
0000000000001173 T driver
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `driver` | Global text (`T`) | `driver` | Present |

The C shared library exports one defined public symbol. The Rust shared library
exports the same symbol with the exact name:

```text
$ nm -D --defined-only target/release/libdriver.so
0000000000011720 T driver
```

Missing C symbols in Rust: **0**.
