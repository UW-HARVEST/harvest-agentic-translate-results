# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, compiled from the unchanged
`c_src/src/main.c` with `cc -shared -fPIC`.

Command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| C symbol | Type | Rust status before Phase A fixes | Required action |
|----------|------|----------------------------------|-----------------|
| `bad` | `T` | Missing; Rust implementation is private in a binary crate | Export the real implementation from the Rust `cdylib` |
| `good` | `T` | Missing; Rust implementation is private in a binary crate | Export the real implementation from the Rust `cdylib` |
| `main` | `T` | Missing; the Rust binary entry point is not a C ABI export | Export a C ABI wrapper with the C signature from the Rust `cdylib` |
| `printIntLine` | `T` | Missing; Rust implementation is private and snake-cased | Export the real implementation with the exact C symbol name |
| `printLine` | `T` | Missing; Rust implementation uses `&str` instead of the C ABI | Export the real implementation with the exact C signature and null behavior |

The weak runtime entries and undefined platform dependencies reported by plain
`nm -D` are not library-defined public API symbols. The Rust object has only
versioned GLIBC calls, weak ELF hooks, and `_Unwind_*` entries resolved by its
declared `libgcc_s` runtime; it has no undefined application/library symbol.
Final completion requires all five symbols above in the Rust shared object.

## Final Parity

| C symbol | Rust `nm -D --defined-only` | Status |
|----------|-----------------------------|--------|
| `bad` | `T bad` | [x] |
| `good` | `T good` | [x] |
| `main` | `T main` | [x] |
| `printIntLine` | `T printIntLine` | [x] |
| `printLine` | `T printLine` | [x] |

The sorted defined-symbol diff is empty: 5 C symbols and 5 Rust symbols.
