# MUTANTS.md — does the differential suite actually CATCH bugs?

Passing tests only mean something if they would fail on a wrong implementation.
`./mutants.sh` injects 35 deliberate bugs into `src/lib.rs`, runs the whole
differential suite against each, and reports whether the suite noticed.

Detection is measured by the **test-runner exit code**, not by counting
`FAILED` lines: several mutants make the Rust `.so` crash (SIGSEGV/SIGABRT),
which kills the libtest process before it can print a per-test verdict.

## Result

```
=== killed: 32   survived: 3 ===
```

| # | mutant | killed? | failing tests |
|---|--------|---------|---------------|
| 1 | charset drops `'e'` | KILLED | 10 |
| 2 | charset drops `'+'` | KILLED | 4 |
| 3 | charset drops `'.'` | KILLED | 12 |
| 4 | charset gains `' '` | KILLED | 2 |
| 5 | scan continues past `default:` | KILLED | 1 |
| 6 | `can_access_at_index` uses `<=` not `<` | KILLED | 2 |
| 7 | `can_access_at_index` off-by-one low | KILLED | 2 |
| 8 | `can_access_at_index` ignores `offset` | KILLED | crash |
| 9 | `can_access_at_index` saturating add | **SURVIVED** | — (equivalent, see below) |
| 10 | no `content == NULL` check | KILLED | crash |
| 11 | no `input_buffer == NULL` check | KILLED | crash |
| 12 | no `'\0'` terminator on the temp buffer | KILLED | 2 |
| 13 | `memcpy` copies one byte too few | KILLED | 2 |
| 14 | `strtod` failure ignored | KILLED | 14 |
| 15 | `strtod` treated as always failing | KILLED | 2 |
| 16 | returns `true` on `parse_error` | KILLED | 14 |
| 17 | returns `false` on success | KILLED | 2 |
| 18 | `INT_MAX` saturation writes `INT_MIN` | KILLED | 6 |
| 19 | the two saturation branches swapped | KILLED | 7 |
| 20 | `(int)number` rounds instead of truncating | KILLED | 7 |
| 21 | `(int)number` ceils instead of truncating | KILLED | 8 |
| 22 | NaN maps to `0` instead of `INT_MIN` | **SURVIVED** | — (dead code, see below) |
| 23 | `cJSON_Number` constant wrong (`1<<4`) | KILLED | 2 |
| 24 | `item->type` not written | KILLED | 2 |
| 25 | `item->valuedouble` not written | KILLED | 2 |
| 26 | `*item` written on the failure path | KILLED | 14 |
| 27 | `input_buffer->depth` clobbered | KILLED | 2 |
| 28 | `input_buffer->length` clobbered | KILLED | 2 |
| 29 | `offset +=` the full scanned run instead of what `strtod` consumed | KILLED | 4 |
| 30 | `offset` overwritten instead of incremented | KILLED | 3 |
| 31 | `offset` not advanced at all | KILLED | crash |
| 32 | `decimal_point` becomes `','` | KILLED | 12 |
| 33 | `'.'`-rewrite loop always runs | **SURVIVED** | — (no-op, see below) |
| 34 | `item` store via checked deref `(*item).f = v` | KILLED | 2 |
| 35 | `input_buffer` read via checked deref `(*buffer).length` | KILLED | 2 |

Mutants 34 and 35 are the two **real bugs this exercise found** (see the
"Divergences found and fixed" table in `ERRORS.md`); they are kept in the mutant
list as regression guards.

## The three survivors are provably semantics-preserving

They are *equivalent mutants*, not test blind spots. Each has a dedicated test
that establishes why no input can distinguish it.

### 9. `can_access_at_index`: `wrapping_add` → `saturating_add`

The two differ only when `offset + index` overflows `size_t`. The loop condition
is evaluated at `index == 0` first, so the body is entered only when
`offset < length`; from then on `index` grows at most to `length - offset`, giving
`offset + index <= length <= usize::MAX`. Overflow is therefore reachable only at
`index == 0`, where wrapping and saturating arithmetic agree (`offset + 0`).

`wrapping_add` is kept because it is the faithful translation of C's `size_t`
addition in the macro.

*Test:* `offset_plus_index_never_overflows` — sweeps `offset ∈ {SIZE_MAX,
SIZE_MAX-1, SIZE_MAX-31, SIZE_MAX/2, 2^63}` against every `length` with
`offset >= length` (the only region where overflow is possible) and asserts
nothing is consumed, plus the complementary `offset < length` region.

### 22. NaN arm of `double_to_int_c` returns `0` instead of `INT_MIN`

Unreachable. `strtod` returns NaN only for the spellings `nan` / `nan(...)`, and
`inf`/`infinity` for infinity — none of which can appear in the temporary buffer,
because the C `switch` admits only `[0-9+-.eE]`. Hexadecimal floats (`0x…p…`) are
likewise impossible (`x`/`p` are not in the charset). So `strtod` here always
yields a finite value or `±HUGE_VAL`, and the NaN arm is dead code.

The arm is kept because it documents the x86-64 `cvttsd2si` behaviour the C cast
would have exhibited.

*Test:* `strtod_never_returns_nan_over_the_accepted_charset` — asserts
`!is_nan()` for **every** charset string of length 1..5 (813 615 inputs) plus the
overflow/underflow shapes.

### 33. `if has_decimal_point != false` → `if true`

The loop body replaces `'.'` with `decimal_point`, and `decimal_point` is
hard-coded to `'.'` at `lib.c:18`. The loop is a no-op in either case, and it
only ever runs over the temp buffer (never the caller's `const` content). Running
it unconditionally cannot change any output.

Note that mutant 32 (`decimal_point = ','`) IS killed by 12 tests, which proves
the suite does observe the loop's *effect* — it is only the guard that is
redundant.

*Test:* `rewrite_loop_is_a_no_op`, plus the exhaustive charset sweeps which cover
inputs with and without `'.'`.

## Reproducing

```sh
./mutants.sh          # ~35 × (build + full suite)
```

The script restores `src/lib.rs` from a backup on exit (including on `Ctrl-C`).
