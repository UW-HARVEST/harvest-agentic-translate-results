# ERRORS.md — Error-surface table (Phase C)

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h` by
grepping for every rejection construct:

```
grep -n "return"                     -> (none)
grep -nE "assert|abort|exit\("       -> (none)
grep -nE "#define|enum|const|MAX|MIN"-> only the DRIVER_H_ include guard
grep -nE "if|switch|while|for"       -> driver.c:30  if (line != NULL)
                                        driver.c:51  if (useGood)
```

The library's rejection surface is genuinely tiny, and this is a *derived*
result, not an assumption:

* **All four functions return `void`.** There are zero `return` statements, so
  there are no error codes, no sentinel returns and no `errno` usage anywhere.
* There are **no** `assert`s, no `abort`/`exit`, no error enums, no error-return
  macros, and no min/max constants.
* There are exactly **two** conditionals in the whole translation unit, and only
  one of them is a rejection: the `NULL` guard in `printLine`.

## Error-surface rows

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` (`driver.c:30`, guard is false) | Returns normally, **prints nothing**, no crash. This is the *only* input-rejection branch in the library. |

That single row is the complete error surface. To avoid a false sense of
coverage, the generic FFI boundary conditions the instructions call for are
enumerated below and are tested with equal rigour, even though the C code does
not treat them as errors.

## Generic FFI-boundary rows (not C-detected errors, but tested)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 2 | `printLine` | `line` = valid pointer to `""` (empty string, length 0) | prints a single `\n` (**not** "nothing" — distinguishes empty-string from NULL) |
| 3 | `printLine` | `line` = pointer to string with no interior NUL, length 1 | prints that byte + `\n` |
| 4 | `printLine` | `line` contains `printf` conversion specifiers (`%s`, `%d`, `%n`, `%%`) | printed **literally**; never interpreted as a format string (C uses `puts`, Rust passes it as a `%s` argument) |
| 5 | `printLine` | `line` contains embedded newline / CR / tab / `\0`-adjacent bytes | bytes copied verbatim up to the first NUL, then `\n` |
| 6 | `printLine` | `line` contains non-UTF-8 / high bytes `0x80..=0xFF` | bytes copied verbatim (byte-oriented, not UTF-8 validated) |
| 7 | `printLine` | oversized length: 4 KiB and 256 KiB strings (past stdio buffer size) | full string + `\n`, no truncation |
| 8 | `driver` | `useGood == 0` | takes the `bad()` branch (see §UB below) |
| 9 | `driver` | `useGood != 0` — out-of-range "enum-like" ints passed across FFI: `1`, `-1`, `2`, `INT_MAX`, `INT_MIN`, `0x7FFFFFFF`, random | `if (useGood)` is true for **every** non-zero `int`; there is no valid-variant check, so all of these take the `good()` branch. `INT_MIN` and `-1` must **not** be mistaken for false. |
| 10 | `bad`, `good`, `driver` | no arguments to invalidate / repeated invocation | idempotent; each call re-emits its output; no state, no initialization requirement, no ordering constraint |

Note on row 9: the API has **no `enum` type at all** (grep confirms), so the
"out-of-range enum value" class reduces to "arbitrary `int` in the `useGood`
parameter". Rust's `useGood != 0` is exactly equivalent to C's `if (useGood)`
for all 2^32 values, including `INT_MIN`, which is the value most likely to be
mishandled by a translation that used something like `useGood > 0`.

## §UB — `bad()` / `driver(0)`: undefined behaviour, deliberately not "fixed"

`bad()` reads an **uninitialized** local pointer (`driver.c:38-39`):

```c
void bad(void) { char *data; printLine(data); }
```

This is the intentional defect of this MIT-LL test case (CWE-457, use of an
uninitialized variable). It is *not* corrected in the Rust translation. Its
observable behaviour is **not a fixed value** and depends on the optimization
level, which is why it gets special treatment in the tests:

| C build | `bad()` codegen | observable behaviour |
|---------|-----------------|----------------------|
| `-O0` (**the CMakeLists default** — `CMAKE_BUILD_TYPE` is empty) | `mov -0x8(%rbp),%rax; mov %rax,%rdi; call printLine` | reads whatever 8 bytes of **stack residue** sit at `[rbp-8]`, then `puts` them if non-NULL |
| `-O1` | `mov $0x0,%edi; call printLine` | `printLine(NULL)` → prints nothing |
| `-O2`, `-O3`, `-Os` | `xor %edi,%edi; jmp printLine` | `printLine(NULL)` → prints nothing |

