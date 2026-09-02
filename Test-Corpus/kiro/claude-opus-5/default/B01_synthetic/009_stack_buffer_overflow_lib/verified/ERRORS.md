# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

Mechanically derived from `c_src/src/driver.c`. The library is `void`-only: it
has **no return codes, no error enums, no `errno` use, no `assert`, no
`return -1` / `return NULL`**. Every rejection is expressed as *a branch that
prints a fixed diagnostic line instead of doing the work*, plus one silent
null-pointer guard. The complete set of conditionals in the C source is:

```
src/driver.c:31   if (line != NULL)                     -> printLine null guard
src/driver.c:46   if (data >= 0) ... else               -> bad(): negative reject
src/driver.c:65   if (data >= 0) ... else               -> goodG2B(): dead branch (data == 7)
src/driver.c:83   if (data >= 0 && data < (10)) ... else -> goodB2G(): range reject
```

Constants that act as limits: `10` (both the `int buffer[10]` extent and
`goodB2G`'s upper bound). `bad()` has **no** upper bound — that omission is the
injected defect and is row 8/9 below.

Observable result == exact stdout byte stream (and exit status).

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | silent: returns without emitting a single byte |
| 2 | `printIntLine` | *(none — no validation at all; every `int` incl. `INT_MIN`/`INT_MAX` is accepted)* | prints `"%d\n"` of the value |
| 3 | `bad` | `data < 0` (e.g. `-1`) | prints exactly `ERROR: Array index is negative.\n`, nothing else |
| 4 | `bad` | `data == INT_MIN` (extreme negative) | same as row 3 |
| 5 | `good` → `goodB2G` | `data < 0` (e.g. `-1`) | `goodG2B` output (9×`0\n` with `1\n` at index 7) then `ERROR: Array index is out-of-bounds\n` |
| 6 | `good` → `goodB2G` | `data == INT_MIN` | same as row 5 |
| 7 | `good` → `goodB2G` | `data >= 10` (`10`, `11`, `100`, `INT_MAX`) | `goodG2B` output then `ERROR: Array index is out-of-bounds\n` |
| 8 | `bad` | `data == 10` or `data == 11` — **missing** upper-bound check; out-of-bounds write lands in frame slack / the dead loop counter | NOT rejected: writes OOB, then prints ten `0\n` lines (the `1` is never visible) |
| 9 | `bad` | `data >= 12` — OOB write reaches the saved frame pointer / return address | NOT rejected: undefined behaviour. Observed with GCC `-O0`: process dies (`SIGSEGV`, core dumped) for 12…~99, and *sometimes* survives for larger offsets that land in unused caller stack. Result depends on the **caller's** frame layout, not on the library. See "UB caveat". |
| 10 | `driver` | `badData < 0` | `good` section, then `Calling bad()...\n` + row-3 message + `Finished bad()\n` |
| 11 | `driver` | `goodData < 0` or `goodData >= 10` | row 5/7 message inside the `good` section; `bad` section unaffected |
| 12 | `goodG2B` | `data >= 0` is **always** true (`data = 7` is hardcoded) | the `else` printing `ERROR: Array index is negative.` is **dead code, unreachable** — asserted unreachable by testing that `good()` never emits the "negative" text |

## Generic FFI boundary cases also covered in Phase C

| # | case | expected C result | test |
|---|------|-------------------|------|
| G1 | `printLine(NULL)` | no output (row 1) | [x] |
| G2 | `printLine("")` — zero-length string | one empty line (`\n`) | [x] |
| G3 | `printLine` with a non-NUL-terminated-adjacent / very long (64 KiB, 100 KB) buffer | whole buffer + `\n` | [x] |
| G4 | `printLine` with a string containing `%s %d %n` | printed literally (the string is an *argument*, never a format) | [x] |
| G5 | `printLine` with embedded non-UTF-8 / high bytes (0x80..0xFF) | bytes passed through unchanged | [x] |
| G6 | `printIntLine(INT_MIN)` / `INT_MAX` / `0` / `-0` | `-2147483648`, `2147483647`, `0`, `0` | [x] |
| G7 | out-of-range "enum" values across FFI | **N/A — the API declares no enum type.** Every parameter is `int`/`const char *`, so the whole 32-bit range is a legal input; that whole range is what rows 3–9 partition. A 19-value wild-int vector (`INT_MIN`, `INT_MIN+1`, `-2147000000`, `-65536`, `-256`, `-2`, `-1`, `0`, `1`, `9`, `10`, `11`, `255`, `256`, `65535`, `65536`, `1000000`, `INT_MAX-1`, `INT_MAX`) is fed to `printIntLine`, `good`, `driver` and (restricted to the deterministic `<= 11` subset) `bad`. | [x] |
| G8 | one step past each boundary: `bad(-1)`, `bad(0)`, `bad(9)`, `bad(10)`; `good(-1)`, `good(0)`, `good(9)`, `good(10)`; `driver` on both parameters | per rows 3/7/8 | [x] |

## UB caveat (row 9)

`bad(data)` for `data >= 12` is an out-of-bounds stack write whose effect is
determined by the *compiled frame layout of the caller chain*, not by the
library's own semantics. Disassembly of the C `.so` (`objdump -d`) shows
`buffer` at `-0x30(%rbp)` and `i` at `-0x4(%rbp)` inside a `0x40`-byte frame:

* `data == 10` → writes `-0x8(%rbp)`, unused slack → benign.
* `data == 11` → writes `i`, which the following loop immediately re-initialises
  to `0` → benign.
* `data == 12,13` → clobbers the saved `%rbp`; `data >= 14` → clobbers the return
  address → crash on `leave`/`ret`.

No Rust program can reproduce "corrupt my caller's saved frame pointer" both
soundly and portably, so the translation instead backs the 10-element buffer
with a larger zeroed region: the overrun is absorbed, and the ten in-bounds
elements printed afterwards are identical to C's. This means Rust and C agree
byte-for-byte on the entire **deterministic** domain `INT_MIN..=11` and diverge
only in the region where the C program has no defined behaviour at all
(`>= 12`). Rows 1–8 and 10–12 are asserted byte-identical; row 9 is tested with
a documented, explicitly-recorded expectation rather than an equality assert.

## Status

Every row 1–12 and G1–G8 has a passing differential test in
`tests/differential.rs::phase_c_errors` — the C `.so` and the Rust `.so` are
compared on the exact stdout byte stream **and** the exit status / fatal signal,
each call made in a fresh child process that `dlopen`s exactly one of the two
libraries. Rows 3, 5, 7, 10, 11 additionally assert the exact diagnostic text
(including that `goodB2G`'s message has *no* trailing period while `bad`'s does),
so a same-shape-but-different-string rejection cannot pass.

Row 9 is the one row not asserted as equal, by construction: the C program has no
defined behaviour there. It is measured and reported on every run, and what *is*
asserted is that Rust stays deterministic (ten `0\n` lines, exit 0) instead of
crashing. Empirically the C `.so` survives `bad(12)`, `bad(13)`, `bad(20)`,
`bad(32)`, `bad(64)`, `bad(100)`, `bad(500)`, `bad(1000)` from this test's child
process but dies with `SIGSEGV` on `bad(14)`/`bad(15)`; from a differently-shaped
caller stack (a Python `ctypes` process) `bad(12)` onward crashed instead. That
sensitivity to the *caller's* frame is exactly why the row is informational.
