# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-6nhExu.so`

Rust library:
`target/release/libencode_quant_lib.so`

The table is derived from `nm -D --defined-only` on the reference shared
library. Toolchain-generated weak undefined imports are not library exports.

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `encode_quant` | `T` | `encode_quant` (`T`) | present |

Missing C exports in Rust: **0**

