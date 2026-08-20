# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`.

## How this table was derived (anti-blind-spot audit)

Every rejection/error mechanism was grepped for. Results:

```
grep -nE "return"                       src/driver.c include/driver.h  -> 0 hits
grep -nE "assert"                       src/driver.c                   -> 0 hits
grep -nE "RETURN_ERROR|errno|exit\(|abort" src/driver.c                -> 0 hits
grep -nE "NULL"                         src/driver.c                   -> 2 hits (L32 guard, L61 init)
grep -nE "\bif\b|\bswitch\b|#if"        src/driver.c                   -> L32, L75
```

Consequences, stated explicitly because they shape the whole phase:

- **No function returns a value.** All five public symbols are `void`. There is
  therefore *no* error code, *no* sentinel return, *no* `-1`/`NULL` return and
  *no* error enum anywhere in this library. "Same error/rejection" can only be
  observed as *identical bytes written to stdout* (usually: none) plus
  *identical non-crashing control flow*.
- **Exactly one input-rejection branch exists in the entire library**: the
  `if (line != NULL)` guard at `src/driver.c:32`. That is row 1.
- `if (useGood)` at `src/driver.c:75` is a *mode selector*, not a rejection, so
  its rows live in `CONFIGS.md`. Its behaviour on out-of-range int values is
  still audited here (rows 4–7) because `useGood` is a boolean-typed `int` that
  accepts any of the 2^32 values a caller can push across the FFI boundary.
- `data = NULL;` at `src/driver.c:61` is a dead store overwritten on the next
  line; it rejects nothing.
- The two loops are fixed-trip (`i < 10`) with no caller influence, so they
  contribute no error rows.

## Error-surface table

| #  | function       | trigger (the exact invalid input/condition)                                              | expected C result                                                              | test | ok |
|----|----------------|------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------|------|-----|
| 1  | `printLine`    | `line == NULL` — the sole guard, `src/driver.c:32`                                        | guard fails, `printf` skipped: **zero bytes** written, returns normally         | `err_01_print_line_null` | [x] |
| 2  | `printLine`    | `line` = `""` (valid but degenerate: empty string, `strlen == 0`) — passes the guard       | `puts("")` → exactly one byte `"\n"`                                            | `err_02_print_line_empty` | [x] |
| 3  | `printLine`    | `line` points at a 1 MiB unterminated-until-the-very-end buffer (oversized length)         | whole buffer + `"\n"` written; no truncation, no length cap (C has none)        | `err_03_print_line_oversized` | [x] |
| 4  | `driver`       | `useGood == 0` (the false branch of `src/driver.c:75`)                                     | calls `bad()` → `"0\n"`                                                        | `err_04_driver_zero` | [x] |
| 5  | `driver`       | `useGood == INT_MIN` (out-of-range for a 0/1 flag; negative → still truthy in C)           | truthy → calls `good()` → `"0\n"`                                              | `err_05_driver_int_min` | [x] |
| 6  | `driver`       | `useGood == INT_MAX`                                                                       | truthy → calls `good()` → `"0\n"`                                              | `err_06_driver_int_max` | [x] |
| 7  | `driver`       | `useGood` = any non-{0,1} value with no valid "variant" (`-1, 2, 3, 42, 0x100, 0xFFFF…`)   | every nonzero → `good()` → `"0\n"`; only exact 0 → `bad()`                     | `err_07_driver_enum_range` | [x] |
| 8  | `printIntLine` | `INT_MIN` (boundary; `%d` of the value with no positive counterpart)                       | `"-2147483648\n"` — no overflow/abs bug                                        | `err_08_print_int_line_int_min` | [x] |
| 9  | `printIntLine` | `INT_MAX`                                                                                  | `"2147483647\n"`                                                               | `err_09_print_int_line_int_max` | [x] |
| 10 | `printLine`    | `line` contains `printf` format specifiers (`"%d %s %n %p %%"`) — format-string injection  | C uses `printf("%s\n", line)`, so bytes are **literal**; no arg consumed, no crash | `err_10_print_line_format_injection` | [x] |
| 11 | `printLine`    | `line` contains a NUL at index 0 of a longer buffer (early terminator)                     | stops at the NUL: only `"\n"` written; trailing bytes ignored                   | `err_11_print_line_embedded_nul` | [x] |
| 12 | `bad`          | `bad()` writes 40 bytes through a 10-byte `alloca` (CWE-131 undersized allocation)         | the intentional bug: still prints `"0\n"` and returns without trapping         | `err_12_bad_undersized_alloc` | [x] |

### Row 12 — the deliberate CWE-131 bug, and what "identical" means for it

`bad()` does `alloca(10)` (10 **bytes**) and then stores ten 4-byte `int`s
through it, i.e. 40 bytes into a 10-byte region — a stack buffer overflow. This
is the defect the test case exists to demonstrate; per instructions the C is
ground truth and the behaviour is *not* "fixed".

The *observable contract* of `bad()` is nevertheless well defined and is what
the differential test asserts: it prints `"0\n"` and returns normally. The 30
overflowing bytes land in unused red-zone/alignment padding of the frame that
gcc reserves, so the C build does not crash or alter its output.

The Rust translation backs `data` with a full `[0i32; 10]`, so it writes the
same values, prints the same `"0\n"`, and returns the same way — matching every
*observable* aspect of the C — while not committing UB that could
non-deterministically corrupt the Rust frame. Reproducing the out-of-bounds
*store* itself would be unobservable on stdout yet could make the harness crash
at random, which would be a worse match, not a better one. This is the one place
where byte-identical output is achieved without a byte-identical memory effect,
and it is called out here deliberately rather than silently.
