# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

The complete C source, comments stripped
(`gcc -fpreprocessed -dD -E -P c_src/src/main.c`):

```c
#include <stdio.h>
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) {
        printf("%d %d\n", i, j);
    }
}
int main() {
    int x = 0;
    scanf("%d", &x);
    driver(x);
    return 0;
}
```

Grepping the source for every rejection construct
(`grep -nE 'assert|RETURN|return|NULL|errno|exit|abort|<|>|==|!=|\?|INT_|MAX|MIN|if|switch|#if'`)
yields exactly three hits:

| line | text | classification |
|------|------|----------------|
| 24 | `#include <stdio.h>` | not a check |
| 27 | `for (int i = 0, j = 0; i < x; i++, j += 2)` | **the only explicit range check in the program**: `i < x` |
| 36 | `return 0;` | unconditional success return of `main` |

So the program contains:

* **no** `assert`, `RETURN_ERROR`-style macro, error enum, `errno` use, `exit`/`abort`;
* **no** pointer parameters at all → **no null checks are possible** (`driver` takes
  an `int` by value, `main` takes nothing);
* **no** enums anywhere → there is **no out-of-range-enum class of input** for this
  API (an out-of-range `int` for `driver` *is* covered — rows 1–6);
* **no** length/size parameters → the "zero length / oversized length" class maps
  onto `x = 0` and `x = INT_MAX` (rows 1 and 6);
* exactly **one** guard, `i < x`, which "rejects" every `x <= 0` by executing the
  loop body zero times;
* two libc calls whose failure modes are part of the observable C behaviour:
  `scanf` (return value **discarded**, so the only observable effect of a failure
  is that `x` keeps its initializer `0`) and `printf` (return value discarded).

Because `scanf`'s result is thrown away, every `scanf` failure mode collapses to
the same observable: `x` is left at `0`. The rows below still enumerate each
distinct failure mode separately, because the Rust translation re-implements the
`%d` conversion by hand and each branch of that re-implementation must be
exercised.

