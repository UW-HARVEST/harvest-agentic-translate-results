# ERRORS.md — Phase A error / rejection surface table

Every distinct way `c_src/src/lib.c` rejects, guards against, bails out of, or
clamps its input. Derived mechanically by grepping every `if`, `return`,
`break`, `switch` default fall-through, `NULL` comparison and bit-mask
constant in the C source. No row is invented; each cites the C line.

`operation`/`param` values are plain `int` across the FFI boundary, so any
`int` is a real input the C accepts — including values with no matching
`switch` case (the C-enum-style out-of-range class).

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|--------------------------------------------|-------------------|
| E1 | `create_state` | `malloc(sizeof(ProcessState))` returns `NULL` (`lib.c:60`) — forced with an `LD_PRELOAD` shim that fails exactly one `malloc(24)` | prints `Error: Failed to allocate memory for state\n`, returns `NULL` |
| E2 | `create_state` | `capacity < 0` → `malloc((size_t)(int64)capacity)` is an enormous request and returns `NULL` (`lib.c:76,78`); e.g. `capacity = -1, -128, INT_MIN` | prints `Error: Failed to allocate buffer\n`, `free(state)`, returns `NULL` |
| E3 | `create_state` | the *buffer* `malloc` fails for an otherwise valid `capacity` (`lib.c:78`) — forced with the `LD_PRELOAD` shim; also reached by a `capacity` the heap cannot serve | prints `Error: Failed to allocate buffer\n`, `free(state)`, returns `NULL` |
| E4 | `create_state` | `capacity == 0` → `malloc(0)` succeeds (glibc, non-`NULL`), `snprintf(buf, 0, …)` writes **nothing** (`lib.c:84`) | returns non-`NULL` state; buffer left untouched (contents indeterminate) |
| E5 | `create_state` | `capacity` too small for `"State:%d:Mode:%d"` (e.g. 1..15) → `snprintf` truncates | returns non-`NULL`; buffer holds a truncated, NUL-terminated prefix (`capacity-1` chars) |
| E6 | `destroy_state` | `state == NULL` (`lib.c:91`) | no-op, no output, no crash |
| E7 | `destroy_state` | `state != NULL` but `state->buffer == NULL` (`lib.c:92`) | `free(state)` only, buffer not freed, no output |
| E8 | `process_buffer` | `state == NULL` (`lib.c:100`, first disjunct) | prints `Error: Null pointer in process_buffer\n`, returns `-1` |
| E9 | `process_buffer` | `state != NULL` but `state->buffer == NULL` (`lib.c:100`, second disjunct) | prints `Error: Null pointer in process_buffer\n`, returns `-1` |
| E10 | `process_buffer` | `strlen(state->buffer) == 0` → `remaining == 0`, loop body never entered (`lib.c:109`) | returns `0`, no `Operation:` lines |
| E11 | `process_buffer` | `memchr` finds no (further) occurrence → `found == NULL` → `break` (`lib.c:112,113`) | returns the count accumulated so far (0 when the very first probe fails) |
| E12 | `process_buffer` | `target == '\0'` (0): the NUL terminator is *outside* `remaining = strlen(buf)` | returns `0` |
| E13 | `process_buffer` | `target` with the sign bit set (`char` is signed on x86-64: `-1 … -128`); `memchr` compares `(unsigned char)c` | returns `0` for an ASCII buffer, no match |
| E14 | `update_flags` | `state == NULL` (`lib.c:127,128`) | returns immediately (`void`), **no** `Debug:`/`Bit fields` output |
| E15 | `update_flags` | `counter` would exceed its 5-bit field: `(counter + 1) & 0x1F` (`lib.c:131`) | wraps `31 → 0`; only the low 5 bits are kept |
| E16 | `update_flags` | `param` bits above bit 5 are ignored: `mode = (param >> 3) & 0x7` (`lib.c:135`) | `mode` clamped to `0..7` |
| E17 | `update_flags` | `param < 0` → `param >> 3` is an **arithmetic** shift (GCC) | `mode = ((param >> 3) & 7)`, e.g. `param = -1 → mode = 7` |
| E18 | `confuse_types` | `state == NULL` (`lib.c:144,145`) | returns `0`, no output |
| E19 | `confuse_types` | `operation` has no `case`: `4`, `5`, `-1`, `INT_MAX`, `INT_MIN`, … (`switch` at `lib.c:150` has only 0/1/2/3 and no `default`) | returns `0`, **no** output at all |
| E20 | `confuse_types` | `operation == 2`: value is clamped by `& 0xFF` (`lib.c:163`) | result in `0..255` regardless of `uint_val` |
| E21 | `confuse_types` | `operation == 1` and `float_val * 100.0f` is NaN / ±Inf / outside `int32` (`lib.c:158`) | x86-64 `cvttss2si` "integer indefinite" → `INT_MIN` (`-2147483648`) |
| E22 | `confuse_types` | `operation == 3`: `bytes[0] + bytes[1]` on **signed** `char` (`lib.c:170`) | sum in `-256 … 254`, sign-extended, may be negative |
| E23 | `confusion` | `create_state(param1, 128)` returns `NULL` (`lib.c:187,188`) — forced with the `LD_PRELOAD` shim, for *both* underlying failure sites (`malloc(24)` and `malloc(128)`) | returns `-1` (after the four `Debug: paramN` lines) |
| E24 | `confusion` | `param3 < 0` → `param3 % 10` is negative (truncating remainder) → `search_char = '0' + negative` < `'0'` (`lib.c:193`) | search char in `')' (39) … '9' (57)`; non-digit chars find 0 matches |
| E25 | `confusion` | `param4 < 0` → `param4 % 4 ∈ {-3,-2,-1,0}` → negative operations hit the `switch` fall-through of row E19 (`lib.c:197`) | `confusion_result == 0` |
| E26 | `confusion` | signed `int` overflow of `result += confusion_result` / `+ found*10` / `+ counter*5` / `+ mode*3` (`lib.c:195..201`) when `confuse_types` returns a near-`INT_MAX`/`INT_MIN` value | wraps two's-complement (GCC `add`); see the reachability note below |

