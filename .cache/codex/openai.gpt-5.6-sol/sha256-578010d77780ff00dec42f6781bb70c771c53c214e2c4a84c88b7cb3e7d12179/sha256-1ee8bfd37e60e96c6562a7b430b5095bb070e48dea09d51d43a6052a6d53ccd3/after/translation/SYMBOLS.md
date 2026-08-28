# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

The export list was derived with:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `encode_base64` | `T` | `encode_base64` | Present |

The C shared object has one defined public dynamic symbol. The Rust shared
object exports the same symbol with the same name. There are no missing
symbols and no missing C source modules.

- [x] `nm -D` reports zero C exports missing from the Rust shared object.
- [x] The Rust shared object has zero undefined non-system symbols.
- [x] Verified with default features and `--no-default-features`.
