# CONFIGS.md — Configuration-surface table (valid inputs)

Axes the C code actually branches on, derived from `c_src/src/lib.c`:

**A. Runtime options / modes**
* `complexmode`'s `mode` argument — a `switch` with `case 1..4` + `default`
  (line 115). This is the only "mode" selector in the library.
* `permissions` bitmask. Inside `complexmode` it is hard-wired to `0644`, but
  the low-level entry points `check_permissions(perms, required)` and
  `safe_add(a, b, perms)` take it from the caller, so the whole permission
  lattice is reachable from outside: `READ_PERM 0400`, `WRITE_PERM 0200`,
  `EXEC_PERM 0100`, and the derived masks `0600` (`safe_add`'s requirement) and
  `0100` (mode 4's test).

**B. Input shapes**
* integer operand magnitude/sign: zero, small positive, small negative, values
  that make `a+b` / `a*b` overflow `int`, `INT_MIN`, `INT_MAX`.
* `copy_and_sum` element count: `0`, `1`, `3` (what `complexmode` mode 3 uses),
  many, negative; plus the pointer being non-NULL vs NULL.
* string shape for `create_result_string` / `compare_operations`: empty, short,
  long enough to make `snprintf` truncate at 64 bytes, embedded `%`
  format-specifier characters, high bytes ≥ 0x80, and the equal / less /
  greater orderings for `strcmp`.

**C. Entry points** — all seven exported symbols, low-level first. The tests
call `check_permissions`, `safe_add`, `create_result_string`,
`multiply_with_log`, `copy_and_sum`, `compare_operations` **directly** through
the `.so`, not only through the `complexmode` convenience wrapper.

There are no `#ifdef` compile-time branches and no cargo features, so there is a
single build configuration.

Every row is driven with many randomized inputs from a fixed-seed xorshift PRNG
(seed `0x2024_0C0D_E5EE_D001`), plus the hand-picked boundary values listed, and
both the return value **and** the captured `stdout` bytes are compared.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `check_permissions` | `required == 0` (vacuously satisfied for every `perms`, including `perms == 0` and negative `perms`) | [x] |
| 2 | `check_permissions` | single-bit `required` ∈ {`0400`,`0200`,`0100`} × `perms` covering set/clear of that bit | [x] |
| 3 | `check_permissions` | multi-bit `required` (`0600`, `0644`, `0777`) × `perms` with all / some / none of the bits set | [x] |
| 4 | `check_permissions` | full random 32-bit `perms` × `required`, incl. negative values and `INT_MIN`/`INT_MAX` as raw bit patterns | [x] |
| 5 | `safe_add` | `perms` grants `0600` (e.g. `0600`,`0644`,`0777`,`-1`) → returns `a+b`, random `a`,`b` | [x] |
| 6 | `safe_add` | `perms` grants `0600` × operands that overflow `int` (`INT_MAX`+1, `INT_MIN`-1, `INT_MAX`+`INT_MAX`) | [x] |
| 7 | `safe_add` | `perms` missing `0400` only, missing `0200` only, missing both → prints rejection, returns `0` | [x] |
| 8 | `create_result_string` | short ASCII `op` × random `val` (positive, negative, `0`, `INT_MIN`, `INT_MAX`) — compare the 64-byte returned buffer contents | [x] |
| 9 | `create_result_string` | empty `op` (`""`) | [x] |
| 10 | `create_result_string` | `op` long enough that `snprintf` truncates at the 64-byte limit (boundary: `op` lengths that land the NUL exactly at 62/63/64 and beyond) | [x] |
| 11 | `create_result_string` | `op` containing `%d`/`%s`/`%%` (passed as data, not format) and bytes ≥ 0x80 | [x] |
| 12 | `multiply_with_log` | random `a`,`b` → check return value **and** the out-parameter string bytes | [x] |
| 13 | `multiply_with_log` | operands whose product overflows `int` (`INT_MAX*2`, `INT_MIN*-1`, `65536*65536`) and products that are `0` or negative | [x] |
| 14 | `copy_and_sum` | `count == 0` with a valid pointer | [x] |
| 15 | `copy_and_sum` | `count == 1` | [x] |
| 16 | `copy_and_sum` | `count == 3` (the shape `complexmode` mode 3 uses) with random values | [x] |
| 17 | `copy_and_sum` | `count` large (16, 64, 1024) with random values, incl. values whose running sum overflows `int` | [x] |
| 18 | `copy_and_sum` | all elements `INT_MAX` / all `INT_MIN` (guaranteed accumulator wraparound) | [x] |
| 19 | `compare_operations` | equal strings (identical bytes, incl. both empty) → `0` | [x] |
| 20 | `compare_operations` | unequal strings differing at the first byte / a middle byte / by length (prefix), both orderings — raw `strcmp` value must match | [x] |
| 21 | `compare_operations` | strings differing only in a byte ≥ 0x80 (glibc `strcmp` compares as `unsigned char`) | [x] |
| 22 | `compare_operations` | random byte strings of random lengths from the fixed-seed PRNG | [x] |
| 23 | `complexmode` | `mode == 1` (addition), random `value1`,`value2`; `value3` ignored | [x] |
| 24 | `complexmode` | `mode == 1` with `value1+value2` overflowing `int` | [x] |
| 25 | `complexmode` | `mode == 2` (multiplication + log), random `value1`,`value2` — the printed `Mode 2: Operation: multiply, Value: N` line is part of the diff | [x] |
| 26 | `complexmode` | `mode == 2` with `value1*value2` overflowing `int`, and with a product of `0` / negative (changes the printed digit string) | [x] |
| 27 | `complexmode` | `mode == 3` (array sum of all three values), random values | [x] |
| 28 | `complexmode` | `mode == 3` with a sum that overflows `int` | [x] |
| 29 | `complexmode` | `mode == 4` (complex): `permissions=0644` lacks `0100`, so the `else` branch `v1+v2+v3` is taken — random values | [x] |
| 30 | `complexmode` | `mode == 4` with values that overflow in the `else` branch | [x] |
| 31 | `complexmode` | every mode × the extreme operand triple (`INT_MIN`,`INT_MAX`,`0`) permutations | [x] |
| 32 | `complexmode` | full random sweep: random `mode` in `[-8, 12]` × random operand triples (covers valid modes and the `default` arm interleaved, exercising the trailing `Operation performed:` line for each `operation` string) | [x] |

## Row → test mapping

Every row is covered by the identically-numbered test in
`tests/phase_b_valid.rs` (`row01_…` … `row32_…`), each of which compares both
the return value and the captured stdout of the C `.so` and the Rust `.so`.
Volume per row is set by `CTORUST_N` (default 400; verified at 8000).

`tests/bulk_sweep.rs` additionally re-covers all seven entry points as batched
property sweeps (`CTORUST_BULK`, default 100 000 inputs per entry point;
verified at 1 000 000, ~14 M compared calls) where an entire batch of calls
runs under one stdout redirection and the concatenated output must match
byte-for-byte.

`tests/harness_selftest.rs` is the negative control: it proves `capture` really
records the loaded library's stdout, that `diff` / `diff_batch` / `diff_oom`
actually fail on a return-value, stdout-only, or out-parameter divergence, and
that the two `.so` files are distinct objects with distinct symbol addresses —
so a passing row above is not a vacuous pass.

## Build configurations

`Cargo.toml` has no `[features]` table, so the feature axis is a single point.
The axis that does vary for the shipped artifact is the cargo profile, because
`[profile.release]` sets `panic = "abort"`. `run_matrix.sh` runs `cargo check`,
`cargo build`, the `nm -D` symbol diff and the whole test suite for
`{dev, release} × {default, --no-default-features, --all-features}` — all six
combinations pass.
