# Dynamic Symbol Surface

Source artifact:
`../c_src/build/shared/libmacrodepth_add_5.so`, built from the unmodified
`mdcore.c` and `mdmain.c` with `OP=add` and `REPEAT=5`.

Extraction command:

```sh
nm -D --defined-only ../c_src/build/shared/libmacrodepth_add_5.so
```

Only defined dynamic symbols are part of the implementation surface; imported
libc symbols are not C library exports.

| C symbol | ELF kind | Rust status at initial inventory | Source |
|----------|----------|----------------------------------|--------|
| `G_OP` | `D` | exported | `mdcore.c:36` |
| `G_OP_NAME` | `D` | exported | `mdcore.c:37` |
| `helper_call` | `T` | exported | `mdcore.c:39` |
| `helper_ptr` | `T` | exported | `mdcore.c:47` |
| `main` | `T` | missing: `mdmain.c` had not been translated | `mdmain.c:28` |
| `op_add` | `T` | exported | `mdcore.c:28` |
| `op_mul` | `T` | exported | `mdcore.c:30` |
| `op_sub` | `T` | exported | `mdcore.c:29` |
| `use_generated` | `T` | exported | `mdcore.c:54` |

`accum_<OP>` is macro-generated but declared `static`, so it is correctly
absent from the dynamic symbol table.

## Completion

- [x] Translate and export `main`.
- [x] Confirm the defined C-to-Rust symbol diff is empty for all 24 builds.
- [x] Confirm Rust has no undefined non-libc project symbols.
