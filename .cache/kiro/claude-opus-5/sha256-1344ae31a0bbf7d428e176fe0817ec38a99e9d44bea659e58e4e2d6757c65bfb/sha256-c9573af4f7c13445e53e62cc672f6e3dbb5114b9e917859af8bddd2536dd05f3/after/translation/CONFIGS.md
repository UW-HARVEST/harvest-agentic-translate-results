# CONFIGS.md — Configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**Build-time configuration.** None. There is no `#ifdef` in `lib.c`, no
build options in `CMakeLists.txt`, and the Rust crate declares **no cargo
features**, so the default configuration is the only configuration.

**Runtime "options" / modes** (the state the public API can set):

| axis | set by | distinct values the C distinguishes |
|------|--------|-------------------------------------|
| `flags.flag1` | `update_flags(param & 1)` | 0, 1 |
| `flags.flag2` | `update_flags((param & 2) >> 1)` | 0, 1 |
| `flags.flag3` | `update_flags((param & 4) >> 2)` | 0, 1 |
| `flags.mode` (3 bits) | `update_flags((param >> 3) & 7)`; init `3` | 0..7 |
| `flags.counter` (5 bits) | `create_state` init 0; `+1 & 0x1F` per `update_flags` | 0..31, wraps |
| `flags.status` (5 bits) | `create_state` init 15 only | 15 |
| `flags.reserved` (16 bits) | `create_state` init 0 only | 0 |
| `data` interpretation | `confuse_types(operation)` | `0`=write int, `1`=read float, `2`=read uint, `3`=read bytes, other=no-op |

**Input shapes:**