At `-O0` the residue is real and observable. Measured against the actual C
`.so`, the same `bad()` produced **four different results** purely from changing
what ran before it:

| preceding calls | C `-O0` `bad()` output |
|-----------------|------------------------|
| `bad()` called first | `0a` (residual pointer aimed at an empty string) |
| after a deep `memset` call chain | *(nothing — residue happened to be NULL)* |
| after `printLine("aaaa…")` | `aaaaaaaaaaaaaaaa\n` — **re-prints the previous argument**, because the stale pointer is still in that slot |
| `driver(0)` called first | `55 48 89 e5 48 83 ec 10 89 7d fc 83 7d fc 0a` — the residual pointer aimed **into the code segment** and `puts` printed `driver`'s own machine-code bytes |

The last case proves byte-identical reproduction of `-O0` `bad()` is
**impossible for any translation**: the output is a function of the gcc-emitted
object code and its load address. (A frame-exact `naked_asm!` replica of the C
prologue was prototyped and confirmed to match the *first* case but still
diverge on the others, so it buys nothing and would inject real UB into the
Rust library. It was rejected.)

**Resolution.** The Rust `bad()` passes a null pointer, which is exactly what
gcc emits at `-O1/-O2/-O3/-Os`. So:

* rows 1–7, 9, 10 and `driver(non-zero)` are differentially tested against the
  **default `-O0`** C `.so` — they are fully deterministic there;
* rows 8 and `bad()` are differentially tested against an **`-O2`** C `.so`,
  where the C behaviour is well-defined, so this is a real assertion rather
  than a skipped test;
* the `-O0` divergence is asserted to be *exactly* the documented UB and
  nothing more, by `tests/ub_bad.rs`, which pins the `-O0`/`-O2` codegen
  difference so the exclusion can never silently widen.

## Row status

| # | tested by | status |
|---|-----------|--------|
| 1 | `errors.rs::row1_printline_null_prints_nothing` | ✅ |
| 2 | `errors.rs::row2_printline_empty_string_prints_newline` | ✅ |
| 3 | `errors.rs::row3_printline_single_byte` | ✅ |
| 4 | `errors.rs::row4_printline_format_specifiers_are_literal` | ✅ |
| 5 | `errors.rs::row5_printline_control_bytes` | ✅ |
| 6 | `errors.rs::row6_printline_high_bytes` | ✅ |
| 7 | `errors.rs::row7_printline_oversized` | ✅ |
| 8 | `ub_bad.rs::row8_driver_zero_matches_optimized_c` | ✅ |
| 9 | `errors.rs::row9_driver_nonzero_int_boundaries` | ✅ |
| 10 | `errors.rs::row10_repeated_invocation_is_idempotent` | ✅ |

Supporting tests:

| test | purpose |
|------|---------|
| `ub_bad.rs::row18_bad_matches_optimized_c` | `bad()` vs the `-O2` C build |
| `ub_bad.rs::row17_driver_zero_interleaved_with_good` | zero/non-zero branches composed in one stream |
| `ub_bad.rs::optimized_c_folds_uninitialized_read_to_null` | pins the `-O2` codegen the Rust `bad()` mirrors |
| `ub_bad.rs::default_c_build_is_unoptimized_and_reads_stack_residue` | pins the `-O0` codegen that justifies the exclusion |
| `ub_bad.rs::default_c_build_bad_is_undefined_behaviour_characterization` | runs the `-O0` `bad()` in a forked child and characterises clean-return **or** SIGSEGV |
| `ub_bad.rs::rust_bad_never_crashes` | the Rust `bad()`/`driver(0)` are deterministic: always clean return, zero output |
| `ub_bad.rs::nonzero_branch_is_deterministic_against_default_build` | contrast: `driver(non-zero)` *is* compared against the default `-O0` build |
| `harness_selftest.rs::null_guard_is_load_bearing_in_both_implementations` | the guard cannot be dropped: gcc lowers `printf("%s\n",…)` to `puts`, and `puts(NULL)` segfaults |
| `harness_selftest.rs::mutant_*` | negative controls proving these rows actually detect divergence |

### Note on row 1's importance

While building the mutation tests, deleting the `NULL` guard was found to
**SIGSEGV** rather than print `(null)`: gcc rewrites `printf("%s\n", line)` into
`puts(line)` even at `-O0`, and `puts(NULL)` dereferences null. The guard is
therefore load-bearing in the C original, and the Rust translation must keep an
equivalent check — which it does (`if !line.is_null()`). Row 1 is the test that
protects this.
