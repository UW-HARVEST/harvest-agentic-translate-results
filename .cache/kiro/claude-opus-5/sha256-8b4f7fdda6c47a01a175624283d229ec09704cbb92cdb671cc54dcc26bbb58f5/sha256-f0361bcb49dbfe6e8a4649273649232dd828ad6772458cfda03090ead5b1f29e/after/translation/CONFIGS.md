# CONFIGS.md — configuration surface table (valid inputs)

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` (`switch`, `if`, `%`, and the constants). The
library has no build-time configuration: no `#ifdef` other than the standard
header guards, no runtime option struct, no global mode flag. Its
"configuration" is therefore entirely (a) which operation is selected and
(b) the shape/state of the history buffer.

* **A — operation selector** (`Operation`, an `int`): the 5 valid enumerators
  `OP_ADD=1, OP_MULTIPLY=2, OP_SUBTRACT=3, OP_DIVIDE=4, OP_MODULO=5`, plus the
  `default:` bucket (`0`, `6`, negative, `INT_MIN`, `INT_MAX`).
* **B — operand shape**: `b == 0` vs `b != 0` (only `divide`/`modulo` branch on
  it), sign of each operand, and the boundary values `0, ±1, INT_MIN, INT_MAX`
  (overflow of `+`, `-`, `*`; truncation-toward-zero of `/` and `%`).
* **C — history state** at entry to `perform_computation_with_history`:
  `*history == NULL` (lazy allocate + count reset) vs non-NULL; and
  `*history_count` relative to the cap 10: `0`, `1..9`, exactly `10`, `>10`.
* **D — number of history entries recorded per `mathop` call**: `mathop` calls
  `perform_computation_with_history` twice, so the process-wide static count
  goes 2, 4, 6, 8, 10, 10, 10 … across successive `mathop` calls (a
  call-ordering axis, observable only on stdout).
* **E — `mathop` validation-char branch**: `param1 % 128` inside `49..53`
  (`'1'..'5'`, `is_valid == true`) vs outside (`is_valid == false`).
* **F — `mathop` operation pair**: `selected_op = param3 % 5 + 1` (5 positive
  residues + negative residues) × `second_op = (param4+1) % 5 + 1`, i.e. the
  cross product of first and second operation, with `param4` doubling as the
  second operand.
* **G — `is_valid_operation` char domain**: the full signed-`char` range
  `-128..127`, with `'1'..'5'` accepted.
* **H — `allocate_results` count**: `0`, `1`, `10` (the value `mathop` uses),
  other small positive.
* **I — struct-layout observation**: `ComputationResult` is written through the
  returned pointer, so its 24-byte layout (`int` value, 4 pad, `time_t`, `int`
  status, 4 tail pad) is part of the ABI and is compared byte-for-byte.

## Public entry points

