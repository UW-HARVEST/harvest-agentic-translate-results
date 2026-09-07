# Dynamic symbol surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-BZe1V7.so
nm -D --defined-only target/release/libcollided_lib.so
```

The C library exports 10 defined public symbols. The Rust library currently
exports every one with the exact spelling.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `c2AABBtoAABB` | `T` | `c2AABBtoAABB` | [x] |
| 2 | `c2CircletoAABB` | `T` | `c2CircletoAABB` | [x] |
| 3 | `c2CircletoCircle` | `T` | `c2CircletoCircle` | [x] |
| 4 | `c2Clampv` | `T` | `c2Clampv` | [x] |
| 5 | `c2Dot` | `T` | `c2Dot` | [x] |
| 6 | `c2Maxv` | `T` | `c2Maxv` | [x] |
| 7 | `c2Minv` | `T` | `c2Minv` | [x] |
| 8 | `c2Sub` | `T` | `c2Sub` | [x] |
| 9 | `c2V` | `T` | `c2V` | [x] |
| 10 | `collided` | `T` | `collided` | [x] |

Missing C exports in Rust: **0**.

The C library has no undefined non-libc dependency symbols. Its only undefined
entries are standard weak runtime symbols. The Rust `cdylib` has the expected
Rust runtime, unwind, allocator, pthread, dynamic-loader, and libc imports; none
are unresolved project-library symbols.
