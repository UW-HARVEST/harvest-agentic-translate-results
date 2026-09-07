# CONFIGS.md — Phase A configuration-surface table (VALID inputs)

Derived mechanically from the branches the C actually takes. There are no
`#ifdef`s, no compile-time options, no runtime option/flag setters and no
global state in `c_src/src/lib.c`; the "configuration" of this library is
therefore (a) which of the **six** exported entry points is called, (b) the
mutable `StringBuffer` state that `create_buffer`/`append_to_buffer` build up,
and (c) the input *shapes* that each `if`/`switch` distinguishes.

Axes enumerated from the source:

* **A1 `create_buffer(initial_capacity)`** — `initial_capacity` shape:
  `0` (degenerate, `malloc(0)`), `1`, small (< what will be appended),
  `32` (the value `buffapp` uses), exact-fit, large.
* **A2 `append_to_buffer(buffer, str)`** — the single branch is
  `required_capacity > buffer->capacity`:
  * no-grow (`length + strlen(str) + 1 <= capacity`)
  * grow (`>` capacity, `realloc` to `required_capacity * 2`)
  * boundary: `required_capacity == capacity` (no grow) vs
    `required_capacity == capacity + 1` (grow)
  * `str` shape: empty (`strlen == 0`), 1 byte, many bytes, embedded-NUL-free
    random bytes; repeated appends so `length` accumulates and a second grow
    happens.
* **A3 `destroy_buffer(buffer)`** — `buffer` non-NULL with non-NULL `data`
  (the valid path).
* **A4 `get_operation_name(op_code)`** — `switch` arms `0,1,2,3` (valid),
  `default` (in ERRORS.md row 10).
* **A5 `perform_operation(a, b, operation)`** — `strcmp` chain selects
  `add`/`subtract`/`multiply`/`divide`; operand shapes: zeros, positives,
  negatives, mixed signs, `INT_MAX`/`INT_MIN` (signed wrap on `+ - *`),
  exact division, truncating division, negative-operand truncation
  (C truncates toward zero).
* **A6 `buffapp(p1,p2,p3,p4)`** — two data-dependent `switch` selections
  (`param1 % 4` and `param3 % 4`, each of which can be
  `0,1,2,3,-1,-2,-3` because C `%` keeps the sign of the dividend) crossed
  with the final `intermediate3 != 0` branch. Also the only entry point that
  writes to **stdout**, so its bytes on fd 1 are part of the observable output.
* **A7 composed pipeline** — `create_buffer` -> N x `append_to_buffer` ->
  read back `data`/`capacity`/`length` -> `destroy_buffer`, driven at the
  *low level* (not through `buffapp`), with the C-created buffer passed to the
  Rust functions and vice-versa (the structs are ABI-identical and the
  allocator is shared, so cross-calling is a real consumer pattern and
  exercises field layout).

