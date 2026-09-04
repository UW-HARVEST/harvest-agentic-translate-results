# ERRORS.md — error / rejection surface table (Phase C)

## Derivation (mechanical)

Every construct in `c_src/` that can reject, error, or take an "invalid input"
branch was collected with:

```sh
grep -nE "return"                                   c_src/src/*.c c_src/src/*.h
grep -nE "assert|NULL|errno|exit\(|abort|error|ERR" c_src/src/*.c c_src/src/*.h
grep -nE "\bif\b|\bswitch\b|\bdefault\b|\bcase\b|#if|#ifndef|#ifdef" c_src/src/*.c c_src/src/*.h
grep -nE "fprintf|stderr"                           c_src/src/*.c c_src/src/*.h
```

Results of the grep, verbatim scope:

* `assert|NULL|errno|exit(|abort|error|ERR` → **zero matches**. There is no
  `RETURN_ERROR` macro, no error enum, no sentinel return, no `assert`, no
  `NULL` check, no `errno` use anywhere in the project.
* the only `if` in the whole project is `mdmain.c:29` (`argc < 3`).
* the only `switch` is `DISPATCH_REP` (`mdmacros.h:83`), whose `default:`
  (`mdmacros.h:91`) is the sole silent-rejection branch.
* the only `stderr` write is `mdmain.c:30`.
* no exported function takes a pointer, an array, a length, a size, or an enum,
  so the classic C rejection surface (null pointer / zero length / oversized
  length / out-of-range enum tag) **does not exist** in this API. Rows 9–11
  record that explicitly rather than inventing checks the C does not perform.

## Table

One row per distinct rejection / invalid-input branch. `INIT_<OP>` is `0` for
`OP=add` and `OP=sub`, `1` for `OP=mul` (`mdmacros.h:56-58`).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `main` (`mdmain.c:29-32`) | `argc < 3` with **0** operands: `driver` | writes `usage: <argv[0]> A B\n` to **stderr**, empty stdout, exit status **2** | `errors.rs::e01_argc_zero_operands` | [x] |
| 2 | `main` (`mdmain.c:29-32`) | `argc < 3` with **1** operand: `driver 7` | same as row 1 (`usage: <argv[0]> A B\n`, stderr, exit **2**) | `errors.rs::e02_argc_one_operand` | [x] |
| 3 | `use_generated` → `accum_<OP>` `default:` (`mdmacros.h:91`) | `n == 7` — first value past the last `case` | **no error signalled**: `switch` falls to `default: break;`, `acc` keeps `INIT_<OP>`; prints `gen.acc=<INIT_<OP>>\n`; returns `INIT_<OP>` | `errors.rs::e03_use_generated_seven` | [x] |
| 4 | `use_generated` → `accum_<OP>` `default:` | `n` negative (`-1`, `-7`, `INT_MIN`) | returns `INIT_<OP>`, prints `gen.acc=<INIT_<OP>>` | `errors.rs::e04_use_generated_negative` | [x] |
| 5 | `use_generated` → `accum_<OP>` `default:` | `n` far above range (`8`, `100`, `1<<20`, `INT_MAX`) | returns `INIT_<OP>`, prints `gen.acc=<INIT_<OP>>` | `errors.rs::e05_use_generated_oversized` | [x] |
| 6 | `op_add` (`mdcore.c:28`) | signed-`int` overflow, e.g. `(INT_MAX, 1)`, `(INT_MIN, -1)`, `(INT_MAX, INT_MAX)` | **no rejection**: `a + b`. gcc emits a two's-complement `add`, so the result wraps. (Formally UB in C; the C binary is ground truth and it wraps.) | `errors.rs::e06_op_add_overflow` | [x] |
| 7 | `op_sub` (`mdcore.c:29`) | signed-`int` overflow, e.g. `(INT_MIN, 1)`, `(INT_MAX, -1)`, `(INT_MIN, INT_MAX)` | no rejection; wraps | `errors.rs::e07_op_sub_overflow` | [x] |
| 8 | `op_mul` (`mdcore.c:30`) | signed-`int` overflow, e.g. `(INT_MAX, 2)`, `(INT_MIN, -1)`, `(65536, 65536)` | no rejection; wraps (low 32 bits) | `errors.rs::e08_op_mul_overflow` | [x] |
| 9 | `helper_call` / `helper_ptr` / `use_generated` accumulator | `INIT_<OP>` + `REPEAT` steps overflow, e.g. `OP=mul` with `REPEAT=7`, or `op_mul` result `+ acc` overflowing | no rejection; wraps at every step. `OP=mul, REPEAT=7`: `1*1*2*3*4*5*6*7 = 5040` (no wrap), but `helper_call`'s `r + acc` can still wrap for large `a,b` | `errors.rs::e09_accumulator_overflow` | [x] |
| 10 | *(generic boundary — no such check in C)* every exported function | out-of-range **enum** value across FFI | **not applicable**: `nm -D` shows 6 functions, all taking only `int`; there is no `enum` type in `c_src/`. The nearest analogue is an `int` with no matching `case`, which is row 3–5. Every 32-bit pattern is a legal input to all six functions. | `errors.rs::e10_no_enum_surface` (asserts every function accepts the full `int` range, fuzzed) | [x] |
| 11 | *(generic boundary — no such check in C)* every exported function | null pointer / zero length / oversized length | **not applicable**: no exported function has a pointer or length parameter. The only pointers in the API are the two `.data` globals; `G_OP` holds a callee address supplied by the caller, and setting it to `NULL` and calling through it is a caller bug that faults identically in C and Rust (not asserted — it would abort the test process). Documented, not stubbed. | `errors.rs::e11_no_pointer_parameters` (asserts the ABI shape) | [x] |
| 12 | `atoi` in `main` (`mdmain.c:34-35`) | operand is not a number at all (`""`, `"abc"`, `"--5"`, `"0x10"`, `" \t+12xyz"`) | **no error**: glibc `atoi` parses optional space, optional sign, then digits and stops; unparsable ⇒ `0`. `"0x10"`⇒`0`, `" \t+12xyz"`⇒`12`, `"--5"`⇒`0`. Program still exits **0**. | `errors.rs::e12_atoi_garbage` | [x] |
| 13 | `atoi` in `main` (`mdmain.c:34-35`) | operand out of `int` range (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) | no error: glibc `atoi` is `(int)strtol`, so the value saturates at `long` bounds and is then truncated to `int` (`"2147483648"`⇒`-2147483648`, `"99999999999999999999"`⇒`-1`). Exit **0**. | `errors.rs::e13_atoi_out_of_range` | [x] |

