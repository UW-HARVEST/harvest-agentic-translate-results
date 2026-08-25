# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and therefore has exactly one valid
feature combination:

| # | Rust features | CMake configuration | [ ] |
|---|---------------|----------------------|-----|
| 1 | `--no-default-features` (empty feature set) | default shared target; no CMake options or conditional sources | [x] |

The compile check for that combination is:

```text
cargo check --no-default-features
```

## Runtime Configurations

Rows are derived from all six C-defined dynamic entry points and their
`if`/`while`/`switch` branches. "Randomized" means a fixed-seed sample spanning
negative, zero, positive, and signed integer boundary values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `create_state` | randomized `initial_val`; capacity `0` (`malloc(0)`/zero-size `snprintf`) | [x] |
| 2 | `create_state` | randomized `initial_val`; capacity `1`, producing only a terminator | [x] |
| 3 | `create_state` | randomized `initial_val`; capacity `2..formatted_len`, truncating the formatted state | [x] |
| 4 | `create_state` | randomized `initial_val`; capacity `formatted_len + 1`, exact fit including terminator | [x] |
| 5 | `create_state` | randomized `initial_val`; capacity larger than exact fit | [x] |
| 6 | `destroy_state` | ordinary state with a non-null allocated buffer | [x] |
| 7 | `process_buffer` | empty NUL-terminated buffer (`remaining == 0`) | [x] |
| 8 | `process_buffer` | nonempty buffer with target absent (`memchr == NULL`) | [x] |
| 9 | `process_buffer` | nonempty buffer with exactly one target | [x] |
| 10 | `process_buffer` | nonempty buffer with multiple targets | [x] |
| 11 | `process_buffer` | target byte `0`, `SCHAR_MIN`, or `SCHAR_MAX` across empty/nonempty buffers | [x] |
| 12 | `update_flags` | all 64 combinations of `flag1..flag3` and `mode 0..=7`; counter in `0..30` increments normally | [x] |
| 13 | `update_flags` | all 64 flag/mode combinations; five-bit counter `31` wraps to `0` | [x] |
| 14 | `confuse_types` | operation `0`: write integer bit pattern `1078530011` | [x] |
| 15 | `confuse_types` | operation `1`: finite float interpretations, including negative/zero/subnormal/normal values | [x] |
| 16 | `confuse_types` | operation `1`: NaN, infinity, and values outside the C `int` result range | [x] |
| 17 | `confuse_types` | operation `2`: unsigned interpretation and low-byte mask | [x] |
| 18 | `confuse_types` | operation `3`: four signed-byte interpretations and first-two-byte sum | [x] |
| 19 | `confuse_types` | unsupported negative operation values (no matching `switch` case) | [x] |
| 20 | `confuse_types` | unsupported operation values above `3` (no matching `switch` case) | [x] |
| 21 | `confusion` | `param4 % 4 == 0`; `param3 % 10` is an absent punctuation target; all 64 flag/mode states and randomized `param1` text shapes | [x] |
| 22 | `confusion` | `param4 % 4 == 0`; `param3 % 10 == -3` targets the minus sign; all flag/mode states and randomized `param1` | [x] |
| 23 | `confusion` | `param4 % 4 == 0`; `param3 % 10` targets a digit `0..9` with zero/one/many matches; all flag/mode states and randomized `param1` | [x] |
| 24 | `confusion` | `param4 % 4 == 1`; absent punctuation search target; all flag/mode states and randomized `param1` | [x] |
| 25 | `confusion` | `param4 % 4 == 1`; minus-sign search target; all flag/mode states and randomized `param1` | [x] |
| 26 | `confusion` | `param4 % 4 == 1`; digit search targets with zero/one/many matches; all flag/mode states and randomized `param1` | [x] |
| 27 | `confusion` | `param4 % 4 == 2`; absent punctuation search target; all flag/mode states and randomized `param1` | [x] |
| 28 | `confusion` | `param4 % 4 == 2`; minus-sign search target; all flag/mode states and randomized `param1` | [x] |
| 29 | `confusion` | `param4 % 4 == 2`; digit search targets with zero/one/many matches; all flag/mode states and randomized `param1` | [x] |
| 30 | `confusion` | `param4 % 4 == 3`; absent punctuation search target; all flag/mode states and randomized `param1` | [x] |
| 31 | `confusion` | `param4 % 4 == 3`; minus-sign search target; all flag/mode states and randomized `param1` | [x] |
| 32 | `confusion` | `param4 % 4 == 3`; digit search targets with zero/one/many matches; all flag/mode states and randomized `param1` | [x] |
| 33 | `confusion` | negative unsupported `param4 % 4`; absent punctuation search target; all flag/mode states and randomized `param1` | [x] |
| 34 | `confusion` | negative unsupported `param4 % 4`; minus-sign search target; all flag/mode states and randomized `param1` | [x] |
| 35 | `confusion` | negative unsupported `param4 % 4`; digit search targets with zero/one/many matches; all flag/mode states and randomized `param1` | [x] |
