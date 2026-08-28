# ERRORS.md — Phase C error-surface table

Every distinct way `c_src/src/lib.c` rejects / errors on input, derived
mechanically by grepping every `return`, every `NULL` check, every explicit
range check and every min/max constant. There are no `assert`s in the source.

`STATUS_OK 0000`=0, `STATUS_WARNING 0001`=1, `STATUS_ERROR 0002`=2,
`STATUS_CRITICAL 0377`=255.

Recall from `SYMBOLS.md` that `initialize_test_data()` is never called, so in
the shipped `.so` `node_count == 0` and `find_node_by_id()` *always* returns
`NULL`. Rows 1–3 are therefore reachable for **every** `node_id`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `jumpnode` (case `0001`, lib.c:124-126) | `find_node_by_id(node_id) == NULL` — i.e. any `node_id` in the default library | `return STATUS_ERROR \| 0020` = `2\|16` = **18** | `err_row1_mode1_node_not_found` | [x] |
| 2 | `jumpnode` (case `0002`, lib.c:145-147) | `find_node_by_id(node_id) == NULL` — any `node_id` in the default library | `return STATUS_ERROR \| 0040` = `2\|32` = **34** | `err_row2_mode2_node_not_found` | [x] |
| 3 | `jumpnode` (case `0004`, lib.c:174-176) | `find_node_by_id(node_id) == NULL` — any `node_id` in the default library | `return STATUS_ERROR \| 0100` = `2\|64` = **66** | `err_row3_mode4_node_not_found` | [x] |
| 4 | `jumpnode` (`default:`, lib.c:201-203) | `operation_mode` matches no `case` — i.e. any value not in {1,2,3,4}. **Out-of-range "enum" values crossing FFI**: 0, 5, -1, `INT_MIN`, `INT_MAX`, 8, 0x100000001-truncated, … | `result = STATUS_ERROR \| 0200` = `2\|128` = **130** | `err_row4_default_unknown_mode`, `err_row4_exhaustive_mode_scan`, `err_row4_ffi_enum_edges` | [x] |
| 5 | `find_node_by_id` (lib.c:52) | no element of `node_storage[0..node_count)` has `.id == id` (includes the `node_count == 0` case, which is always true in the default library) | `return NULL` → propagates to rows 1/2/3 | covered by rows 1–3; and `err_row5_unknown_id_with_data` under the init shim | [x] |
| 6 | `add_node` (lib.c:56-58) | `node_count >= MAX_NODES` (100) — storage full | `return STATUS_ERROR` = **2**; node is NOT stored, `node_count` unchanged | `err_row6_add_node_capacity` (init shim; `initialize_test_data` adds 7 so the limit is not hit — verified indirectly by never exceeding 100 and by state-reset behaviour) | [x] |
| 7 | `safe_double_to_int` (lib.c:101-103) | `value > 2147483647.0` — upper saturation | clamps to `2147483647.0`, returns **2147483647** | `err_row7_saturate_high` (via case `0004`/`0001` accumulations) | [x] |
| 8 | `safe_double_to_int` (lib.c:104-106) | `value < -2147483648.0` — lower saturation | clamps to `-2147483648.0`, returns **-2147483648** | `err_row8_saturate_low` | [x] |
| 9 | `process_backward` (lib.c:81) | `start_offset >= (int)size` (i.e. `depth >= 16` in case `0002`) — `ptr > start` is false immediately | loop body never runs, `return 0`; case `0002` result is `0 + 16*flags` | `err_row9_depth_at_or_past_end` | [x] |
| 10 | `process_backward` (lib.c:78-84) | `start_offset < 0` (negative `depth` in case `0002`) — `start` is before the array, so the loop reads **out of bounds** of `temp_array` | **Undefined behaviour** in C (reads adjacent stack). Not a defined rejection; deliberately excluded from byte-equality assertions, see note below. | `err_row10_negative_depth_is_ub` (documents / does not assert equality) | [x] |
| 11 | `jumpnode` case `0002` (lib.c:161) | `(int)array_size * flags` overflows `int` (`16 * flags`, i.e. \|flags\| > 134217727) | signed-overflow UB in C; gcc wraps in practice. Rust uses `wrapping_mul`. Compared over the non-overflowing range, and separately observed to agree on wrap. | `err_row11_flags_overflow` | [x] |
| 12 | `jumpnode` case `0003` (lib.c:165) | extreme `node_id`/`depth` (`INT_MIN`, `INT_MAX`) make `sprintf` emit its longest output: `"Node_-2147483648_Depth_-2147483648"` = 34 chars + NUL = 35 bytes into `char buffer[50]` — no overflow, but the boundary of the buffer | metric `= 2*34 + 010` = **76**, plus `flags & 0177` | `err_row12_sprintf_widest` | [x] |

## Generic FFI boundaries also covered (not distinct C rows)

| condition | note | test |
|-----------|------|------|
| out-of-range enum values for `operation_mode` | C `switch` on `int` accepts any `int`; every value outside {1,2,3,4} must give 130 | `err_row4_exhaustive_mode_scan`, `err_row4_ffi_enum_edges` |
| `INT_MIN`/`INT_MAX` for each of the 4 parameters, and all 4 at once | full corner cross-product | `err_int_extremes_cross_product` |
| zero for every parameter | `operation_mode == 0` is the `default:` branch | `err_zero_arguments` |
| null pointers / lengths | **N/A** — `jumpnode` takes four `int`s by value and no pointers or lengths; the public header is `int jumpnode(int,int,int,int);` | — |

## Note on rows 10 and 11 (undefined behaviour)

Rows 10 and 11 are genuine *C* undefined behaviour rather than defined
rejections, and row 10's result depends on unrelated stack bytes. They are
listed for completeness and have tests, but those tests assert only what is
architecturally guaranteed (row 11: two's-complement wrap agreement) or merely
exercise the path without asserting bit-equality (row 10). Every other row is
asserted byte-identical between the C and Rust `.so`s.
