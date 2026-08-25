# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The C shared object exports exactly three public text symbols:

| symbol | C address/type | Rust status before Phase A fixes | reason if missing |
|--------|----------------|----------------------------------|-------------------|
| `driver` | `0000000000001196 T` | missing | The translated output inlined this operation into Rust `main`; the C function itself was not translated or exported. |
| `foo` | `0000000000001149 T` | missing | A private Rust slice helper exists, but it does not have the C pointer/character ABI and is not exported. |
| `main` | `00000000000011f3 T` | missing | The Rust binary entry point is not an exported `extern "C"` function in a `cdylib`. |

Phase D completion status:

- [x] `driver`
- [x] `foo`
- [x] `main`
- [x] No C-defined dynamic symbols are missing from the Rust shared object.
- [x] No unexpected undefined non-libc symbols exist in the Rust shared object.

Final comparison:

```text
C:    driver foo main
Rust: driver foo main
Missing from Rust: (none)
```
