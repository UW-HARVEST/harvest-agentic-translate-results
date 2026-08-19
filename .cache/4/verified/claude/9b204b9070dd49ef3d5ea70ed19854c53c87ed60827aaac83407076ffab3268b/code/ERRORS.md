# ERRORS.md — Error / rejection surface table

Derived **mechanically** from `c_src/src/driver.c` (the only translation unit).

## Mechanical grep of every rejection / guard / constant

```
$ grep -n -E "return|NULL|assert|errno|if *\(|switch|#if" c_src/src/driver.c
32:    if(line != NULL)          <- guard 1 (null check)
44:    if (data < 100)           <- guard 2 (range check)
```

* Error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`): **none** —
  both public functions return `void`, so there is **no error code channel at
  all**. Every "rejection" is expressed purely as *suppressed side effects* or
  as *undefined behaviour*.
* `assert` / `abort` / `exit`: **none**.
* Error enums: **none** (no `enum` in the library).
* Explicit range checks: `data < 100` (line 44).
* Null checks: `line != NULL` (line 32).
* Min/max constants: `100` (both array bounds), `100-1` = `99`
  (`memset` length and NUL index).

## The table

One row per distinct rejection / guard branch the C code actually takes.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `printLine` | `line == NULL` (line 32 guard fails) | `puts` is **not** called; function returns; **zero bytes** written to `stdout` | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` points at an immediate NUL (`""`) — guard passes, degenerate content | `puts("")` → exactly the 1 byte `"\n"` | `err_e2_print_line_empty` | [x] |
| E3 | `driver` | `data == 100` (first value that fails `data < 100`) | `strncpy`/`dest[data]` **skipped**; `dest` still all-zero from `= ""` → `printLine("")` → `"\n"` | `err_e3_driver_at_bound_100` | [x] |
| E4 | `driver` | `data == 101` (one step past the bound) | same as E3 → `"\n"` | `err_e4_driver_past_bound_101` | [x] |
| E5 | `driver` | `data == INT_MAX` (oversized length, maximal) | `data < 100` false → `"\n"` | `err_e5_driver_int_max` | [x] |
| E6 | `driver` | `data == 99` (largest value that *passes* `data < 100`; `dest[99]` is the **last in-bounds** byte — the off-by-one boundary) | 99 `'A'` bytes + `"\n"` | `err_e6_driver_at_99_boundary` | [x] |
| E7 | `driver` | `data == 0` (zero length; `strncpy(dest,src,0)` copies nothing, `dest[0]='\0'`) | `"\n"` (empty line) | `err_e7_driver_zero_len` | [x] |
| E8 | `driver` | `data < 0` (e.g. `-1`): passes `data < 100`, then `movslq` sign-extends `data` into `size_t` for `strncpy` → length `SIZE_MAX-…` → **out-of-bounds write past `dest`** (CWE-787). `dest[data]` (`cltq`) would also underwrite. | **process terminated by a signal** (`SIGSEGV`) — no error code, no output. This is the injected vulnerability and must reproduce identically. | `err_e8_driver_negative_crashes` (forked child, compares `WTERMSIG`) | [x] |
| E8b | `driver` | *width* of the `data` → `size_t` conversion for `data < 0`: C emits `movslq` (**sign**-extend, length `2^64-1`), not a zero-extension (`2^32-1`) | sign-extension must be preserved; see the note on observational equivalence below | `err_e8b_negative_data_is_sign_extended_not_zero_extended` | [x] |
| E9 | `driver` | `data == INT_MIN` (most extreme negative; `-data` overflows) | same as E8 → terminated by `SIGSEGV` | `err_e9_driver_int_min_crashes` | [x] |
| E10 | `driver` | `data == -100` / `-99` / `-1` (sweep of negatives incl. exactly the buffer size) | same as E8 → terminated by `SIGSEGV` | `err_e10_driver_negative_sweep_crashes` | [x] |

## Generic FFI boundary cases (required even though not in the C table)

