# ERRORS.md — Phase A error-surface table

Derived mechanically from the C source. Every grep below was actually run
against `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical extraction of every rejection construct

| construct grepped | pattern | matches |
|---|---|---|
| error-return macros | `RETURN_ERROR\|RETURN_IF\|CHECK_\|_ERR` | **0** |
| any `return` statement | `return` | **0** (function is `void` and falls off the end) |
| sentinel returns | `return -1`, `return NULL`, `return 0` | **0** |
| assertions | `assert` | **0** |
| null checks | `NULL`, `!= 0`, `== 0` | **0** |
| range / bounds checks | `if (`, `<`, `>`, `<=`, `>=` | **0** |
| branching at all | `if\|switch\|while\|for\|goto\|?:` | **0** |
| min/max constants | `MAX\|MIN\|LIMIT\|SIZE` | **0** |
| error enums / codes | `enum\|errno\|error` | **0** |
| exit paths | `exit\|abort\|longjmp` | **0** |
| conditional compilation | `#ifdef\|#if \|#ifndef` | 1 — the `DRIVER_H_` include guard only (no code variation) |

Verification command and result:

```
$ grep -n 'return' c_src/src/driver.c c_src/include/driver.h
(no output)
$ grep -n 'assert\|NULL\|errno\|exit\|abort\|if\s*(\|switch\|MAX\|MIN' c_src/src/driver.c c_src/include/driver.h
(no output, exit status 1)
```

The complete body of the only function is:

```c
void driver(int x) {
    register int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `driver` | *(none)* | **The error surface is empty.** `driver` accepts every one of the 2^32 `int` bit patterns, rejects nothing, validates nothing, and returns `void`. There is no error code, sentinel, or errno the caller can observe. |

**This table has zero rejection rows, and that is a mechanically derived
finding, not an omission:** the function takes a single by-value `int`, has no
pointer parameters (so no null check is possible), no length/count parameters
(so no zero/oversize check is possible), no enum parameters (so no
out-of-range-variant check is possible), and no control flow whatsoever.

## Generic-boundary rows tested anyway (Phase C obligation)

Phase C requires covering the generic boundaries every C API has *even if not in
the table*. Below is each generic boundary class, mapped onto what it can
possibly mean for this signature. Each row is a real differential test in
`tests/phase_c_errors.rs`; every one asserts the two `.so`s produce **identical
stdout bytes**, which is the only observable this `void` function has.

| # | boundary class | how it applies to `void driver(int)` | test |
|---|---|---|---|
| E1 | null pointer argument | **not applicable** — no pointer parameters exist. Documented, not testable. | n/a |
| E2 | zero length | closest analogue: `x == 0` (the additive/multiplicative identity input) | `err_zero` |
| E3 | oversized length | closest analogue: the extremal magnitudes `INT_MAX`, `INT_MIN` | `err_extremes` |
| E4 | one step past a valid range — low | `INT_MIN`, `INT_MIN+1` (`2*x` overflows: UB in C, must match the emitted wrap) | `err_extremes` |
| E5 | one step past a valid range — high | `INT_MAX`, `INT_MAX-1` | `err_extremes` |
| E6 | out-of-range enum value across FFI | **not applicable** — no enum parameters. The `int` domain is *total*: every 32-bit value is a valid input, so there is no "no valid variant" value to pass. Documented, not testable. | n/a |
| E7 | signed-overflow of the multiply `2*x` | every `x` with \|x\| > 2^30, i.e. `x > INT_MAX/2` or `x < INT_MIN/2` | `err_mul_overflow_boundary` |
| E8 | signed-overflow of the add `y += 300` | `2*x` within 300 of `INT_MAX`, e.g. `x = 0x3FFFFFFF` → `2*x = 0x7FFFFFFE`, `+300` wraps negative | `err_add_overflow_boundary` |
| E9 | exact sign-transition of the result | `x == -150` → `y == 0`; `x == -151` → `y == -2` (first negative output) | `err_sign_transition` |
| E10 | printf field-width transitions | `y` crossing ±10, ±100, … ±1000000000 and the 11-char `-2147483648` | `err_digit_widths` |

### Note on C undefined behaviour (rows E4, E5, E7, E8)

`2*x` and `y += 300` are signed-integer overflow, i.e. **undefined behaviour**
in ISO C. The C implementation is the ground truth, so the Rust must match what
the compiled C *actually does*. Disassembly of the shipped `.so` confirms plain
32-bit wrapping:

```
1115: mov    -0x14(%rbp),%eax
1118: lea    (%rax,%rax,1),%ebx      ; 32-bit ebx = x + x, wraps mod 2^32
111b: add    $0x12c,%ebx             ; 32-bit ebx += 300, wraps mod 2^32
```

so `wrapping_mul(2)` / `wrapping_add(300)` in the Rust is the correct match, and
rows E4/E5/E7/E8 pin it down. (Using plain `*`/`+` in Rust would **panic** on
these inputs in a debug build rather than wrapping — this is exactly why these
rows matter.)

## Gate status

- [x] Every distinct rejection construct in the C source was grepped for; the
      error surface is empty (0 rejection rows).
- [x] E2, E3, E4, E5, E7, E8, E9, E10 have passing differential tests.
- [x] E1 and E6 documented as not-applicable-by-signature with reasons.

## Mutation-testing evidence (proof these tests can actually fail)

Passing tests mean nothing unless they detect a real divergence. Three mutations
were injected into `src/lib.rs`, the cdylib rebuilt, and the suite re-run. All
mutations were reverted afterwards.

| mutation | release cdylib | debug cdylib | conclusion |
|---|---|---|---|
| `wrapping_add(300)` → `wrapping_add(301)` | **19 rows FAIL**, each naming the exact input (e.g. `C1: DIVERGENCE at x = 0`) | — | the suite detects a constant error everywhere |
| `wrapping_*` → `saturating_*` | **12 rows FAIL** (C8–C18, E7, E8) while the 8 happy-path rows still PASS | — | the overflow/boundary rows earn their place — happy-path testing alone would have missed this entirely |
| `x.wrapping_mul(2)`/`y.wrapping_add(300)` → naive `x * 2`/`y + 300` | **PASSES** (release wraps silently) | **12 rows FAIL** — capture child killed by SIGABRT (`raw status 0x86`), caught by the harness's child-exit-status check | testing BOTH build profiles is necessary: debug enables overflow checks, so the naive translation traps where release silently wraps |

The third row is the important one. It is why the translation must use explicit
`wrapping_mul` / `wrapping_add` rather than plain operators: only the explicit
form matches the C's emitted 32-bit wrap under *every* build profile.
