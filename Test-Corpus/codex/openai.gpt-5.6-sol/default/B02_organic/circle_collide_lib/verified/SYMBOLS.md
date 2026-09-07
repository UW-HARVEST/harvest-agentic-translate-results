# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-mnGvRm.so
```

The C shared library exports 12 public text symbols. The Rust status column is
checked only after verifying the exact symbol name in the Rust shared library.

| # | C symbol | kind | Rust export |
|---|----------|------|-------------|
| S01 | `c2V` | function | [x] |
| S02 | `c2Mulvs` | function | [x] |
| S03 | `c2Maxv` | function | [x] |
| S04 | `c2Minv` | function | [x] |
| S05 | `c2Clampv` | function | [x] |
| S06 | `c2Sub` | function | [x] |
| S07 | `c2Dot` | function | [x] |
| S08 | `c2CircletoCircle` | function | [x] |
| S09 | `c2CircletoAABB` | function | [x] |
| S10 | `c2CircletoCapsule` | function | [x] |
| S11 | `c2Collided` | function | [x] |
| S12 | `circle_collide` | function | [x] |

Undefined imports are toolchain/runtime symbols, not C-library API symbols.
The final sorted `nm -D --defined-only` diff is empty (12 C, 12 Rust).
