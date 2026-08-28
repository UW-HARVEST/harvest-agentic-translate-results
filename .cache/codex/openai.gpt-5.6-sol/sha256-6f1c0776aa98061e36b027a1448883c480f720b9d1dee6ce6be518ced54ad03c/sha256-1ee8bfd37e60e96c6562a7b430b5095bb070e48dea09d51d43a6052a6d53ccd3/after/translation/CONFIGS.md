# Configuration Surface

This table is derived from all nine C-defined dynamic entry points and every
runtime branch in `c_src/src/lib.c`. Randomized rows use bounded integers so
the C oracle does not encounter signed-overflow undefined behavior. There are
no Cargo features, compile-time `#ifdef` branches, byte-order options, or
element-type options; the sole build configuration is the featureless crate.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `shift_array` | size 2, `positions = 1` (minimum shifting shape) | [x] |
| 2 | `shift_array` | size greater than 2, `positions = 1` | [x] |
| 3 | `shift_array` | size greater than 3, `1 < positions < size - 1` | [x] |
| 4 | `shift_array` | size greater than 2, `positions = size - 1` | [x] |
| 5 | `process_string` | empty NUL-terminated byte string (`*str == 0`) | [x] |
| 6 | `process_string` | one non-NUL byte followed by NUL | [x] |
| 7 | `process_string` | many non-NUL bytes followed by NUL | [x] |
| 8 | `apply_bitmask` | `operation = 0` (AND `0xf0`), values include negative/zero/positive/boundaries | [x] |
| 9 | `apply_bitmask` | `operation = 1` (AND `0x0f`), values include negative/zero/positive/boundaries | [x] |
| 10 | `apply_bitmask` | `operation = 2` (OR `0xaa`), values include negative/zero/positive/boundaries | [x] |
| 11 | `apply_bitmask` | `operation = 3` (XOR `0x55`), values include negative/zero/positive/boundaries | [x] |
| 12 | `apply_bitmask` | operation outside `0..=3`, including `-1`, `4`, and integer boundaries; default returns value | [x] |
| 13 | `init_matrix` | writable contiguous 3-by-4 `int` matrix | [x] |
| 14 | `compare_allocations` | `val1 <= 0`; no `+10` branch, randomized `val2` | [x] |
| 15 | `compare_allocations` | `val1 > 0`; takes `+10` branch, randomized `val2` | [x] |
| 16 | `arity4` | `param1 % 4 == 0`, `param1 > 0`; `param3 = 0`, `param4 = 0` | [x] |
| 17 | `arity4` | `param1 % 4 == 0`, `param1 > 0`; `param3 != 0`, `param4 = 0` | [x] |
| 18 | `arity4` | `param1 % 4 == 0`, `param1 > 0`; `param3 = 0`, `param4 != 0` | [x] |
| 19 | `arity4` | `param1 % 4 == 0`, `param1 > 0`; `param3 != 0`, `param4 != 0` | [x] |
| 20 | `arity4` | `param1 = 0`; `param3 = 0`, `param4 = 0` | [x] |
| 21 | `arity4` | `param1 = 0`; `param3 != 0`, `param4 = 0` | [x] |
| 22 | `arity4` | `param1 = 0`; `param3 = 0`, `param4 != 0` | [x] |
| 23 | `arity4` | `param1 = 0`; `param3 != 0`, `param4 != 0` | [x] |
| 24 | `arity4` | `param1 % 4 == 0`, `param1 < 0`; `param3 = 0`, `param4 = 0` | [x] |
| 25 | `arity4` | `param1 % 4 == 0`, `param1 < 0`; `param3 != 0`, `param4 = 0` | [x] |
| 26 | `arity4` | `param1 % 4 == 0`, `param1 < 0`; `param3 = 0`, `param4 != 0` | [x] |
| 27 | `arity4` | `param1 % 4 == 0`, `param1 < 0`; `param3 != 0`, `param4 != 0` | [x] |
| 28 | `arity4` | `param1 % 4 == 1` (positive); `param3 = 0`, `param4 = 0` | [x] |
| 29 | `arity4` | `param1 % 4 == 1` (positive); `param3 != 0`, `param4 = 0` | [x] |
| 30 | `arity4` | `param1 % 4 == 1` (positive); `param3 = 0`, `param4 != 0` | [x] |
| 31 | `arity4` | `param1 % 4 == 1` (positive); `param3 != 0`, `param4 != 0` | [x] |
| 32 | `arity4` | `param1 % 4 == 2` (positive); `param3 = 0`, `param4 = 0` | [x] |
| 33 | `arity4` | `param1 % 4 == 2` (positive); `param3 != 0`, `param4 = 0` | [x] |
| 34 | `arity4` | `param1 % 4 == 2` (positive); `param3 = 0`, `param4 != 0` | [x] |
| 35 | `arity4` | `param1 % 4 == 2` (positive); `param3 != 0`, `param4 != 0` | [x] |
| 36 | `arity4` | `param1 % 4 == 3` (positive); `param3 = 0`, `param4 = 0` | [x] |
| 37 | `arity4` | `param1 % 4 == 3` (positive); `param3 != 0`, `param4 = 0` | [x] |
| 38 | `arity4` | `param1 % 4 == 3` (positive); `param3 = 0`, `param4 != 0` | [x] |
| 39 | `arity4` | `param1 % 4 == 3` (positive); `param3 != 0`, `param4 != 0` | [x] |
| 40 | `arity4` | negative `param1` with remainder `-1`, `-2`, or `-3` (default bitmask); `param3 = 0`, `param4 = 0` | [x] |
| 41 | `arity4` | negative `param1` with remainder `-1`, `-2`, or `-3` (default bitmask); `param3 != 0`, `param4 = 0` | [x] |
| 42 | `arity4` | negative `param1` with remainder `-1`, `-2`, or `-3` (default bitmask); `param3 = 0`, `param4 != 0` | [x] |
| 43 | `arity4` | negative `param1` with remainder `-1`, `-2`, or `-3` (default bitmask); `param3 != 0`, `param4 != 0` | [x] |
| 44 | `arity2` | two parameters; randomized `param1` spans all reachable bitmask/sign classes | [x] |
| 45 | `arity3` | three parameters with `param3 = 0`; randomized `param1` spans all reachable bitmask/sign classes | [x] |
| 46 | `arity3` | three parameters with `param3 != 0`; randomized `param1` spans all reachable bitmask/sign classes | [x] |
| 47 | `arity` | low byte of ABI `len` is exactly 2; dispatches to `arity2` and reads two integers | [x] |
| 48 | `arity` | low byte of ABI `len` is exactly 3, third parameter zero; dispatches to `arity3` | [x] |
| 49 | `arity` | low byte of ABI `len` is exactly 3, third parameter nonzero; dispatches to `arity3` | [x] |
| 50 | `arity` | low byte of ABI `len` is `4..=255`; dispatches to `arity4`, reads only four integers; includes 4 and 255 | [x] |
| 51 | `arity` | ABI `int len` is outside unsigned-byte range but low byte is 2 (for example 258 or -254) | [x] |
| 52 | `arity` | ABI `int len` is outside unsigned-byte range but low byte is 3 (for example 259 or -253) | [x] |
| 53 | `arity` | ABI `int len` is outside unsigned-byte range but low byte is `4..=255`; dispatches to `arity4` | [x] |

Unchecked rows: **0**.

