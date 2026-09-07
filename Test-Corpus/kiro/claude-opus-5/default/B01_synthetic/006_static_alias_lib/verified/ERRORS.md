# ERRORS.md — Phase C error-surface table

## How this table was derived

Mechanical grep of the **entire** C source (`c_src/src/staticalias.c`, 50 lines,
26 of which are the license header) plus the only public header
(`c_src/include/staticalias.h`):

```sh
grep -nE "return|assert|NULL|errno|-1|if|for|while|INT_|LIMIT|MAX|MIN|switch|#if" \
     c_src/src/staticalias.c
30:  if(*outer >= inner) {
32:    return &inner;
35:    return outer;
45:  for (int i = 0; i < iterations; i++) {
49:  return;
```

That is the complete result. Therefore, as a matter of fact about this library:

* there is **no** error-return macro (`RETURN_ERROR`, `CHECK`, `goto fail`, …);
* there is **no** `return -1`, **no** `return NULL`, **no** error enum, **no**
  `errno` use, **no** status/result type;
* there is **no** `assert`, **no** null check, **no** explicit range check, and
  **no** min/max constant;
* there is **no** enum anywhere in the public API, so there is no "invalid enum
  variant" to pass — however the *analogous* case for this API is that **every**
  `int` bit pattern is an accepted input, so the full-range values `INT_MIN`,
  `-1`, `0`, `INT_MAX` are exercised as first-class inputs (rows 4–11 below and
  `CONFIGS.md`);
* `static_alias` can therefore **never** return a null or sentinel pointer: it
  returns either `&inner` or its own `outer` argument, both non-null whenever
  `outer` is non-null.

Consequently the rows below are the library's *implicit* rejection / boundary
conditions — the places where the C either performs no work at all, or where the
C standard says the behaviour is undefined and the compiled artifact
nevertheless has one specific observable behaviour that the Rust must reproduce.
Rows are derived from what the C code actually does, not from invented API
contracts.

Legend for "expected C result": the empirically-established behaviour of the
built `libStaticAlias.so` (gcc, default cmake flags, no `-ftrapv`, no UBSan).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `static_alias` | `outer == NULL` → `*outer` at `staticalias.c:30` dereferences the null page | fatal `SIGSEGV` (signal 11), no return, nothing written | `err_row01_null_pointer_segv` | [x] |
| 2 | `static_alias` | `outer` is a non-null but unmapped/invalid address (e.g. `0x1`) | fatal `SIGSEGV` (signal 11) | `err_row02_wild_pointer_segv` | [x] |
| 3 | `static_alias` | `outer` points to a *read-only* mapping and the else-branch is taken (`*outer += inner` writes) | fatal `SIGSEGV` (signal 11) on the store | `err_row03_readonly_store_segv` | [x] |
| 4 | `static_alias` | signed overflow of `inner += *outer` (then-branch, `*outer >= inner`): `inner == 1`, `*outer == INT_MAX` ⇒ `1 + INT_MAX` | UB per C; artifact wraps two's-complement ⇒ `inner == INT_MIN`, returns `&inner` | `err_row04_then_overflow_intmax` | [x] |
| 5 | `static_alias` | signed overflow of `inner += *outer` where `outer` **aliases** `inner` and `inner` is large: `inner == *outer == INT_MAX` ⇒ `INT_MAX + INT_MAX` | UB per C; artifact wraps ⇒ `inner == -2`, returns `&inner` | `err_row05_then_overflow_aliased` | [x] |
| 6 | `static_alias` | signed *underflow* of `inner += *outer`: `inner == INT_MIN`, `*outer == INT_MIN` (`INT_MIN >= INT_MIN` is true ⇒ then-branch) | UB per C; artifact wraps ⇒ `inner == 0`, returns `&inner` | `err_row06_then_underflow_intmin` | [x] |
| 7 | `static_alias` | signed overflow of `*outer += inner` (else-branch, `*outer < inner`): `inner == INT_MAX`, `*outer == 1` | UB per C; artifact wraps ⇒ `*outer == INT_MIN`, returns `outer` | `err_row07_else_overflow` | [x] |
| 8 | `static_alias` | signed underflow of `*outer += inner`: `inner` negative-large and `*outer` negative (`*outer < inner`) | UB per C; artifact wraps | `err_row08_else_underflow` | [x] |
| 9 | `static_alias` | boundary of the only predicate `*outer >= inner`: `*outer == inner - 1` (last value that takes the else-branch) | else-branch: `*outer = *outer + inner`, returns `outer` (**not** `&inner`) | `err_row09_predicate_one_below` | [x] |
| 10 | `static_alias` | boundary of `*outer >= inner`: `*outer == inner` exactly (first value that takes the then-branch) | then-branch: `inner = 2*inner`, returns `&inner` | `err_row10_predicate_equal` | [x] |
| 11 | `static_alias` | `*outer == INT_MIN` with `inner == 1` (most negative input, guaranteed else-branch) | `*outer = INT_MIN + 1`, returns `outer` | `err_row11_intmin_else` | [x] |
| 12 | `driver` | `iterations == 0` — loop condition `0 < 0` false at first test (`staticalias.c:45`) | returns immediately; **zero** bytes on stdout; `inner` untouched | `err_row12_iterations_zero` | [x] |
| 13 | `driver` | `iterations < 0` (e.g. `-1`) — `0 < -1` false; the C does **not** validate or clamp | returns immediately; zero bytes on stdout; `inner` untouched | `err_row13_iterations_negative` | [x] |
| 14 | `driver` | `iterations == INT_MIN` (most negative, one step past every valid count) | returns immediately; zero bytes on stdout; `inner` untouched | `err_row14_iterations_intmin` | [x] |
| 15 | `driver` | `iterations == INT_MAX` — the loop counter `i++` would itself overflow (UB) after `INT_MAX` iterations | not runnable in bounded time; **excluded by design** (documented, not tested) | *n/a — see note* | [x] |
| 16 | `driver` | `initial_value == INT_MAX`, `iterations >= 1` ⇒ drives the row-4 overflow through the wrapper | wrapped result printed via `printf("%d\n", …)` | `err_row16_driver_overflow_intmax` | [x] |
| 17 | `driver` | `initial_value == INT_MIN`, `iterations >= 2` ⇒ drives the row-11 else-branch then subsequent state | wrapped/negative results printed, byte-identical stdout | `err_row17_driver_intmin` | [x] |
| 18 | `static_alias` | *misaligned* `int*` (address not 4-byte aligned) — UB per C, and x86-64 tolerates it | same value/pointer behaviour as the aligned case on x86-64 | `err_row18_misaligned_pointer` | [x] |

