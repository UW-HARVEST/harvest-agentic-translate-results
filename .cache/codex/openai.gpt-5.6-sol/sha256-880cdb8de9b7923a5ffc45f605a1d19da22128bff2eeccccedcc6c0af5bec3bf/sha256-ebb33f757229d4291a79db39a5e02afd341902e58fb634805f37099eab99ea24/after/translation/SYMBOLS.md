# Dynamic symbol surface

Built reference:
`../c_src/build/libharvest-work-N3dP5u.so`

Built translation:
`target/release/libread_side_info_lib.so`

Source command:

```text
nm -D --defined-only <library>
```

| C address | type | symbol | Rust address | status |
|-----------|------|--------|--------------|--------|
| `00000000000011d1` | `T` | `read_side_info` | `0000000000011b80` | present |

Missing C symbols in Rust: **0**

- [x] Final Phase D symbol comparison repeated after all fixes.
