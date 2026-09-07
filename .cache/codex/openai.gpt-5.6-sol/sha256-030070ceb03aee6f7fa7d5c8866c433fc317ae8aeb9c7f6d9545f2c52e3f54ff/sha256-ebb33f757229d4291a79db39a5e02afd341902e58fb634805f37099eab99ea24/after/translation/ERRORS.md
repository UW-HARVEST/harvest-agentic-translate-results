# Error surface

Mechanical searches over `../c_src/include/` and `../c_src/src/` found no
`RETURN_ERROR`, error return, `return -1`, `return NULL`, assertion, null
check, explicit rejection, or error enum. The only public function returns
`void` and unconditionally dereferences its argument.

## Explicit C rejection branches

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are zero explicit rejection rows.

## Mandatory generic FFI boundaries

These are required Phase C probes even though the C source does not reject
them explicitly.

| # | function | boundary input/condition | expected C behavior | status |
|---|----------|--------------------------|---------------------|--------|
| G1 | `update_frame_header` | `t == NULL` | no sentinel exists; observed external call terminates by signal after the unconditional dereference | [x] |
| G2 | `update_frame_header` | all numeric fields zero | accepted; writes the same bytes in C and Rust | [x] |
| G3 | `update_frame_header` | all width-limited fields at `UINT32_MAX`/`UINT8_MAX` | accepted; writes the same bytes in C and Rust | [x] |
| G4 | `update_frame_header` | `channel_mode == TFLAC_CHANNEL_MODE_COUNT` (`4`, one past the named enum values) | accepted and reduced modulo 4 | [x] |
| G5 | `update_frame_header` | `channel_mode == UINT8_MAX` (unnamed enum value across FFI) | accepted and reduced modulo 4 | [x] |
| G6 | `update_frame_header` | one past the largest listed values: block size `32769`, sample rate `882001`, bit depth `33` | accepted and processed by default branches | [x] |

There are no length parameters, so zero-length and oversized-length probes are
not applicable.