All 12 exported symbols are in scope. `mathop` is the only one declared in
`include/lib.h` (the convenience / one-shot wrapper); the other 11 are the
lower-level entry points and are driven **directly**, not only through
`mathop`.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `is_valid_operation` | axis G: exhaustive over all 256 `char` bit patterns (`-128..127`), raw return byte compared | [x] |
| 2  | `get_operation_priority` | axis A: each valid enumerator 1..5 | [x] |
| 3  | `get_operation_priority` | axis A: `default` bucket + boundaries `0, -1, 6, INT_MIN, INT_MAX`, plus randomized `int`s (wrapping `*10`) | [x] |
| 4  | `add_operation` | axis B: `{0, ±1, INT_MIN, INT_MAX}²` boundary grid + randomized pairs; third param varied (must be ignored) | [x] |
| 5  | `multiply_operation` | axis B: same grid + randomized pairs (overflow wrap) | [x] |
| 6  | `subtract_operation` | axis B: same grid + randomized pairs (overflow wrap) | [x] |
| 7  | `divide_operation` | axis B, `b != 0`: boundary grid + randomized pairs, incl. mixed signs (truncation toward zero), `INT_MIN/1`, `INT_MAX/-1` | [x] |
| 8  | `modulo_operation` | axis B, `b != 0`: boundary grid + randomized pairs, incl. negative dividend (C remainder sign) | [x] |
| 9  | `select_operation` | axis A: each of 1..5 → the returned fn-pointer's *identity* (which of the 5 exported ops it equals) compared C vs Rust | [x] |
| 10 | `select_operation` | axis A: `default` bucket `{0, 6, 7, -1, -5, INT_MIN, INT_MAX}` + randomized ints → must resolve to `add_operation` on both | [x] |
| 11 | `select_operation` | axis A×B: the returned pointer *invoked* with randomized `(a,b,unused)` and results compared (verifies dispatch, not just identity) | [x] |
| 12 | `get_computation_timestamp` | no inputs; value compared C vs Rust (arithmetic `>>29` of `time()`) | [x] |
| 13 | `allocate_results` | axis H: `count = 1` → NULL-ness compared, 24 bytes/elem all zero, writable | [x] |
| 14 | `allocate_results` | axis H: `count = 10` (the value `mathop` uses) → NULL-ness + 240 zero bytes | [x] |
| 15 | `allocate_results` | axis H: `count = 0` → NULL-ness compared | [x] |
| 16 | `allocate_results` | axis H: randomized `count ∈ 1..=64` → NULL-ness + all-zero payload | [x] |
| 17 | `allocate_results` + axis I | struct layout probe: write `{value, timestamp, status}` into slot *k* of a C-allocated and a Rust-allocated buffer via `perform_computation_with_history`, compare all 24·n bytes | [x] |
| 18 | `perform_computation_with_history` | axis C=`NULL` history: caller passes `*history == NULL` and a *nonzero* `*history_count`; both must allocate and reset count to 0, then record at slot 0. Return value, final count, and full buffer bytes compared | [x] |
| 19 | `perform_computation_with_history` | axis C=non-NULL, count `0` → record lands in slot 0; op axis A ∈ 1..5, operands randomized | [x] |
| 20 | `perform_computation_with_history` | axis C=non-NULL, count `1..9` (mid-buffer) × op axis A ∈ 1..5 × randomized operands; buffer bytes + count compared | [x] |
| 21 | `perform_computation_with_history` | axis C=non-NULL, count `9` → last legal slot, count becomes 10 | [x] |
| 22 | `perform_computation_with_history` | axis A `default` bucket (op ∉ 1..5) × randomized operands → `add_operation` behaviour recorded | [x] |
| 23 | `perform_computation_with_history` | axis A ∈ {4,5} with `b == 0` → value `0` recorded, `STATUS_SUCCESS` | [x] |
| 24 | `perform_computation_with_history` | repeated calls on one buffer: 12 sequential calls (2 past the cap) with randomized ops/operands; the whole 10-slot buffer and the count trajectory compared after every call | [x] |
| 25 | `mathop` | axis E=valid (`param1 % 128 ∈ 49..53`) × axis F: `param3 % 5 ∈ 0..4` × `param4` randomized — return value **and** stdout bytes compared | [x] |
| 26 | `mathop` | axis E=invalid × axis F: full 5×5 cross product of (`param3 mod 5`, `param4 mod 5`) with randomized magnitudes — return value + stdout | [x] |
| 27 | `mathop` | axis F with negative `param3`/`param4` (out-of-range `Operation` from `%`), incl. `param3 = -1..-5`, `param4 = -1..-6` | [x] |
| 28 | `mathop` | axis B boundaries as parameters: `param1, param2, param4 ∈ {0, ±1, INT_MIN, INT_MAX}` (division-by-zero and overflow paths reached through the wrapper) | [x] |
| 29 | `mathop` | axis D: 8 consecutive calls in one process → stdout `History entries:` trajectory 2,4,6,8,10,10,10,10 compared side by side | [x] |
| 30 | `mathop` | axis A×B fully randomized: 400 random `(param1..param4)` quadruples, return value + stdout compared | [x] |
| 31 | binary driver | *not applicable* — `c_src/CMakeLists.txt` builds only `add_library(SHARED)`; there is no executable target, and `translation/Cargo.toml` declares `crate-type = ["cdylib"]` with no `[[bin]]`. Nothing to compare stdout-for-stdout at the binary level. | [x] |
| 32 | feature combinations | *not applicable* — `translation/Cargo.toml` has no `[features]` section, so the only configuration is the default (empty) feature set. Verified by grepping the manifest. | [x] |

## Test mapping

Rows 1–24 → `tests/phase_b_valid.rs`, one `#[test] fn rowNN_…` per row.
Rows 25–30 → `tests/phase_b_mathop.rs::phase_b_mathop_all_rows` (a single test
fn, in its own test binary: each `.so` owns a `static history_count` that only
stays in step if C and Rust are called the same number of times, and the fd-1
redirection used to capture `printf` output must not race with libtest's own
progress writes).
Rows 31–32 (both N/A) → asserted by `verify.sh`, which greps `Cargo.toml` for
`[features]` / `[[bin]]` and `c_src/CMakeLists.txt` for `add_executable`.

Each row runs its configuration against many inputs from a fixed-seed
xorshift64\* generator (`Rng::new(<constant>)`), biased to include `0`, `±1`,
`INT_MIN`, `INT_MAX` and small magnitudes alongside the full 32-bit range, so a
row is only checked off after passing across randomized inputs rather than one
hand-picked value.

Where a row's inputs would hit `INT_MIN / -1` — genuine UB that raises SIGFPE in
the C code — the value is nudged and the trap is instead asserted
out-of-process in `ERRORS.md` rows 7–8.

## Run

```
cd translation && ./verify.sh
```

Rebuilds both `.so` files, diffs `nm -D`, and runs all 48 tests against the dev
**and** release Rust artifacts (`MATHOP_RUST_SO` selects which one the harness
loads). `cargo test` alone is not sufficient: it does not rebuild a
`crate-type = ["cdylib"]` artifact, so the harness refuses to run against a
`.so` older than `src/lib.rs`.