`x` values are produced by glibc `%d`, which converts with `strtol` into a
`long` and then stores `(int)` of it — so out-of-`int` inputs are **truncated**,
and out-of-`long` inputs **saturate** to `LONG_MAX`/`LONG_MIN` first.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `driver` | `x == 0` — loop guard `0 < 0` false | returns normally, writes **0 bytes** | `err_01_driver_zero` |
| 2 | `driver` | `x == -1` | returns normally, 0 bytes | `err_02_driver_minus_one` |
| 3 | `driver` | `x == INT_MIN` (`-2147483648`) | returns normally, 0 bytes | `err_03_driver_int_min` |
| 4 | `driver` | `x == INT_MIN + 1` (one step past the low end of "valid") | returns normally, 0 bytes | `err_04_driver_int_min_plus_one` |
| 5 | `driver` | arbitrary randomized `x < 0` | returns normally, 0 bytes | `err_05_driver_random_negative` |
| 6 | `driver` | `x == INT_MAX` (oversized) — 2³¹−1 iterations, `j` signed-overflows at `i == 2³⁰` (C UB) | starts emitting `0 0\n1 2\n…`; unbounded output, so only the leading bytes are comparable | `err_06_driver_int_max_prefix` |
| 7 | `main` | stdin is **empty** → `scanf` input failure at the very first `inchar()` | `scanf` returns `EOF`, `x` stays `0`, `driver(0)` writes 0 bytes, exit status 0 | `err_07_main_empty_stdin` |
| 8 | `main` | stdin is **whitespace only** (`" \t\n\v\f\r"`) → whitespace skipped, then EOF | `EOF`, `x = 0`, 0 bytes, exit 0 | `err_08_main_whitespace_only` |
| 9 | `main` | first non-space byte is not a sign or digit (`"abc"`) → matching failure | `x = 0`, 0 bytes, exit 0 | `err_09_main_non_numeric` |
| 10 | `main` | `"-"` followed by a non-digit (`"-a"`) → sign consumed, matching failure | `x = 0`, 0 bytes, exit 0 | `err_10_main_minus_then_non_digit` |
| 11 | `main` | `"+"` followed by a non-digit (`"+x"`) → matching failure | `x = 0`, 0 bytes, exit 0 | `err_11_main_plus_then_non_digit` |
| 12 | `main` | `"-"` then EOF → sign consumed, then input failure | `x = 0`, 0 bytes, exit 0 | `err_12_main_minus_then_eof` |
| 13 | `main` | `"+"` then EOF → input failure | `x = 0`, 0 bytes, exit 0 | `err_13_main_plus_then_eof` |
| 14 | `main` | leading NUL byte (`"\0" "5"`) — `'\0'` is neither space nor digit → matching failure | `x = 0`, 0 bytes, exit 0 | `err_14_main_leading_nul` |
| 15 | `main` | sign directly followed by another sign (`"--5"`, `"+-5"`) → matching failure | `x = 0`, 0 bytes, exit 0 | `err_15_main_double_sign` |
| 16 | `main` | decimal point / non-`%d` numeric syntax (`".5"`) → matching failure | `x = 0`, 0 bytes, exit 0 | `err_16_main_leading_dot` |
| 17 | `main` | positive value **overflowing `long`** (`"99999999999999999999999"`) → `strtol` saturates to `LONG_MAX`, stored as `(int)LONG_MAX == -1` | `x = -1`, 0 bytes, exit 0 | `err_17_main_overflow_long_pos` |
| 18 | `main` | negative value **overflowing `long`** (`"-99999999999999999999999"`) → `LONG_MIN`, `(int)LONG_MIN == 0` | `x = 0`, 0 bytes, exit 0 | `err_18_main_overflow_long_neg` |
| 19 | `main` | pathologically long digit run (10 000 digits) → same saturation path, exercises the unbounded conversion buffer | `x = -1`, 0 bytes, exit 0 | `err_19_main_10k_digits` |
| 20 | `main` | value inside `long` but **outside `int`**, truncating to a non-positive `int` (`"4294967296"` → `0`, `"2147483648"` → `INT_MIN`, `"-4294967296"` → `0`) | 0 bytes, exit 0 | `err_20_main_int_truncation_nonpositive` |
| 21 | `main` | value inside `long` but outside `int`, truncating to a **large positive** `int` (`"-2147483649"` → `INT_MAX`) | unbounded output; only the leading bytes are comparable | `err_21_main_int_truncation_to_int_max_prefix` |
| 22 | `main` | `"0x10"` — `%d` is base 10, so it converts `0` and stops at `'x'` (successful conversion, *not* an error, but the value is the rejecting one) | `x = 0`, 0 bytes, exit 0 | `err_22_main_hex_prefix` |
| 23 | `main` | `printf` write failure: stdout closed (`>&-` → `EBADF`) with a non-empty output | return value discarded; process still exits **0**, nothing on stdout | `err_23_main_stdout_closed` |
| 24 | `main` | `printf` write failure: stdout is a pipe whose reader closed → **SIGPIPE**, default disposition | process is **killed by signal 13** (`$? == 141`) | `err_24_main_sigpipe` |
| 25 | `driver`/`main` | stdin is a closed fd (`<&-` → `read` fails with `EBADF`, not EOF) | `scanf` input failure, `x = 0`, 0 bytes, exit 0 | `err_25_main_stdin_closed` |

### Notes on classes that do not exist in this API

* **Null pointers** — neither `driver(int)` nor `main(void)` accepts a pointer,
  so there is no null-pointer row to write. The generic-boundary requirement is
  satisfied instead by rows 1–6 (`x` at both extremes and one step past them)
  and by rows 23–25 (invalid file descriptors, the only "handles" the API uses).
* **Out-of-range enum values** — the C source declares no `enum`, so every
  32-bit bit pattern is a *valid* `int` for `driver`; row 5 fuzzes the negative
  half and row 6 the maximum. There is no representable `c_int` that `driver`
  rejects other than by the `i < x` guard.
