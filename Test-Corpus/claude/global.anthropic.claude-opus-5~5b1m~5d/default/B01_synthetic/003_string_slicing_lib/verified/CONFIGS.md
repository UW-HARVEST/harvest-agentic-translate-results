# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

Mechanically derived from `c_src/include/slicing.h` (the full public API) and
from every branch in `c_src/src/slicing.c`.

## Public entry points (complete)

The header exports exactly one function; there are no convenience wrappers and
no lower level to reach past — `slice` *is* the lowest-level entry point.

```c
int slice(char *mystr, int *start_ptr, int *stop_ptr);
```

## Axes the C code actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A1 `start_ptr` presence | `NULL` ⇒ `start = 0`; non-NULL ⇒ `start = *start_ptr` + range check | `if (start_ptr)` |
| A2 `stop_ptr` presence | `NULL` ⇒ `stop = len` (`size_t`→`int` truncation); non-NULL ⇒ `stop = *stop_ptr` + two checks | `if (stop_ptr) … else stop = len;` |
| A3 string length `len` | `0` (empty), `1`, small, large (multi-KiB) | `strlen`, and every comparison against `len` |
| A4 `start` position | `0` (first), interior, `len` (boundary — accepted, `>` not `>=`) | `start > len` |
| A5 `stop` position | `start + 1` (minimal non-empty), interior, `len` (boundary — accepted) | `stop > len`, `stop <= start` |
| A6 slice width `stop - start` | `0` (only reachable via A2=NULL with `start == len`), `1`, interior, `len` (whole string) | `printf("%.*s", stop - start, …)` precision |
| A7 byte content of `mystr` | plain ASCII; bytes containing `%` and `\` (must be passed as *data*, never as a format string); embedded `\n`; high bytes `0x80–0xFF` (invalid UTF-8 — must survive verbatim); bytes `0x01–0x1F` | `printf("%.*s", …)` operand |
| A8 return path | `0` (printed a slice) vs `1` (rejected — see `ERRORS.md`) | `return 0` / `return 1` |

There are no runtime option setters, no global/`static` state, no `#ifdef`
configuration, no enums, and no byte-order or element-width axes in this
library — `CMakeLists.txt` defines no compile options and `slicing.c` has no
preprocessor conditionals.

## Configuration rows (cross-product of A1×A2 × data shape, pruned to what C distinguishes)

Each row is run against **many randomized inputs with a fixed seed**
(deterministic xorshift PRNG in `tests/common/mod.rs`) and compared byte-for-byte
between the C `.so` and the Rust `.so` — return code **and** captured `stdout`.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL` — whole string; random ASCII of random length 1..64 | `c1_both_null_ascii` | [x] |
| C2 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`, `len == 0` (empty string) ⇒ precision `0`, prints just `\n` | `c2_both_null_empty` | [x] |
| C3 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`, `len == 1` | `c3_both_null_single_char` | [x] |
| C4 | `slice` | `start_ptr = NULL`, `stop_ptr = NULL`, large string (1 KiB … 8 KiB, random bytes `0x01–0xFF`) — crosses libc `printf` buffer size | `c4_both_null_large` | [x] |
| C5 | `slice` | `start_ptr` set, `stop_ptr = NULL`, `start` random in `[0, len]` ⇒ suffix slice (includes the `start == len` boundary ⇒ width `0`) | `c5_start_only_random` | [x] |
| C6 | `slice` | `start_ptr` set to `0`, `stop_ptr = NULL` — explicit-zero start behaves like `NULL` start | `c6_start_zero_vs_null` | [x] |
| C7 | `slice` | `start_ptr` set to exactly `len`, `stop_ptr = NULL` — accepted boundary, empty output | `c7_start_at_len` | [x] |
| C8 | `slice` | `start_ptr = NULL`, `stop_ptr` set, `stop` random in `[1, len]` ⇒ prefix slice | `c8_stop_only_random` | [x] |
| C9 | `slice` | `start_ptr = NULL`, `stop_ptr` set to exactly `len` (accepted boundary) ⇒ whole string | `c9_stop_at_len` | [x] |
| C10 | `slice` | `start_ptr` **and** `stop_ptr` set, random `0 <= start < stop <= len` ⇒ interior slice | `c10_both_set_random` | [x] |
| C11 | `slice` | both set, minimal width: `stop == start + 1` at random positions ⇒ single character | `c11_both_set_width_one` | [x] |
| C12 | `slice` | both set, maximal width: `start == 0`, `stop == len` ⇒ whole string via explicit bounds | `c12_both_set_full_range` | [x] |
| C13 | `slice` | both set on a large string (1 KiB … 8 KiB), random interior window | `c13_both_set_large` | [x] |
| C14 | `slice` | any of A1×A2, data containing `%` / `%s` / `%n` / `%%` — verifies the payload is never treated as a format string | `c14_percent_payload` | [x] |
| C15 | `slice` | any of A1×A2, data containing high bytes `0x80–0xFF` (invalid UTF-8) | `c15_high_bytes` | [x] |
| C16 | `slice` | any of A1×A2, data containing embedded `\n`, `\r`, `\t` and other control bytes `0x01–0x1F` | `c16_control_bytes` | [x] |
| C17 | `slice` | slice window ending exactly at the NUL terminator vs. strictly before it (`%.*s` precision semantics) | `c17_window_vs_terminator` | [x] |
| C18 | `slice` | repeated / interleaved calls on the same buffer (`NULL,NULL` → `start` → `stop` → both) to prove `slice` keeps no state and does not mutate `mystr`, `*start_ptr`, or `*stop_ptr` | `c18_stateless_repeat` | [x] |
| C19 | `slice` | exhaustive sweep: for every `len` in `0..=24`, every `start` in `0..=len` × every `stop` in `0..=len`, plus the `NULL` variants of each pointer — full valid **and** invalid cross-product | `c19_exhaustive_small` | [x] |
| C20 | `slice` | `mystr` buffer with trailing bytes **after** the NUL terminator (so `len` < allocation) — confirms nothing past the terminator is read or printed | `c20_bytes_after_nul` | [x] |

## Feature combinations

`Cargo.toml` declares no `[features]`; the default and `--no-default-features`
builds are byte-identical. `./run_all.sh` runs the whole suite under both, in
debug and release (release additionally exercises `panic = "abort"`).