| # | case | why it is a real input | expected | test | [x] |
|---|------|------------------------|----------|------|-----|
| G1 | `printLine(NULL)` | null pointer | no output (= E1) | `err_e1_print_line_null` | [x] |
| G2 | `driver` with zero length (`0`) | zero length | `"\n"` (= E7) | `err_e7_driver_zero_len` | [x] |
| G3 | `driver` with oversized length (`INT_MAX`, `1<<30`, `100000`) | oversized length | `"\n"` | `err_g3_driver_oversized_lengths` | [x] |
| G4 | `driver(99)` / `driver(100)` — one step either side of the documented valid range | boundary ±1 | 99 `'A'`s+`\n` / `\n` | `err_g4_driver_one_step_past_range` | [x] |
| G5 | out-of-range **enum** value across FFI | **N/A — the library declares no `enum` type**; the only parameter types are `int` (full `i32` range swept, see `CONFIGS.md` rows C10–C16) and `const char *` (null + non-null covered). Documented here so the check is not silently skipped. | — | (see C10–C16) | [x] |
| G6 | `printLine` with a `'%'`-bearing string | `printf("%s\n", line)` in Rust vs folded `puts(line)` in C: a format-string confusion would show up here | `%` copied literally | `err_g6_print_line_percent_not_format` | [x] |
| G7 | `printLine` with embedded NUL / embedded newline / non-ASCII high bytes | terminator & byte-transparency | truncate at NUL; bytes passed through | `err_g7_print_line_embedded_bytes` | [x] |

All rows above are asserted **differentially**: the C `.so` and the Rust `.so`
are both loaded with `libloading`, invoked with the identical input, and their
captured `stdout` bytes (or their `waitpid` termination signal, for E8–E10) are
compared for exact equality — not merely "both failed somehow".

## Test-suite strength: mutation testing

The differential suite is only meaningful if it can actually *detect* a wrong
translation. Each mutant below was injected into `src/lib.rs`, rebuilt, and the
full suite re-run.

| mutant injected into `src/lib.rs` | detected? | detecting rows |
|---|---|---|
| `memset` fill byte `'A'` → `'B'` | **caught** (6 tests) | C11, C12, C15–C18 |
| guard `data < 100` → `data <= 100` | **caught** (5 tests) | C13/C15/C16/C17, E3 |
| guard `data < 100` → `data < 99` | **caught** (5 tests) | C12/C15, E6 |
| `printLine`: drop the `line != NULL` check | **caught** (2 tests) | C17, C18, E1 |
| `printLine`: invert the `line != NULL` check | **caught** (18 tests) | all `printLine` rows |
| `printf("%s\n")` → `printf("%s")` (no newline) | **caught** (18 tests) | all rows |
| `memset` length `100-1` → `100-2` | **caught** (5 tests) | C12/C15, E6 |
| `dest` not zero-initialised (`[0;100]` → `[1;100]`) | **caught** (6 tests) | C13/C14, E3–E5 |
| `data as usize` → `data as u32 as usize` (zero-extend) | **caught** (1 test) | **E8b** |
| `strncpy_c`: remove the `if ch == 0 { break }` NUL-stop | **EQUIVALENT** — not detectable | see below |

### Two observational-equivalence limits (documented, not hidden)

1. **`strncpy` NUL-stop / zero-pad is unreachable.** `driver` is the only caller
   of the private `strncpy_c` helper, always with `n = data <= 99`, while
   `source` holds 99 `'A'`s with its NUL at index **99**. The copied range is
   therefore always `0..=98` — all `'A'` — so the NUL-stop branch and the
   zero-padding loop can never execute. `strncpy_c` is **not** an exported
   symbol (`nm -D` confirms), so no external caller can reach it either. The
   mutant is genuinely equivalent, not a coverage hole.

2. **Sign- vs zero-extension of a negative `data` is not black-box observable.**
   `(size_t)(int)-1` = `2^64-1` and `(size_t)(unsigned)-1` = `2^32-1` both make
   `strncpy` run off the top of the stack, so both die from the *same* `SIGSEGV`
   and E8/E9/E10 cannot separate them. Separating them would require more than
   4 GiB of writable memory *above* `dest`, which is unobtainable because
   `dest` is a stack local near the top of its stack region. The distinction is
   therefore checked at the machine-code level by **E8b**, which compares the
   number of sign-extending instructions in `driver` between the two `.so`s
   (C: `movslq` + `cltq` = 2; Rust: `movslq` ×2; the zero-extending mutant: 1).