* **Rows 6 and 21** produce ≈2³¹ lines of output. They are verified on a bounded
  prefix (the first 64 KiB written by each implementation, after which both
  children are killed), because materialising the full output is not feasible.
* **`j` overflow (`j += 2` at `i == 2³⁰`)** is signed-overflow UB in C and is not
  reachable in a bounded test; the Rust translation uses `wrapping_add`, which
  matches what the C compiler actually emits (`j = 2*i` truncated to 32 bits).

## Phase C results

All 25 rows have a passing differential test; `tests/phase_c_errors.rs` reports

```
running 29 tests
...
test result: ok. 29 passed; 0 failed
```

(25 table rows + 4 extra generic-boundary cases, rows 26–29 of the test file).
Every row is checked with **both** shared objects (`main` reached through
`dlopen` + `dlsym` in a fresh child process, so the buffered-stdin state is
per-invocation exactly as in C) **and** with both linked executables on a pipe
and on a regular file. Each assertion compares the stdout bytes *and* the exact
wait status (exit code plus terminating signal), not just "both failed".

### Divergence found and fixed

**SIGPIPE (row 24).** Rust's runtime installs `SIG_IGN` for `SIGPIPE` before
entering `main`, so the first translation kept running after the reader of its
stdout pipe went away and exited with status 0, whereas the C program is killed
by signal 13 (`$? == 141`). `src/main.rs` now restores `SIG_DFL` for `SIGPIPE`
as the first thing the binary does. The reset deliberately lives in the *binary*
entry point only: the `main` exported from the `cdylib` must inherit whatever
disposition the host process installed, which is what the C shared object does.
Removing the reset makes `err_24_main_sigpipe` fail, so the test is load bearing.

### Rejections that are provably unobservable

`x` influences the program only through the loop guard `i < x`, so **every**
`x <= 0` produces byte-identical output (nothing). That makes the following
distinctions unobservable through the public API, by construction:

* positive `long` overflow → `(int)LONG_MAX == -1` versus any other non-positive
  value;
* negative `long` overflow → `(int)LONG_MIN == 0` versus any other non-positive
  value;
* `scanf` returning `EOF` (input failure) versus `0` (matching failure) — the C
  `main` discards the return value.

The rows above still exist and are still tested, because the Rust code
re-implements the `%d` conversion by hand and each of those branches has to be
reached; they are simply not distinguishable *from each other* by output. Rows 20,
21 and 26 (in `CONFIGS.md`) pin down the value of `x` precisely wherever the
truncated result is positive, which is where a wrong conversion becomes visible.

### Testing pitfall found and fixed

`cargo test` builds the package's `rlib` and binaries but **not** the `cdylib`
artifact. The first version of this suite therefore compared the C `.so` against
a *stale* `target/debug/libdriver.so` and reported success even when the Rust
source had been deliberately broken. `tests/common/mod.rs` now runs
`cargo build --offline` itself before the first comparison and then hard-fails if
either Rust artifact is older than anything in `src/`, so a stale-artifact pass
is impossible. Mutation testing (see the bottom of this file) confirms the fix.

### Mutation check (does the suite actually detect divergence?)

| mutation applied to `src/` | result |
|----------------------------|--------|
| `j += 2` → `j += 1` | 26 of 32 Phase B rows fail |
| `'\v'` removed from the `isspace` set | 4 rows fail |
| `'+'` sign no longer accepted | 5 rows fail |
| `while i < x` → `while i <= x` | 24+ rows fail |
| saturating instead of truncating `long`→`int` store | 2 rows fail |
| `-` sign ignored (`wrapping_neg` dropped) | 6 rows fail |
| `restore_default_sigpipe()` removed | `err_24_main_sigpipe` fails |
| overflow saturation value changed | *survives* — provably unobservable (see above) |
| `return None` → `return Some(0)` on matching failure | *survives* — `x` is initialised to `0` anyway |
| explicit `out.flush()` removed | *survives* — `BufWriter`'s `Drop` flushes |

The three survivors are equivalent mutants: they cannot change the bytes the
program writes for any input.
