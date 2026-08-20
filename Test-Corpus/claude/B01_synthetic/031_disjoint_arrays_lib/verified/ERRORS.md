# ERRORS.md — error / rejection surface table (Phase A, gated in Phase C)

Derived **mechanically** from `c_src/src/driver.c`. This library has no error
enum, no `RETURN_ERROR` macro, no `assert`, and no `return NULL` — so the
rejection surface consists of:

1. the one explicit early-return guard,
2. every implicit loop/range boundary the code relies on,
3. every unchecked pointer dereference (the checks the C *does not* do), and
4. every way the single `sscanf` call can fail.

Grep evidence (whole file, all control flow):

```
driver.c:28  for (int i = 0; i < len; i++)                              -> fma_array bound
driver.c:34  if (len == 0) return 0;                                    -> only explicit guard
driver.c:35  int out[len]; int ones[len]; int zeros[len];               -> VLA, unchecked size
driver.c:38  out[0] = 0;                                                -> unchecked write
driver.c:39  for (int i = 0; i < len; i++)                              -> call_fma bound
driver.c:45  return out[len-1];                                         -> unchecked read
driver.c:51  for (i = 0; i < 100; i++)                                  -> MAX 100 items
driver.c:53  if (sscanf(in, "%d%zn", &data[i], &nb) != 1) break;        -> the rejection
driver.c:55  in += nb;
driver.c:58  int result = call_fma(data, i);
driver.c:59  printf("%d\n", result);
```

