# Dynamic Symbol Surface

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-L5oDcq.so
nm -D --defined-only target/release/libcircle_collide_lib.so
```

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

Extra Rust API symbols: **0**.

Undefined references to C-library API symbols from the Rust library: **0**.

Completion gate: [x] exact dynamic symbol parity.

Feature matrix:

| Cargo mode | `cargo check` | release differential tests |
|------------|---------------|----------------------------|
| default | [x] | [x] 11 passed |
| `--no-default-features` | [x] | [x] 11 passed |

`Cargo.toml` declares no optional features, so these modes exhaust the feature
configurations.
