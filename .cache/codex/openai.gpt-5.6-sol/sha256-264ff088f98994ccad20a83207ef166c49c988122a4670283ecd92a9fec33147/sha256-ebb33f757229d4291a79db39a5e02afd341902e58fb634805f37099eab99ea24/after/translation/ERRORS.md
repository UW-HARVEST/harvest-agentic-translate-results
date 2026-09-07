# Error-Surface Table

Scope: externally reachable rejection behavior through every symbol declared in
`include/lib.h` and exported by the C shared library. The only such entry point
is `jumpnode(int, int, int, int)`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `jumpnode` | `operation_mode == 0001`; `find_node_by_id(node_id) == NULL` because file-static `node_count` is zero and no public initializer exists | `STATUS_ERROR \| 0020` = `0022` (18) | [x] |
| E2 | `jumpnode` | `operation_mode == 0002`; `find_node_by_id(node_id) == NULL` because file-static `node_count` is zero and no public initializer exists | `STATUS_ERROR \| 0040` = `0042` (34) | [x] |
| E3 | `jumpnode` | `operation_mode == 0004`; `find_node_by_id(node_id) == NULL` because file-static `node_count` is zero and no public initializer exists | `STATUS_ERROR \| 0100` = `0102` (66) | [x] |
| E4 | `jumpnode` | `operation_mode` is any integer other than `0001`, `0002`, `0003`, or `0004`, including zero, one-past-range `0005`, negative values, and `INT_MIN`/`INT_MAX` | `STATUS_ERROR \| 0200` = `0202` (130) | [x] |

## Mechanical audit of non-public conditions

These source conditions were found by the same grep audit but are not rows in
the ABI error table because no exported C symbol can construct or invoke them:

- `find_node_by_id` returns `NULL` after scanning `node_count`; its externally
  reachable consequences are E1, E2, and E3.
- `add_node` returns `STATUS_ERROR` when `node_count >= MAX_NODES` (100).
  `add_node` and its sole caller `initialize_test_data` are both file-static,
  and `initialize_test_data` is never called.
- A missing parent in mode `0001` breaks traversal rather than rejecting the
  call. It is unreachable in the shipped library because storage stays empty.
- `safe_double_to_int` clamps above `2147483647.0` and below
  `-2147483648.0`; clamping is not rejection, and all paths to it require a
  node that the public ABI cannot add.

Pointer/length generic boundaries do not apply: the public ABI has four
by-value `int` parameters and no pointer, enum-typed, or length parameters.

All rows pass under the default build and `--no-default-features`.
