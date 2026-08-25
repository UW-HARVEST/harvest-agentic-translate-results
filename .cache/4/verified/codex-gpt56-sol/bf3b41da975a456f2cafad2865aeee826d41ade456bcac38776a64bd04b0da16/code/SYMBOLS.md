# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so
```

The C shared library has one defined public dynamic symbol. Undefined weak
toolchain symbols are not library API and are excluded.

| # | C symbol | Kind | Rust export | Status |
|---|----------|------|-------------|--------|
| 1 | `hdr_compare` | global function (`T`) | `hdr_compare` | [x] |

Completion criterion: the set difference between defined public C symbols and
defined public Rust symbols is empty.
