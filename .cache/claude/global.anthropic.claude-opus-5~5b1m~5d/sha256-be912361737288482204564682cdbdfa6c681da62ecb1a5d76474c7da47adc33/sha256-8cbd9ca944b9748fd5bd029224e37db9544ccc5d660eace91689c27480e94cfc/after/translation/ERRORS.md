# ERRORS.md — Phase C error-surface table

Mechanically derived from the complete C source (`c_src/src/driver.c`, 58 lines,
and `c_src/include/driver.h`, 28 lines).

## Mechanical grep of every rejection construct

```bash
grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit|abort|if *\(' \
     c_src/src/driver.c c_src/include/driver.h
```

Result: the ONLY `if` in the whole library is `if (useGood)` in `driver`. There
are:

* **0** `return` statements with a value (every function returns `void`)
* **0** `assert` / `NDEBUG` uses
* **0** error enums, error codes, or sentinel return values
* **0** `NULL` checks
* **0** explicit range checks, and **0** min/max constants
* **0** `errno` / `exit` / `abort` uses

So this library has **no error-reporting channel at all**: no function can
report failure to its caller. The "error surface" is therefore entirely made of
*undefined behaviour* triggers and of *out-of-range argument values that the C
silently accepts*. Those are enumerated below, one row per distinct
trigger/condition that the C code actually reaches.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| E1 | `printIntPtrLine` | `intNumber == NULL` — no null check exists, so `*intNumber` dereferences the null page | UB: process terminated by `SIGSEGV` (no return, no error code) | `err_e1_null_ptr_segv` |
| E2 | `printIntPtrLine` | `intNumber` = a non-null but unmapped address (e.g. `0xdeadbeef000`, `usize::MAX & !7`) | UB: process terminated by `SIGSEGV` | `err_e2_wild_ptr_segv` |
| E3 | `printIntPtrLine` | `intNumber` misaligned (valid mapped byte address that is not 4-byte aligned) | no fault on x86-64; prints the little-endian `int` read at that byte offset, `"%d\n"` | `err_e3_misaligned_ptr` |
| E4 | `printIntPtrLine` | `intNumber` points at the last 1..3 mapped bytes of a page whose successor page is unmapped (partial out-of-bounds 4-byte read) | UB: process terminated by `SIGSEGV` | `err_e4_page_edge_segv` |
| E5 | `printIntPtrLine` | `*intNumber == INT_MIN` / `INT_MAX` / `-1` — boundary values of the formatted type, no range check | prints the decimal value, incl. `"-2147483648\n"` | `err_e5_int_boundaries` |
| E6 | `bad` | called at all (CWE-457): local `int *data` is read while uninitialised and then dereferenced by `printIntPtrLine` | UB, no error code. Deterministic per-build only: whatever garbage the stack slot holds. Must be compared as *class of outcome* (fault vs. print), never as a fixed value | `err_e6_bad_is_ub_no_error_code` |
| E7 | `driver` | `useGood` = an out-of-range "enum-like" `int` with no valid variant (`2`, `-1`, `INT_MIN`, `INT_MAX`, `0x100`, `0x80000000`) | `if (useGood)` is a plain truthiness test, so every non-zero value takes the `good()` branch and prints `"5\n"`; only exactly `0` takes `bad()` | `err_e7_out_of_range_enum_values` |
| E8 | `driver` | `useGood == 0` (the single value that routes into the CWE-457 `bad()` path) | UB via E6; no error code returned to the caller | `err_e8_driver_zero_routes_to_bad` |
| E9 | any | calling `good` / `driver(nonzero)` repeatedly / re-entrantly | no state, no allocation, no failure mode: always prints `"5\n"`, never errors | `err_e9_no_failure_mode_on_repeat` |



### Observed results (x86-64 glibc, C at cmake default `-O0`, Rust `--release`)

* E1, E2, E4 and the generic sweep: **both** implementations are killed by
  `SIGSEGV` (signal 11) with an empty stdout — statuses compared and equal.
* E3: neither faults; both print the same little-endian `int` read from the
  unaligned address (cross-checked against `read_unaligned`).
* E5, E7, E9: no fault, byte-identical stdout, values as tabulated.
* E6/E8 (`bad`, `driver(0)`): on this platform **neither** faults — the
  indeterminate stack slot happens to hold a dereferenceable address in both
  builds, so each prints one `%d\n` line of *garbage* (e.g. C `-76966001`,
  Rust `0`). The **values necessarily differ**: C reads whatever its own stack
  frame left behind, and the C standard leaves that unspecified, so it is not
  part of the contract and MUST NOT be "matched" by the Rust. What the tests do
  assert is (a) the same outcome class (both survive / both die with the same
  signal), (b) the same exit status, and (c) that the surviving output is
  exactly one `printf("%d\n", ...)` line — proving the Rust really executed the
  same uninitialised-read-then-dereference path rather than short-circuiting.

### Notes on how E1/E2/E4/E6/E8 are asserted

A `SIGSEGV` cannot be observed in-process, so each of those rows is exercised in
a **forked child process** (`fork()` + `waitpid()`), asserting that the C child
and the Rust child agree on the exact `WTERMSIG` / `WEXITSTATUS` and on the bytes
written to stdout before dying. For E6/E8 (`bad`) the value of the garbage
pointer is *not* part of the contract, so those rows assert that C and Rust agree
on the *observable class* (both fault with the same signal, or both survive) —
they deliberately do not pin a value the C standard leaves indeterminate.
