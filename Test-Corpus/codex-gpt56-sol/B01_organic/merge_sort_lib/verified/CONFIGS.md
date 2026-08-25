# Configuration surface

## Build-time configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options, feature variables, or conditional branches. There is exactly one
valid build-time configuration:

| # | Cargo feature set | CMake configuration | [ ] |
|---|-------------------|---------------------|-----|
| 1 | empty (`--no-default-features`) | default | [x] compile check |

## Runtime configurations

The public surface contains only `merge_sort`. It has no runtime options,
modes, flags, formats, element-type choices, or byte-order choices. The rows
below enumerate the input shapes and value relationships distinguished by the
C control flow. Every row is exercised repeatedly with a fixed-seed generator,
and both caller-visible buffers are compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `merge_sort` | size 0; valid non-null empty buffers; recursion base case | [x] |
| 2 | `merge_sort` | size 1; randomized record and padding; recursion base case | [x] |
| 3 | `merge_sort` | size 2; left `sort_bits <=` right; merge takes left branch | [x] |
| 4 | `merge_sort` | size 2; left `sort_bits >` right; merge takes right branch | [x] |
| 5 | `merge_sort` | odd size greater than 1; asymmetric recursive split | [x] |
| 6 | `merge_sort` | even size greater than 2; symmetric recursive split | [x] |
| 7 | `merge_sort` | all `sort_bits` equal; randomized `texture_id`; stable tie path | [x] |
| 8 | `merge_sort` | mixed repeated `sort_bits`; both merge branches and stable ties | [x] |
| 9 | `merge_sort` | many records already nondecreasing by `sort_bits` | [x] |
| 10 | `merge_sort` | many records decreasing by `sort_bits` | [x] |
| 11 | `merge_sort` | `sort_bits` at `INT_MIN`/`INT_MAX` and `texture_id` at 0/`ULLONG_MAX` | [x] |
| 12 | `merge_sort` | large valid length (8,192 records) with allocated non-overlapping buffers | [x] |

The second comparison in
`spritebatch_internal_sprite_less_than_or_equal` checks equal `sort_bits` and
then `texture_id`, but the preceding `sort_bits <=` branch already returns for
all equal keys. Rows 7, 8, and 11 therefore verify the C behavior that
`texture_id` does not affect ordering.
