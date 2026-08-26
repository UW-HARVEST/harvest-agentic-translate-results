# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional compilation. The complete valid feature combination
set therefore contains one member:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| 1 | `--no-default-features` (empty feature set) | default | [x] |

## Runtime Configurations

Rows are derived from the branches, loops, pointer comparisons, fixed-width
arrays, and allocation-size expression in `c_src/src/lib.c`. Scalar values not
constrained in a row are randomized over their complete FFI representation.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `create_block` | empty NUL-terminated name; randomized `id` and all 256 `flags` values | [x] |
| 2 | `create_block` | name payload length `1..=30`; randomized bytes without interior NUL, `id`, and `flags` | [x] |
| 3 | `create_block` | maximum in-bounds name payload length 31; randomized bytes, `id`, and `flags` | [x] |
| 4 | `allocate_block` | empty shape: `count == 0`; randomized `init_value` | [x] |
| 5 | `allocate_block` | singleton shape: `count == 1`; randomized `init_value` | [x] |
| 6 | `allocate_block` | many shape: `count >= 2`; randomized count and initialization sequence | [x] |
| 7 | `free_block` | null outer pointer (`mb == NULL`) | [x] |
| 8 | `free_block` | non-null outer block with null `data` | [x] |
| 9 | `free_block`, `allocate_block` | non-null block with non-null zero-length allocation | [x] |
| 10 | `free_block`, `allocate_block` | non-null block with non-null populated allocation | [x] |
| 11 | `compute_hash` | `mb1 < mb2` and `mb1->data < mb2->data`; expected hash 110 | [x] |
| 12 | `compute_hash` | `mb1 < mb2` and equal data pointers (including null); expected hash 10 | [x] |
| 13 | `compute_hash` | `mb1 < mb2` and `mb1->data > mb2->data`; expected hash 210 | [x] |
| 14 | `compute_hash` | `mb1 > mb2` and `mb1->data < mb2->data`; expected hash 120 | [x] |
| 15 | `compute_hash` | `mb1 > mb2` and equal data pointers (including null); expected hash 20 | [x] |
| 16 | `compute_hash` | `mb1 > mb2` and `mb1->data > mb2->data`; expected hash 220 | [x] |
| 17 | `compute_hash` | identical block pointers, which necessarily have identical data pointers; expected hash 0 | [x] |
| 18 | `betagamma` | zero-sized internal blocks: `param1 % 10 == -5`; randomized `param2..=param4` | [x] |
| 19 | `betagamma` | singleton internal blocks: `param1 % 10 == -4`; randomized `param2..=param4` | [x] |
| 20 | `betagamma` | many-element internal blocks: `param1 % 10` in `-3..=9`; randomized valid `param1` and `param2..=param4`, including integer boundaries | [x] |