### Note on row 15

`driver(v, INT_MAX)` would execute 2³¹−1 iterations, each printing a line, and
then overflow `i++`. It cannot be run inside the 600 s budget in either
language, so it is deliberately excluded rather than stubbed. Its two
constituent behaviours *are* covered: the loop-counter increment is verified for
large finite counts (`CONFIGS.md` rows 19–20 use counts up to 4096, and both
implementations use the identical `i < iterations` / `i++` form — the Rust uses
`i.wrapping_add(1)`), and integer overflow of the accumulator is covered by rows
4–8 and 16–17.

### Note on rows 1–3, 18 (UB / crashing inputs)

These are tested by `dlopen`ing both libraries in the parent, then `fork()`ing a
child that makes the already-resolved call, and comparing the parent's `waitpid`
status (`WIFSIGNALED` / `WTERMSIG`, or the exit code if the child survived)
between the C and the Rust library. The assertion is on the *specific*
termination signal, not merely "both failed somehow". The child's stderr is
captured on a pipe as well, so a genuine hardware fault can be told apart from a
Rust runtime diagnostic.

Rows 1 and 2 additionally accept one specific, *evidence-backed* alternative
outcome for the Rust side: `SIGABRT` **together with** `core`'s
`unsafe precondition(s) violated` diagnostic on stderr. That is Rust's optional
debug-only UB precondition assertion, which `core` documents as optional and not
to be relied upon, and which is compiled out of the release `cdylib` — the
artifact that corresponds to the C `.so`. The release build matches C's
`SIGSEGV` exactly; the allowance applies only when the child's own stderr proves
the debug check fired (see `assert_same_fault` in `tests/common/mod.rs`). No
other divergence is tolerated.

### Divergence found and fixed (row 18)

Row 18 initially **failed** in a `debug-assertions` build: the translation read
`outer` with a plain `*outer`, which carries an alignment precondition in Rust,
so a misaligned `int*` aborted with
`misaligned pointer dereference: address must be a multiple of 0x4` while the C
artifact read the value and returned normally. Fixed in
`translation/src/lib.rs` by accessing `outer` through `ptr::read_unaligned` /
`ptr::write_unaligned`, which lower to the same alignment-agnostic `mov` that
gcc emits for the C `*outer`. Verified against a standalone probe: a plain
deref of a misaligned pointer aborts (exit 134) whereas `read_unaligned` returns
the value (exit 0), matching C, and both faulted identically for unmapped
addresses. Row 18 now passes in **both** build profiles.

## Completion checklist

- [x] Every distinct rejection/boundary condition the C source contains has a row.
- [x] Every row (except the documented, non-runnable row 15) has a differential
      test that asserts C and Rust agree on the *same* error/sentinel/signal.
- [x] All rows pass under every build configuration swept by `./verify.sh`
      (default / `--no-default-features`, × release / debug profiles).
- [x] Generic C-API boundaries covered even though absent from the C source:
      null pointer (row 1), wild pointer (row 2), read-only target (row 3),
      misaligned pointer (row 18), zero length/count (row 12), negative and
      `INT_MIN` counts (rows 13–14), and values one step past each predicate
      boundary (rows 9–11). No enums exist in this API; the equivalent
      full-range integer inputs are covered instead.
