# Exported Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libcircle_collide_lib.so
```

The C shared library exports 12 public symbols. All 12 are exported by the
Rust shared library with exact names.

| # | C symbol | Rust export | Status |
|---|----------|-------------|--------|
| 1 | `c2V` | `c2V` | [x] |
| 2 | `c2Mulvs` | `c2Mulvs` | [x] |
| 3 | `c2Maxv` | `c2Maxv` | [x] |
| 4 | `c2Minv` | `c2Minv` | [x] |
| 5 | `c2Clampv` | `c2Clampv` | [x] |
| 6 | `c2Sub` | `c2Sub` | [x] |
| 7 | `c2Dot` | `c2Dot` | [x] |
| 8 | `c2CircletoCircle` | `c2CircletoCircle` | [x] |
| 9 | `c2CircletoAABB` | `c2CircletoAABB` | [x] |
| 10 | `c2CircletoCapsule` | `c2CircletoCapsule` | [x] |
| 11 | `c2Collided` | `c2Collided` | [x] |
| 12 | `circle_collide` | `circle_collide` | [x] |

Missing C symbols in Rust: **0**.

Undefined non-system symbols in Rust: **0**. The Rust library's undefined
entries are runtime/libc/libgcc symbols supplied by the platform.
