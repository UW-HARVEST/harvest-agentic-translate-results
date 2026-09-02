# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical grep of every rejection construct

```sh
grep -n "return"                                 c_src/src/lib.c c_src/include/lib.h
grep -niE "assert|NULL|-1|error|errno|if *\(|switch|#if|#ifdef|MAX|MIN" \
                                                 c_src/src/lib.c c_src/include/lib.h
```

Findings:

| construct searched for | occurrences in C |
|------------------------|------------------|
| `return` statements | **1** — `return crc16;` (lib.c:19) |
| `assert` | 0 |
| `return -1` / negative sentinel | 0 |
| `return NULL` / `NULL` checks | 0 |
| `RETURN_ERROR`-style macro | 0 |
| error `enum` / error codes | 0 |
| `errno` use | 0 |
| `if` statements | 0 |
| `switch` statements | 0 |
| `#if` / `#ifdef` | 0 |
| explicit range / bounds check | 0 |
| declared MIN/MAX constants | 0 |

**The C API has no error-reporting channel at all.** `crc16` is a total
function over its declared parameter types: it always executes its two loops
and always returns a `tflac_u16`. There is no in-band sentinel, no out-param
status, and no way for it to signal rejection. The only conditionals in the
entire translation unit are the two loop conditions `len >= 8` and `len--`.

Therefore the rows below are *not* invented error codes. They are the complete
set of **boundary / degenerate / hostile inputs the C actually accepts**, each
with the exact result the C is obliged to produce. "Expected C result" is the
observable contract the Rust must match, and each row has a differential test
in `tests/differential.rs`. For a function with no error channel, "returns the
same value instead of trapping/panicking/UB-ing" *is* the error-path contract —
and it is exactly where a Rust translation can diverge (Rust panics on
arithmetic overflow in debug, on slice index out of range, and on
`unreachable_unchecked`, where C silently wraps).

## Error / boundary surface

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `crc16` | `len == 0`, `d` = valid pointer | Both loops skipped; returns the seed `crc` **unchanged**, `d` never dereferenced. No error. |
| E2 | `crc16` | `len == 0`, `d == NULL` | `len >= 8` false and `len--` false, so `d` is never dereferenced → returns seed `crc` unchanged. Must **not** fault or panic. (Rust must not form `d.add(n)` on null.) |
| E3 | `crc16` | `len == 0`, `d == NULL`, `crc == 0xFFFF` (max seed) | Returns `0xFFFF`. Confirms no seed-dependent path touches `d`. |
| E4 | `crc16` | `len == 0`, `d` = deliberately bogus/unmapped non-null pointer (e.g. `0x1`, `usize::MAX`) | Returns seed unchanged; pointer never read, no fault. |
| E5 | `crc16` | `len--` underflow on the final tail iteration: `len == 1` (and any `len % 8` in `1..=7`) | C's `while (len--)` decrements `len` from 0 to `0xFFFFFFFF` *after* the controlling test already read 0, so the loop exits. The wrap must **not** be observable and must **not** panic (Rust debug-mode `len -= 1` would panic here → must use wrapping semantics). Returns the tail-loop CRC. |
| E6 | `crc16` | `crc == 0xFFFF` entering the tail loop: `crc16 << 8` overflows 16 bits | C promotes to `int`, shifts to `0xFFFF00`, XORs, then **truncates** on assignment to `tflac_u16`. Result is the low 16 bits. Must not panic on overflow (Rust `<<` on `u16` by 8 discards high bits, but a debug-mode overflow check on a widened type would trap). |
| E7 | `crc16` | `crc == 0xFFFF` entering the 8-byte block loop, so `crc >> 8 == 0xFF` and `crc & 0xFF == 0xFF` | Table indices reach the **last** element `[7][255]` / `[6][255]`. Max in-range index; must not be an out-of-bounds index in Rust. |
| E8 | `crc16` | data byte `0xFF` in every table-indexed position (`d[2]`..`d[7]`, and tail `(crc>>8) ^ *d`) | Indices reach 255 on tables `[0]`..`[5]`. Max in-range index; no OOB panic. |
| E9 | `crc16` | tail-loop index `(crc16 >> 8) ^ *d` with **both** operands `0xFF` → index `0x00`, and with `0x00`/`0xFF` → index `0xFF` | Index stays within `0..=255` for all 65536x256 operand pairs; C never bounds-checks, Rust must never panic. Exhaustively probed. |
| E10 | `crc16` | `len` is huge but the buffer is short — i.e. `len` inconsistent with the allocation (classic C misuse) | UB in C (reads past the buffer). **Not differentially testable**: both implementations would read unmapped memory / fault. Documented and deliberately excluded; the test suite always passes a `len` that matches the buffer. |
| E11 | `crc16` | `d == NULL` with `len > 0` | UB in C (null deref → SIGSEGV). **Not differentially testable** — both sides crash the harness. Documented and deliberately excluded. |
| E12 | `crc16` | "out-of-range enum value across the FFI boundary" | **No enum parameter exists** in this API (grep for `enum` in `c_src`: 0 hits). The analogous class here is an out-of-domain *integer* parameter, which is impossible: every bit pattern of `tflac_u32 len` and `tflac_u16 crc` is a valid input, and all of `u16`'s 65536 seed values are covered by E13. |
| E13 | `crc16` | **every** `tflac_u16` seed value `0x0000..=0xFFFF` (exhaustive sweep of the whole parameter domain) | Rust must equal C for all 65536 seeds — the exhaustive version of "one step past a valid range", since no value is out of range. |
| E14 | `crc16` | `len` exactly at the block/tail switch-over boundaries: `len` = 7 (tail only), 8 (one block, empty tail), 9 (one block + 1 tail), 15, 16, 17 | Boundary-correct split between the two loops. Off-by-one in either loop condition diverges here. |
| E15 | `crc16` | zero-length call chained into a non-zero call (result of `len == 0` fed back as the seed) | Seed passthrough must be exact, so `crc16(d,0,c) == c` composes correctly. |

## Gate status (filled in by Phase C)

| row | test | status |
|-----|------|--------|
| E1 | `e1_len_zero_valid_ptr` | [x] pass |
| E2 | `e2_len_zero_null_ptr` | [x] pass |
| E3 | `e3_len_zero_null_ptr_max_seed` | [x] pass |
| E4 | `e4_len_zero_bogus_ptr` | [x] pass |
| E5 | `e5_tail_len_underflow` | [x] pass |
| E6 | `e6_seed_max_tail_shift_overflow` | [x] pass |
| E7 | `e7_seed_max_block_max_table_index` | [x] pass |
| E8 | `e8_all_ff_data_max_table_index` | [x] pass |
| E9 | `e9_tail_index_operand_extremes` | [x] pass |
| E10 | documented as UB / not differentially testable | [x] excluded, justified |
| E11 | documented as UB / not differentially testable | [x] excluded, justified |
| E12 | `e12_no_enum_full_integer_domain` | [x] pass (no enum in API) |
| E13 | `e13_exhaustive_all_65536_seeds` | [x] pass |
| E14 | `e14_loop_split_boundaries` | [x] pass |
| E15 | `e15_seed_passthrough_composition` | [x] pass |