| axis | distinct shapes |
|------|-----------------|
| `capacity` | 0; 1..16 (snprintf truncates); 17 (exact boundary for short values); 18+ full; 128 (`confusion`'s constant); huge; negative (fails) |
| `initial_val` | 0; 1-digit; multi-digit; negative (adds `-`, changes rendered length); `INT_MIN`; `INT_MAX`; bit patterns that reinterpret as NaN/Inf/denormal/normal float; `1078530011` (the magic value written by op 0) |
| `target` (`process_buffer`) | digit present; digit absent; `':'` (occurs twice); letters `S`,`t`,`a`,`e`,`M`,`o`,`d`; `'\0'`; high-bit / negative `char`; a char occurring 3+ times |
| `param3` (`confusion`) | 0..9; ≥10; negative; `INT_MIN` |
| `param4` (`confusion`) | 0..3; ≥4; negative; `INT_MIN` |
| call sequence | single call; `update_flags` repeated 1/2/32/33 times; `confuse_types` op 0 then op 1/2/3 (write-then-reinterpret) |

**Entry points.** All six exports, low-level first:
`create_state`, `destroy_state`, `process_buffer`, `update_flags`,
`confuse_types`, and the composed one-shot wrapper `confusion`.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `create_state` + `destroy_state` | `capacity = 128`, randomized `initial_val` over full `i32`; compare returned struct bytes (flags/data/capacity) and buffer contents | [x] |
| 2  | `create_state` + `destroy_state` | `capacity = 17` (exact-fit boundary), randomized small `initial_val` | [x] |
| 3  | `create_state` + `destroy_state` | `capacity ∈ 1..16` (snprintf truncation) × randomized `initial_val` | [x] |
| 4  | `create_state` + `destroy_state` | `capacity` large (4096, 65536, 1<<20) × randomized `initial_val` | [x] |
| 5  | `create_state` | randomized `initial_val`, `capacity = 20`: verify initial bit-fields `flag1=1 flag2=0 flag3=1 counter=0 mode=3 status=15 reserved=0` are byte-identical in the raw 4-byte storage unit | [x] |
| 6  | `update_flags` | `capacity=128`; `param` sweeping **all 64** low-6-bit patterns (flag1×flag2×flag3×mode cross-product), one call | [x] |
| 7  | `update_flags` | randomized full-range `param` (incl. negatives), one call — exercises arithmetic `>>` | [x] |
| 8  | `update_flags` | randomized `param`, **2 calls** — counter=2, mode from the last call | [x] |
| 9  | `update_flags` | randomized `param`, **32 calls** — counter wraps `31 → 0` | [x] |
| 10 | `update_flags` | randomized `param`, **33 calls** — counter = 1 after wrap | [x] |
| 11 | `update_flags` | `param` sequence randomized per call (mode changes every call) × 40 calls | [x] |
| 12 | `process_buffer` | `capacity=128`, randomized `initial_val`, `target` = each digit `'0'..'9'` (present/absent depends on `initial_val`) | [x] |
| 13 | `process_buffer` | `capacity=128`, `target = ':'` (2 occurrences in `State:N:Mode:M`) | [x] |
| 14 | `process_buffer` | `capacity=128`, `target ∈ {'S','t','a','e','M','o','d','-'}` (literal-text chars, `'-'` only for negative `initial_val`) | [x] |
| 15 | `process_buffer` | `capacity=128`, `target` randomized over **all 256** byte values | [x] |
| 16 | `process_buffer` | `capacity ∈ 1..16` (truncated buffer) × randomized `target` — short/empty haystack | [x] |
| 17 | `process_buffer` | `capacity=128`, called **repeatedly** on the same state (idempotence + `LOG_OPERATION` count sequence) | [x] |
| 18 | `process_buffer` | after `confuse_types` op 0 (buffer untouched by op 0 — confirms no aliasing) | [x] |
| 19 | `confuse_types` | `operation = 0` (write magic `1078530011`) × randomized `initial_val` | [x] |
| 20 | `confuse_types` | `operation = 1` (read as float) × randomized `initial_val` over full `i32` — hits normal, denormal, NaN, ±Inf, huge → `cvttss2si` | [x] |
| 21 | `confuse_types` | `operation = 2` (read as uint) × randomized `initial_val` | [x] |
| 22 | `confuse_types` | `operation = 3` (read as signed bytes) × randomized `initial_val` | [x] |
| 23 | `confuse_types` | `operation = 0` **then** `1` — float read of the magic constant (`1078530011` ≈ `3.14159f`) | [x] |
| 24 | `confuse_types` | `operation = 0` then `2`, and `0` then `3` — write-then-reinterpret chains | [x] |
| 25 | `confuse_types` | full ordered sequence `0,1,2,3` on one state × randomized `initial_val` | [x] |
| 26 | composed pipeline | `create_state` → `update_flags` → `process_buffer` → `confuse_types` → read `flags.counter`/`flags.mode`, driven directly on the low-level exports with randomized inputs (mirrors `confusion` by hand) | [x] |
| 27 | `confusion` | randomized `param1..param4` over the full `i32` range (2000 cases) | [x] |
| 28 | `confusion` | `param3 ∈ 0..9` × `param4 ∈ 0..3` full cross-product (40 combos) × randomized `param1`, `param2` | [x] |
| 29 | `confusion` | `param2` sweeping all 64 low-6-bit patterns × randomized `param1`,`param3`,`param4` | [x] |
| 30 | `confusion` | boundary `param1 ∈ {0, ±1, INT_MIN, INT_MAX, 1078530011, 0x7F7FFFFF, 0x7F800000, 0x7FC00000, 0x00800000, 0x00000001}` × randomized rest | [x] |
| 31 | `confusion` | boundary `param3 ∈ {INT_MIN, -1, 0, 9, 10, INT_MAX}` × boundary `param4 ∈ {INT_MIN, -3..4, INT_MAX}` cross-product | [x] |
| 32 | `confusion` | repeated invocation (state is created/destroyed per call → no cross-call carry-over) | [x] |

## Verification result

All 32 rows pass in `tests/phase_b_valid.rs` (34 tests, one per row plus
`row26b` and a harness smoke test), under both the `--release` and debug
profiles. Each row drives BOTH `.so`s through their exported symbols with many
randomized inputs from a fixed-seed splitmix64 PRNG biased toward boundary and
float-special bit patterns, and compares three things byte-for-byte:

1. every return value,
2. the full 32-bit `flags` word, the 32-bit `data` word, `capacity`, and the
   NUL-terminated `buffer` contents after the operation,
3. all bytes printed to `stdout` (fd 1 is redirected around each call, so the
   `printf`/`snprintf` formatting is compared too, including `%f` rendering of
   `nan` / `inf` / `340282346638528859811704183484516925440.000000`).

Row 5 (`capacity == 0`) compares only the defined observables: `malloc(0)`
returns a non-NULL block and `snprintf(buf, 0, ...)` writes nothing, so the
buffer contents are indeterminate in the C and must not be compared.

### Coverage beyond what `create_state` can produce

`create_state` always initializes `flags` to `0x00007b05` and always writes a
`"State:N:Mode:M"` buffer, so per-wrapper tests cannot observe whether the
bit-field read-modify-write preserves `status`/`reserved`, nor how the `memchr`
loop behaves on arbitrary bytes. `tests/phase_c_errors.rs` therefore builds
`ProcessState` values by hand with libc `malloc`
(`common::make_state`) to reach:

- arbitrary 32-bit `flags` contents (`update_flags_preserves_unrelated_bitfields`),
- a NULL `buffer` (rows 8, 10),
- buffers of arbitrary bytes including `0xFF`/`0x80`, repeats, and adjacent
  matches (`process_buffer_randomized_arbitrary_buffers`).

### Mutation testing — the suite is not vacuous

`mutation_check.py` injects known C→Rust bug classes into a scratch copy of the
crate, rebuilds it, and runs the whole suite against the mutated `.so` via
`RUST_SO_PATH`. Detected (failing-test counts):

| injected bug | tests that caught it |
|--------------|----------------------|
| saturating `as` cast instead of `cvttss2si` semantics | 10 |
| `rem_euclid(10)` instead of C truncating `%` for `param3` | 6 |
| `rem_euclid(4)` instead of C truncating `%` for `param4` | 6 |
| zero-extending negative `capacity` (loses the malloc failure) | 4 |
| `& 0xFFF` instead of `& 0xFF` in `confuse_types` op 2 | 11 |
| `mode * 4` instead of `mode * 3` in `confusion` | 7 |
| off-by-one on `remaining` in the `memchr` loop | 9 |
| zero-extending `bytes[i]` instead of sign-extending | 11 |
| initial `status` 14 instead of 15 | 28 |
| `snprintf` given `capacity - 1` | 6 |
| 5-bit counter masked with `0x0F` instead of `0x1F` | 4 |
| `status` bit-field placed at bit 10 instead of 11 | 34 |

Three further mutations produced 0 failures and were confirmed to be
**semantically equivalent**, not test gaps:

- logical vs. arithmetic `param >> 3`: the subsequent `& 0x7` keeps only bits
  3..5, which the sign fill never touches.
- signed vs. unsigned byte comparison in the `memchr` scan: byte equality is
  independent of signedness.
- computing `float_val * 100` in `f64` then narrowing: `100.0` needs 7 mantissa
  bits and an `f32` needs 24, so the product fits exactly in `f64`'s 53-bit
  significand; rounding once to `f32` gives the same value as an `f32` multiply.
