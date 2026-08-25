# Dynamic Symbol Surface

Source artifact: `c_src/build/libtranslated_rust.so`

Command:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

Raw public-symbol output:

```text
00000000000010f9 T update_frame_header
```

| C symbol | Rust implementation | Rust dynamic export |
|----------|---------------------|---------------------|
| `update_frame_header` | `src/lib.rs` | [x] |

The C library exports one defined dynamic symbol. Symbols supplied by the
runtime or libc are not part of this defined-symbol surface.
