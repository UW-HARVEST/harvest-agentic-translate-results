# Error Surface

The mechanical scan covered `c_src/src/` and `c_src/include/` for
`RETURN_ERROR`, `return -1`, `return NULL`, `assert`, explicit null checks,
range checks, error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows: `update_frame_header` returns `void`, has no return
statement, assertion, null check, range rejection, error enum, or error
sentinel. All scalar bit patterns accepted by the public field types are
processed.

## Generic FFI Boundaries

| # | boundary | C behavior | coverage |
|---|----------|------------|----------|
| G1 | `update_frame_header(NULL)` | Undefined by C; the built reference faults while unconditionally dereferencing `t` | [x] |
| G2 | Zero and maximum scalar values | Accepted; covered as valid configurations in Phase B | [x] |
| G3 | Zero/oversized lengths | Not applicable: the API has no pointer-length pair or length argument | N/A |
| G4 | Out-of-range public enum | Not applicable: the public ABI exposes `channel_mode` as `uint8_t`, not an enum; all 256 values are reduced modulo 4 | [x] |
