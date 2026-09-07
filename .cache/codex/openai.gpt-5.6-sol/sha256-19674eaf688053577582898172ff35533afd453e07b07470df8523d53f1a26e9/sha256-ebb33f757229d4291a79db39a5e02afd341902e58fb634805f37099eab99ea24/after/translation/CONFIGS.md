# Configuration-surface table

This table is derived from all six symbols exported by the C shared object,
the null/non-null branches, the `while`/`memchr` result branches, every
`switch` case, the packed-field masks, the `snprintf` capacity boundary, and
the signed `%` operations in `confusion`.

The randomized checks use fixed seeds. A row naming a set (for example all 64
packed flag/mode states) requires the test to traverse the full named set and
randomize the remaining unconstrained bits/values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `create_state` | boundary `initial_val` values; `capacity == 0` (`snprintf` writes nothing) | [x] |
| C02 | `create_state` | boundary/random `initial_val`; `capacity == 1` (only terminating NUL fits) | [x] |
| C03 | `create_state` | boundary/random `initial_val`; `1 < capacity < rendered_length` (prefix truncation) | [x] |
| C04 | `create_state` | boundary/random `initial_val`; `capacity == rendered_length` (last rendered byte truncated) | [x] |
| C05 | `create_state` | boundary/random `initial_val`; `capacity == rendered_length + 1` (exact fit with NUL) | [x] |
| C06 | `create_state` | boundary/random `initial_val`; `capacity > rendered_length + 1` (spare capacity) | [x] |
| C07 | `destroy_state` | non-null state with non-null buffer created by `create_state` | [x] |
| C08 | `process_buffer` | empty NUL-terminated buffer (`remaining == 0`) | [x] |
| C09 | `process_buffer` | non-empty buffer, target absent (`memchr == NULL` on first search), including NUL and high-bit targets | [x] |
| C10 | `process_buffer` | target present exactly once | [x] |
| C11 | `process_buffer` | target present multiple times, including adjacent and first/last positions | [x] |
| C12 | `process_buffer` | embedded NUL: bytes after first NUL are outside the `strlen` search span | [x] |
| C13 | `update_flags` | all 8 flag triplets × all 8 modes; initial counter `0..30`; randomized status/reserved and ignored `param` bits | [x] |
| C14 | `update_flags` | all 8 flag triplets × all 8 modes; initial counter `31` wraps to `0`; randomized status/reserved and ignored `param` bits | [x] |
| C15 | `confuse_types` | operation `0`; arbitrary initial union bits; union overwritten with integer `1078530011` | [x] |
| C16 | `confuse_types` | operation `1`; float bit patterns covering signed zero, finite normal/subnormal, infinities, NaNs, and conversion overflow | [x] |
| C17 | `confuse_types` | operation `2`; arbitrary unsigned values, including every low-byte boundary | [x] |
| C18 | `confuse_types` | operation `3`; arbitrary four-byte values, including signed-char boundaries | [x] |
| C19 | `confusion` | `param4 % 4 == -3`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |
| C20 | `confusion` | `param4 % 4 == -2`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |
| C21 | `confusion` | `param4 % 4 == -1`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |
| C22 | `confusion` | `param4 % 4 == 0`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |
| C23 | `confusion` | `param4 % 4 == 1`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |
| C24 | `confusion` | `param4 % 4 == 2`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |
| C25 | `confusion` | `param4 % 4 == 3`; all 64 low packed-option states; all `param3 % 10` results `-9..9`; boundary/random `param1`; randomized ignored high bits | [x] |

There is no executable target in either build and no Cargo feature is declared.
The sole feature configuration is therefore the empty/default feature set;
it is verified both normally and with `--no-default-features`.