## Status

| row | differential test | passing |
|-----|-------------------|---------|
| E1  | `err_e1_state_malloc_failure` (forced OOM, 25 param combinations) | [x] |
| E2  | `err_e2_negative_capacity` | [x] |
| E3  | `err_e3_buffer_malloc_failure_forced` (+ `generic_zero_and_oversized_lengths`) | [x] |
| E4  | `err_e4_zero_capacity` | [x] |
| E5  | `err_e5_truncating_capacity` | [x] |
| E6  | `err_e6_destroy_null` | [x] |
| E7  | `err_e7_destroy_state_with_null_buffer` | [x] |
| E8  | `err_e8_process_buffer_null_state` | [x] |
| E9  | `err_e9_process_buffer_null_buffer` | [x] |
| E10 | `err_e10_process_buffer_empty_string` | [x] |
| E11 | `err_e11_process_buffer_no_match` | [x] |
| E12 | `err_e12_process_buffer_nul_target` | [x] |
| E13 | `err_e13_process_buffer_negative_target` | [x] |
| E14 | `err_e14_update_flags_null_state` | [x] |
| E15 | `err_e15_counter_wrap` | [x] |
| E16 | `err_e16_mode_mask` | [x] |
| E17 | `err_e17_negative_param_arithmetic_shift` | [x] |
| E18 | `err_e18_confuse_types_null_state` | [x] |
| E19 | `err_e19_confuse_types_out_of_range_operation` | [x] |
| E20 | `err_e20_confuse_types_uint_mask` | [x] |
| E21 | `err_e21_confuse_types_float_out_of_range` | [x] |
| E22 | `err_e22_confuse_types_signed_bytes` | [x] |
| E23 | `err_e23_confusion_create_state_failure` (both failure sites) | [x] |
| E24 | `err_e24_confusion_negative_param3` | [x] |
| E25 | `err_e25_confusion_negative_param4` | [x] |
| E26 | `err_e26_confusion_result_overflow` | [x] |

### Reachability note for E26

The largest value `confuse_types` can return is bounded by the `f32` grid: the
biggest `float` strictly below `2^31` is `2147483520 == INT_MAX - 127`, and the
other three addends are bounded by `found*10 <= 100`, `counter*5 == 5` and
`mode*3 <= 21`, i.e. at most `126 < 127`. So `result` cannot actually overflow
*through `confusion`*. `err_e26_confusion_result_overflow` therefore drives the
reachable extremes at both ends (near-`INT_MAX` products found by scanning the
`f32` bit patterns, and the `INT_MIN` indefinite result) and also drives the
same additions through the low-level API where `counter` can reach 31. The Rust
translation uses `wrapping_add`/`wrapping_mul` throughout, matching GCC's plain
`add`/`imul`, so the behaviour is identical if it ever does become reachable.

### Allocator parity

`alloc_trace_parity` additionally compares the *allocator* behaviour of the two
implementations through the same `LD_PRELOAD` shim: identical `malloc` count,
`free` count and total bytes for the same work (`2 / 2 / 152` for a
`create_state(_, 128)` + `destroy_state` round trip), which is what rules out a
leak, a double free, or a differently-sized allocation.

Generic FFI boundary cases additionally covered in `tests/phase_c_errors.rs`:
null pointers on every pointer-taking entry point, zero / oversized lengths,
one-past-the-range enum-ish values (`operation = -1, 4, 5, INT_MIN, INT_MAX`),
and `INT_MIN`/`INT_MAX` for every `int` parameter of every entry point.
