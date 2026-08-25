# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command used:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| symbol | C type | Rust type | status |
|---|---|---|---|
| `contrast_ratio` | `T` | `T` | present |

The remaining entries printed by unfiltered `nm -D` are undefined or weak
runtime imports (`pow`, glibc startup symbols, and toolchain bookkeeping), not
public definitions supplied by this library.

Missing C definitions in Rust: **0**

