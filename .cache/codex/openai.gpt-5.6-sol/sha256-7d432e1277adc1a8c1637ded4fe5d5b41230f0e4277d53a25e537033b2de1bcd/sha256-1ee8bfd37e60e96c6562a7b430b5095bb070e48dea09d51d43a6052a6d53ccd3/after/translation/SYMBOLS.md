# Dynamic Symbol Surface

Source shared object: `../c_src/build/libdriver.so`

Command:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

Only globally defined dynamic symbols are part of the library surface.
Undefined GLIBC and toolchain symbols shown by unfiltered `nm -D` are dynamic
dependencies, not library exports.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `driver` | `T` | `driver` | [x] |
| `forward_goto_example` | `T` | `forward_goto_example` | [x] |
| `open_with_cleanup` | `T` | `open_with_cleanup` | [x] |

Missing C exports in Rust: **0**

The public header declares `driver`. The other two externally linked
definitions are still public ELF symbols and are therefore included.