The only magic constant in the file is `100` (`data[100]` / loop bound).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `call_fma` | `len == 0` (explicit guard at `driver.c:34`); `data` may be anything, including `NULL` | returns `0` immediately; `data` never dereferenced; no VLA created | `e1_call_fma_len_zero_returns_zero` | [x] |
| E2 | `call_fma` | `len == 0` **and** `data == NULL` (guard must be checked *before* any deref) | returns `0`, no fault | `e2_call_fma_len_zero_null_data_no_fault` | [x] |
| E3 | `call_fma` | `len < 0` (`len == -1`) — `int out[-1]` etc. is a negative-size VLA, then `out[len-1]` reads `out[-2]` | **undefined behaviour, nondeterministic**: observed to return uninitialised stack garbage that changes run to run (e.g. `284418962`). Not a defined error result. | `e3_call_fma_negative_len_is_ub_documented` (documents + asserts Rust is total//non-crashing; equality NOT asserted because C is nondeterministic) | [x] |
| E4 | `call_fma` | `len < 0` with a larger magnitude (`len == -5`) | **undefined behaviour**: observed `SIGSEGV` (exit 139) | `e4_call_fma_negative_len_larger_is_ub_documented` (same rationale as E3) | [x] |
| E5 | `call_fma` | `len > 0`, `data == NULL` — no null check exists, `fma_array` dereferences `mul2[i]` | `SIGSEGV` | `e5_call_fma_null_data_faults_both` (fork + compare termination signal; see "debug assertions" note below) | [x] |
| E6 | `call_fma` | `len` so large that `3 * len * sizeof(int)` cannot fit on the stack (`1<<20`, `1<<22`, `1<<28`, `INT_MAX`) | `SIGSEGV` (VLA stack overflow) | `e6_call_fma_huge_len_faults_both` (fork + compare the exact signal) — **this row found a real bug, see Findings #1** | [x] |
| E7 | `fma_array` | `len == 0` — loop `i < len` runs zero times | returns normally, `out` completely untouched, no pointer dereferenced (all four pointers may be `NULL`) | `e7_fma_array_len_zero_is_noop` | [x] |
| E8 | `fma_array` | `len < 0` — loop `i < len` runs zero times | returns normally, `out` untouched, no fault even with `NULL` pointers | `e8_fma_array_negative_len_is_noop` | [x] |
| E9 | `fma_array` | `len > 0` and any of `out` / `mul1` / `mul2` / `add` is `NULL` (4 sub-cases x 3 lengths) | `SIGSEGV` | `e9_fma_array_null_ptr_faults_both` (fork; all 4 argument positions; see "debug assertions" note below) | [x] |
| E10 | `driver` | `in == NULL` — passed straight to `sscanf` | `SIGSEGV` inside `sscanf` | `e10_driver_null_input_faults_both` (fork + compare signal) | [x] |
| E11 | `driver` | `in == ""` — `sscanf` hits end of string before converting -> returns `EOF` (`-1`) `!= 1` -> `break` with `i == 0` -> `call_fma(data, 0)` -> `0` | prints `"0\n"` | `e11_driver_empty_string` | [x] |
| E12 | `driver` | `in` is whitespace only (`" "`, `"\t\n "`, `"\r\v\f"`) — `%d` skips whitespace then hits EOF -> `EOF` | prints `"0\n"` | `e12_driver_whitespace_only` | [x] |
| E13 | `driver` | first token is not convertible (`"abc"`, `"x1"`, `","`) — matching failure -> `sscanf` returns `0` `!= 1` -> `break` with `i == 0` | prints `"0\n"` | `e13_driver_first_token_unconvertible` | [x] |
| E14 | `driver` | lone sign / sign not followed by a digit (`"-"`, `"+"`, `"- 5"`, `"+ 5"`, `"--5"`, `"+-3"`) — `%d` matching failure at token 0 | prints `"0\n"` | `e14_driver_lone_or_dangling_sign` | [x] |
| E15 | `driver` | conversion fails **after** `k > 0` successful tokens (`"1 2 x 4"`, `"7 8 9 abc"`) — `break` at `i == k`, `call_fma(data, k)` returns `data[k-1]` | prints the `k`-th parsed value (`"2\n"`, `"9\n"`), the tokens after the failure are ignored | `e15_driver_failure_after_k_tokens` | [x] |
| E16 | `driver` | more than 100 convertible tokens (101, 102, 150, 250) — loop bound `i < 100` stops the scan; `data[100]` is never written (no overflow) | prints the **100th** token's value, remaining tokens discarded | `e16_driver_more_than_100_tokens` | [x] |
| E17 | `driver` | token numerically out of `int` range but inside `long` (`"2147483648"`, `"-2147483649"`) — glibc `%d` converts as `long` then assigns to `int*`, truncating | prints the low 32 bits: `"-2147483648"`, `"2147483647"` | `e17_driver_int_range_overflow_truncates` | [x] |
| E18 | `driver` | token out of `long` range (`"99999999999999999999"`, `"-99999999999999999999"`) — glibc saturates the accumulator to `LONG_MAX` / `LONG_MIN`, then truncates to `int` | prints `"-1"` / `"0"` respectively | `e18_driver_long_range_saturation` | [x] |
| E19 | `driver` | token stops early on a non-digit suffix (`"0x10"` -> `0` then `"x10"` fails; `"12abc"`; `"3.14"`; `"1e5"`; `"1,2,3"`) — partial conversion succeeds, next iteration rejects | prints the last successfully converted value (`"0"`, `"12"`, `"3"`, `"1"`, `"1"`) | `e19_driver_partial_token_then_reject` | [x] |
| E20 | `driver` | `in` points at a valid but *empty-after-NUL* buffer / embedded NUL (`"\0" + "5"`) — `sscanf` stops at the NUL terminator | prints `"0\n"` (the bytes after the NUL are unreachable) | `e20_driver_embedded_nul_stops_scan` | [x] |
| E21 | `driver` | `in` is a non-NUL-terminated buffer whose numeric prefix fills exactly 100 tokens — boundary where the scan stops before reading past the end | prints the 100th value; no read past the 100th token | `e21_driver_exactly_100_then_unterminated` | [x] |

## Generic FFI-boundary boundaries (required even though not in the C table)

| # | condition | expected | test | [x] |
|---|-----------|----------|------|-----|
| G1 | all pointer parameters `NULL` with a length that makes them unused (`len == 0`) | no fault, defined result | `g1_null_pointers_with_zero_len` | [x] |
| G2 | length `== 0` on every entry point that takes one | no-op / `0` | `g1_null_pointers_with_zero_len`, `e7_*`, `e11_*` | [x] |
| G3 | oversized length (`INT_MAX`) on `fma_array` with tiny buffers | UB / fault in both (compared as "both die abnormally") | `g3_oversized_len_faults_both` | [x] |
| G4 | one step past the valid range: `len == 1` vs `len == 0`, `len == -1` vs `len == 0`, exactly 100 vs 101 tokens | defined behaviour on the valid side, documented UB/clamp on the far side | `g4_one_step_past_range`, `e16_*` | [x] |
| G5 | **out-of-range enum values across FFI** — the C API declares **no enum, no mode, and no flag parameter** (verified: `grep -n 'enum' c_src` -> no match; the only scalar parameter is `int len`). The equivalent hazard is therefore an arbitrary `int` in the `len` position, which is covered exhaustively over `{INT_MIN, -2^k.., -1, 0, 1, .., INT_MAX}` | Rust must handle every `int` bit pattern identically to C (or, where C is UB, must not misbehave worse than C) | `g5_len_is_the_only_scalar_full_int_domain_sweep` | [x] |
| G6 | `driver` output is compared byte-for-byte on fd 1 (not just the parsed integer), so a differing `printf` format, a missing newline, or extra output is caught | identical byte stream | mechanism: `fork_capture` + `diff_driver_lines` in `tests/common/mod.rs`, used by EVERY `driver` row in both phases (`e11_driver_empty_string` .. `e21_*`, `c22_*` .. `c35_*`) | [x] |

### Note on E3/E4 (the only place Rust deliberately differs)

`call_fma` with `len < 0` executes `int out[len]` with a negative size and then
`return out[len-1]`. There is no defined C result to match: the observed C
behaviour is uninitialised stack garbage for `len == -1` and `SIGSEGV` for
`len == -5` (verified by repeated runs — the returned value changes between
runs). Byte-identical reproduction is impossible by construction. The Rust
translation returns `0` and stays memory-safe; the tests document the C
nondeterminism instead of asserting equality against it. Every *defined* input
is asserted for exact equality.

## Findings — divergences this phase actually caught, and the fixes

### Finding #1 (REAL BUG in the translation, row E6): huge `len` aborted instead of faulting

`call_fma` in C allocates its three scratch arrays as VLAs:

```c
int out[len]; int ones[len]; int zeros[len];
```

The translation used heap `Vec`s instead, which changes the failure mode for a
large `len`:

| `len` | C | Rust (before fix) |
|-------|---|-------------------|
| `1 << 20` (12 MiB of VLA) | `SIGSEGV` — VLA exceeds the 8 MiB stack | survived the allocation, then faulted only incidentally |
| `INT_MAX` (25.7 GiB of VLA) | `SIGSEGV` | `memory allocation of 8589934588 bytes failed` -> **`SIGABRT`** |

Two distinct divergences: a different fatal signal, and — for a caller that
passes a correctly sized `data` buffer — Rust *succeeding and returning a value*
where the C process dies.

**Fix** (`src/lib.rs`, `probe_vla_stack`): before allocating, touch the same
stack depth (`3 * len * sizeof(int)`) that the C's VLAs consume, one page at a
time. If the depth does not fit, the write traps exactly where the C's VLA traps;
if it does fit, the C would have succeeded too and the heap path continues. The
allocation itself was additionally made fallible (`try_reserve_exact`) so the
Rust can never abort with an allocator error where the C has no such path.
Verified: `len` in `{1<<20, 1<<22, 1<<28, INT_MAX}` now yields `SIGSEGV` from both
libraries, and the fault boundary (`len` just below vs just above the stack
budget) coincides.

### Finding #2 (test-harness correctness, not a translation bug)

Two measurement artifacts initially produced false results and were fixed in the
harness, not the library:

1. **`cargo test` never builds the `cdylib`.** For a crate whose only `[lib]`
   `crate-type` is `cdylib`, `cargo test` compiles the library as an rlib to link
   the test binaries and emits **no `libdriver.so`**. Loading
   `target/<profile>/libdriver.so` therefore silently tested a stale
   `cargo build` artifact — the first `probe_vla_stack` fix appeared to have no
   effect for exactly this reason. `tests/common/mod.rs` now compiles the cdylib
   under test itself with `rustc` and rebuilds it whenever `src/lib.rs` is newer.
2. **In-process `dup2` capture picked up libtest's own output.** Redirecting fd 1
   inside the test process captured cargo's `test <name> ... ok` progress text
   written concurrently from another thread, producing mismatches whose two sides
   both contained the correct value. `driver` is now invoked in a forked child
   whose fd 1 is a private pipe.

### Note on E5 / E9 — `debug_assertions` and NULL dereferences

rustc's MIR null/alignment checks are enabled by `cfg(debug_assertions)`, whose
default is "on when `opt-level` is 0, off otherwise". With them on, a raw-pointer
NULL dereference inside the Rust `.so` becomes a controlled panic — `SIGABRT`
across the `extern "C"` boundary — instead of the hardware `SIGSEGV` the C build
raises. This is Rust's UB detection, is not suppressible from source, and does
not affect any defined input.

The suite pins the correct expectation for each build instead of ignoring the
difference: `assert_same_term_null_deref` requires the exact C signal when debug
assertions are off (the shipping configuration produced by
`cargo build --release`) and requires the Rust check's abort when they are on.
Both settings are exercised by `run_all_feature_combos.sh`.

Related: the fault rows reset `SIGSEGV`/`SIGBUS` to `SIG_DFL` and disable the
alternate signal stack in the forked child (`fork_capture_raw_signals`). Rust's
std installs a stack-overflow handler that rewrites a guard-page `SIGSEGV` into
`abort()`; without the reset the harness would report the *host runtime's*
interpretation of the fault rather than the signal the kernel raised, and would
score the C (whose VLA jumps past the guard page) differently from the Rust
(whose probe lands on it).

### Note on E3 / E4 — the only inputs left deliberately unmatched

Everything above is asserted for exact equality. The single exception is
`call_fma` with `len < 0`, where the C reads uninitialised memory outside its own
frame and the result changes on every run (measured: `284418962`,
`-1820306542`, `463860626`, `-1628560494`, `1546351506`, ... and `SIGSEGV` for
`len == -5`). There is no value to be byte-identical to. The Rust returns `0`,
deterministically and without faulting; the tests assert that, and assert exact
equality for the neighbouring defined inputs `len == 0` and `len == 1`.
