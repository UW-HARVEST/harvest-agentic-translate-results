# Dynamic Symbol Surface

Generated mechanically from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust export present | Differential coverage |
|----------|------|---------------------|-----------------------|
| `synth_pair` | `T` | yes | [x] |

The C shared object has no other defined dynamic symbols. The comparison
excludes undefined runtime/libc imports by using `--defined-only`.

## Completion

- [x] `comm` reports no C symbols missing from the Rust shared object.
- [x] The Rust shared object has no undefined non-runtime symbols.