Each row is checked off only after it passes with **many randomized inputs**
(fixed seed, see `tests/common/mod.rs::Rng`, seed `0x5EED_1234_ABCD_0001`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `get_operation_name` | `op_code == 0` -> `"add"` | [x] |
| 2 | `get_operation_name` | `op_code == 1` -> `"subtract"` | [x] |
| 3 | `get_operation_name` | `op_code == 2` -> `"multiply"` | [x] |
| 4 | `get_operation_name` | `op_code == 3` -> `"divide"` | [x] |
| 5 | `perform_operation` | `"add"`, randomized `(a,b)` incl. `INT_MAX`/`INT_MIN` wrap | [x] |
| 6 | `perform_operation` | `"subtract"`, randomized `(a,b)` incl. wrap | [x] |
| 7 | `perform_operation` | `"multiply"`, randomized `(a,b)` incl. wrap | [x] |
| 8 | `perform_operation` | `"divide"`, `b != 0`, exact division | [x] |
| 9 | `perform_operation` | `"divide"`, `b != 0`, truncating division, both operands positive | [x] |
| 10 | `perform_operation` | `"divide"`, `b != 0`, mixed/negative signs (C truncation toward zero) | [x] |
| 11 | `perform_operation` | operation string obtained from `get_operation_name` of the *other* library (cross-library pointer, valid arm) | [x] |
| 12 | `create_buffer` | `initial_capacity == 0` -> non-NULL, `capacity == 0`, `length == 0` | [x] |
| 13 | `create_buffer` | `initial_capacity == 1` -> `capacity == 1`, `data[0] == 0` | [x] |
| 14 | `create_buffer` | `initial_capacity == 32` (the `buffapp` value) | [x] |
| 15 | `create_buffer` | `initial_capacity` large (randomized 1..=65536) | [x] |
| 16 | `create_buffer` + `destroy_buffer` | round-trip, valid buffer, no leak/crash | [x] |
| 17 | `append_to_buffer` | no-grow: `capacity` generous, single short `str` | [x] |
| 18 | `append_to_buffer` | no-grow boundary: `required_capacity == capacity` exactly | [x] |
| 19 | `append_to_buffer` | grow boundary: `required_capacity == capacity + 1` -> `realloc` to `2*required` | [x] |
| 20 | `append_to_buffer` | grow from `capacity == 0` (degenerate buffer) | [x] |
| 21 | `append_to_buffer` | empty `str` (`strlen == 0`), no-grow | [x] |
| 22 | `append_to_buffer` | empty `str` on a `capacity == 0` buffer (`required == 1 > 0` -> grows) | [x] |
| 23 | `append_to_buffer` | long `str` (much larger than `capacity`) -> single big grow | [x] |
| 24 | `append_to_buffer` (A7) | repeated appends (randomized count 1..=64, randomized lengths) -> multiple grows; compare full `data` bytes + `capacity` + `length` after every step | [x] |
| 25 | `append_to_buffer` (A7) | same as 24 but starting `capacity` randomized in `0..=8` so growth happens on nearly every step | [x] |
| 26 | cross-library (A7) | buffer created by C `create_buffer`, appended by Rust `append_to_buffer`, destroyed by C `destroy_buffer` (and the mirror image) | [x] |
| 27 | `buffapp` | `param1 % 4 == 0` x `param3 % 4 == 0`, randomized params, return value + stdout | [x] |
| 28 | `buffapp` | full cross-product of `param1 % 4` x `param3 % 4` over all 7x7 = 49 reachable residue pairs (`0,1,2,3,-1,-2,-3`), randomized params per cell, return value + stdout | [x] |
| 29 | `buffapp` | `intermediate3 == 0` branch (`intermediate1 == 0` or `intermediate2 == 0`) -> `result = p1+p2+p3+p4` | [x] |
| 30 | `buffapp` | `intermediate3 != 0` branch -> `result / intermediate3`, incl. truncating and negative results | [x] |
| 31 | `buffapp` | fully randomized `int` quadruples (1000 iterations, fixed seed), excluding the SIGFPE quadruples of ERRORS.md row 17 | [x] |
| 32 | `buffapp` | extreme params: `0`, `1`, `-1`, `INT_MAX`, `INT_MIN` cross-product (5^4 = 625 cells), excluding SIGFPE cells | [x] |
| 33 | `buffapp` | stdout byte-for-byte comparison (fd 1 captured per call) for every case in rows 27-32 | [x] |
| 34 | all six | called repeatedly in one process, interleaved C/Rust, to catch hidden global state (there is none in C; assert none in Rust) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **The project builds no binary
driver**, so the "compare C and Rust binary stdout" item is not applicable.
`buffapp`'s stdout is nevertheless compared byte-for-byte (row 33), which
covers the same observable surface.

## Row -> test mapping (all passing)

| CONFIGS rows | test |
|--------------|------|
| 1-4 | `tests/valid_paths.rs::cfg_01_04_get_operation_name_valid_arms` |
| 5-7 | `cfg_05_07_perform_operation_add_sub_mul_randomized` (81 corner pairs + 6000 randomized per arm) |
| 8-10 | `cfg_08_10_perform_operation_divide_randomized` (1000 exact + 144 corners + 4000 randomized) |
| 11 | `cfg_11_perform_operation_with_cross_library_name_pointer` |
| 12-16 | `cfg_12_16_create_buffer_shapes` (2500+ capacities) |
| 17-23 | `cfg_17_23_append_single_shapes` |
| 24 | `cfg_24_repeated_appends_randomized` (200 sequences x up to 64 appends) |
| 25 | `cfg_25_repeated_appends_tiny_initial_capacity` |
| 26 | `cfg_26_cross_library_buffer_ownership` |
| 27 | `tests/buffapp_diff.rs::cfg_27_buffapp_add_add` |
| 28 | `cfg_28_buffapp_residue_cross_product` (49 cells x 40 randomized inputs) |
| 29 | `cfg_29_buffapp_intermediate3_zero_branch` (400 randomized + 11 deterministic) |
| 30 | `cfg_30_buffapp_intermediate3_nonzero_branch` (600 randomized) |
| 31 | `cfg_31_buffapp_fully_randomized` (1000 quadruples) |
| 32 | `cfg_32_buffapp_extreme_cross_product` (5^4 extremes + 24x24 boundary grid) |
| 33 | stdout captured and compared inside `buffapp_diff()`, used by rows 27-32; harness self-check in `cfg_33_stdout_capture_is_meaningful` |
| 34 | `cfg_34_no_hidden_global_state` (50 repetitions of a 200-step interleaved script) |

`buffapp` inputs that reach `INT_MIN / -1` are excluded here (they raise
SIGFPE) and covered out-of-process by `ERRORS.md` rows 14 and 17.

Every row above passed against **all four** Rust artifacts (release/debug x
default/no-default features) and against the C library built at both `-O0`
(the documented build) and `-O2`.
