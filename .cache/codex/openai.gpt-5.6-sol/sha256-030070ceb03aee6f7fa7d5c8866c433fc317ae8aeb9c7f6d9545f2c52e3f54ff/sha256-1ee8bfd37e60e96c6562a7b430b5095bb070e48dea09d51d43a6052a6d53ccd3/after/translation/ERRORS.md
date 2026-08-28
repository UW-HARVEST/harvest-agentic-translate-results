# Error surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, `abort`, `exit`, error enums, null checks, comparisons, and range
conditions in `../c_src/include/lib.h` and `../c_src/src/lib.c`.

The C API has no error return type and the implementation contains no explicit
input rejection, assertion, null check, or error branch. Consequently, the
source-derived rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

## Generic FFI boundaries

These cases are required independently of explicit C rejection branches.

| # | function | boundary | expected C result | status |
|---|----------|----------|-------------------|--------|
| G1 | `update_frame_header` | `t == NULL` | No rejection exists; dereference has undefined behavior and terminates the isolated probe on this build | [x] |

There are no length arguments, enum-typed FFI parameters, documented numeric
ranges, or error sentinels. All `u32`/`u8` field values, including zero,
maximum values, and channel modes outside `0..=3`, are valid inputs. They are
covered by the configuration matrix; `channel_mode` is explicitly reduced
modulo four by the C implementation.