## Build-time rejections (compile-time, not runtime)

Not runtime rows, but they define the valid configuration space used by
`CONFIGS.md`, and they are verified in `cbuild/build_c.sh` / the feature-combo
loop:

| condition | C result | Rust equivalent |
|-----------|----------|-----------------|
| `-DREPEAT=8` (or any value with no `REPn`) | hard **compile error**: `CHOOSE_REP(8)` → `REP8(add, acc)` → `error: 'add' undeclared` (verified: `gcc -c -DREPEAT=8` exits 1) | no `repeat_8` feature exists in `Cargo.toml`; `REPEAT` is constrained to `0..=7` |
| `-DOP=div` (or any token without an `INIT_`/`STEP_`/`op_` family) | hard **compile error**: `error: 'INIT_div' undeclared` (verified) | no `div` feature exists; only `add`/`sub`/`mul` |
| `OP`/`REPEAT` left undefined | `mdmacros.h:27-32` defaults them to `add` / `5` | `--no-default-features` with no `add`/`repeat_*` feature falls back to `add` / `5` (verified by `cargo check --no-default-features`) |

## Verification evidence

Rows 1–13 all pass in **all 24** `(OP, REPEAT)` configurations, in both the
debug and `--release` profiles, and additionally against a `-O0` C build (to
confirm the observed signed-overflow wrapping in rows 6–9 is not an artefact of
`-O2`):

```
$ ./cbuild/run_all.sh                     # 42 tests x 24 configs
configurations passed: 24   failed: 0
$ PROFILE=release ./cbuild/run_all.sh
configurations passed: 24   failed: 0
```

Test-to-row mapping lives in `tests/errors.rs`; each test is named `eNN_...`
after its row number.

### Non-vacuity (fault injection)

To confirm the error-path tests can actually fail, five faults were injected into
the Rust and then reverted:

| injected fault | error-path tests that caught it |
|----------------|---------------------------------|
| `run_loop` uses `i <= REPEAT` (one extra unrolled step) | `e09`, `e10`, `e12`, `e13` (+8 Phase-B tests) |
| `accum` given a `7 =>` arm, i.e. "fixing" the missing `case 7` | `e03`, `e09`, `e10` (+5 Phase-B tests) |
| `OP_NAME` misspelled `"Add"` | `e12`, `e13` (+4 Phase-B tests) |
| `STEP_mul` uses `acc *= i` instead of `acc *= (i+1)` | `e09`, `e10`, `e12`, `e13` (+13 Phase-B tests) |
| `G_OP` reverted to an immutable Rust `static` (read-only `.data.rel.ro`) | `globals.rs` writability pre-flight (previously a SIGSEGV) |
