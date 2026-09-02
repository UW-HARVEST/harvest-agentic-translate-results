# Differential verification log — C vs. Rust `driver`

Ground truth: `c_src/src/main.c`, built with the shipped `c_src/CMakeLists.txt`
(no `CMAKE_BUILD_TYPE`, i.e. unoptimized, gcc/glibc on x86-64 Linux).
Program under test: `translation/src/main.rs`, built with `cargo build --release`.

Comparison method: both programs are spawned as subprocesses with the same bytes on
stdin; stdout, stderr and exit status are compared byte for byte
(`translation/tests/differential.rs` + `translation/tests/harness/mod.rs`).
The Rust code is never loaded as a library.

## Result

**No behavioral mismatch was found.** Every input class enumerated below produces
identical stdout, identical stderr (always empty) and the same exit status (always 0)
in both programs. The sections below record what was checked, the C semantics that had
to be reproduced for that to be true, and how the tests were proven to be capable of
detecting a mismatch.

## What the C program does

```c
void bad()  { int *data;  printIntPtrLine(data); }        /* uninitialized pointer */
void good() { int data = 5; int *p = &data; printIntPtrLine(p); }
int  main() { int x = 0; scanf("%d", &x); if (x) good(); else bad(); return 0; }
```

`main` has one branch, but `scanf("%d", &x)` supplies many distinct input classes that
select it. `good()` prints `5\n`; `bad()` is the undefined-behavior path.

## Hazards that had to be reproduced, and how each was confirmed

### 1. `bad()` dereferences an uninitialized `int *` (CWE-457)

This is undefined behavior, so there is no "correct" value to print — only what the
reference build actually does. Disassembly of the reference binary shows `bad()`
loading its pointer straight out of an untouched stack slot:

```
bad:  sub $0x10,%rsp ; mov -0x8(%rbp),%rax ; mov %rax,%rdi ; call printIntPtrLine
```

The slot holds residue from the preceding `scanf` call frame. Empirically the C program
prints `0\n` and exits 0, **deterministically**. Confirmed stable across:

- 25 repeated runs of the same input,
- environment-block padding of 0 / 1 / 100 / 4000 bytes (shifts the initial stack),
- absolute vs. relative `argv[0]`,
- ~40 differently shaped `bad()`-reaching inputs (EOF, matching failure, parsed zero,
  truncated-to-zero, 64 KiB of junk), which drive materially different glibc `scanf`
  code paths, including ones that grow an internal work buffer.

The translation therefore models the read as `0`, behind a named constant
(`UNINITIALIZED_POINTER_READ`) so the origin of the value is not mistaken for program
logic. Note this is a *pinned observation of undefined behavior*, not a guarantee: on a
different compiler, optimization level or libc the C program could print something else
or fault. The tests compare against the C program on every run, so such a divergence
would surface as a test failure rather than being silently accepted.

### 2. `scanf("%d")` skips whitespace, including newlines

`%d` skips leading `isspace()` bytes before converting, so it reads *across* newlines —
unlike `fgets`. All six C-locale space bytes must be skipped:
`' '`, `'\t'`, `'\n'`, `'\v'` (0x0b), `'\f'` (0x0c), `'\r'`.

Verified with `"   \n\n  7"` → `5\n` (`good()`) and with 65 536 newlines followed by `7`.
A mutation that narrowed the whitespace set to `' '`/`'\t'` was detected by the suite.

### 3. A failed conversion leaves `x` at its initializer, so `x == 0` → `bad()`

EOF (empty input, `/dev/null`, closed fd), whitespace-only input, a non-numeric first
byte, and a sign not followed by a digit (`"+"`, `"-"`, `"+ 5"`, `"--5"`) all leave `x`
untouched at `0`. The C program does **not** check `scanf`'s return value, so all of
these silently take the `bad()` path — no error message, exit 0. Reproduced by returning
`None` from `scanf_int` and leaving `x` at `0`.

### 4. glibc converts `%d` through `strtol`: saturation, then truncation

This is the subtlest part, and it changes which branch runs:

- Values outside `long` range saturate at `LONG_MAX` / `LONG_MIN` (they do **not** wrap).
- The saturated `long` is then **truncated** into the `int` argument.

Consequences that were confirmed against the C binary:

| stdin | `long` value | stored `int` | branch | stdout |
|---|---|---|---|---|
| `2147483648` | 2147483648 | -2147483648 | `good()` | `5\n` |
| `4294967296` (2^32) | 4294967296 | **0** | `bad()` | `0\n` |
| `8589934592` (2^33) | 8589934592 | **0** | `bad()` | `0\n` |
| `9223372032559808512` | as written | **0** | `bad()` | `0\n` |
| `9223372036854775807` (`LONG_MAX`) | `LONG_MAX` | -1 | `good()` | `5\n` |
| `18446744073709551616` (2^64) | saturates to `LONG_MAX` | -1 | `good()` | `5\n` |
| `-9223372036854775808` (`LONG_MIN`) | `LONG_MIN` | **0** | `bad()` | `0\n` |
| `-18446744073709551616` | saturates to `LONG_MIN` | **0** | `bad()` | `0\n` |

The two models are distinguishable: 2^64 wraps to `0` under a wrapping model (which
would select `bad()`), but the C program prints `5\n`, proving saturation. Likewise
`4294967296` proves truncation rather than clamping — a clamping model would yield
`INT_MAX` and select `good()`, while the C program prints `0\n`.

The translation accumulates into `i64`, sets an overflow flag on `checked_mul`/
`checked_add` failure, saturates to `i64::MIN`/`i64::MAX`, then performs `as i32`.
Mutations replacing the saturation with wrapping, and the truncation with `clamp`,
were both detected by the suite.

### 5. Base 10 only

`%d` is decimal, so `"0x10"` converts `0` and stops at `'x'` → `bad()`, not 16.
Same for `"0b1"`. Leading zeros (`"007"`, 5 000 zeros then `1`) are fine.

### 6. Output formatting and exit status

`printf("%d\n", ...)` — one decimal integer, one trailing newline, nothing on stderr,
`return 0` always. There is no input for which the C program writes to stderr or exits
nonzero, so the Rust program must not either; a Rust panic would violate both. Mutations
adding an `eprintln!` and an `exit(1)` were detected.

## Test-suite validity check (mutation testing)

Passing tests only mean something if they can fail. Eight deliberate defects were
injected into `translation/src/main.rs` one at a time; **all eight were caught**, and the
suite passed again once the file was restored byte-identical:

| injected defect | caught |
|---|---|
| branch condition `x >= 0` instead of `x != 0` | yes |
| dropped the trailing `\n` from the printed line | yes |
| clamped instead of truncated the `long`→`int` store | yes |
| wrapped instead of saturating on `long` overflow | yes |
| exit code 1 instead of 0 | yes |
| stopped treating `\n`/`\r`/`\v`/`\f` as skippable whitespace | yes |
| `bad()` printing `32767` instead of `0` | yes |
| stray `eprintln!` on stderr | yes |

## Coverage of the C source

Every statement and branch is reached:

- `main`: both arms of `if (x)`, and `scanf` succeeding, failing to match, and hitting EOF.
- `good()` → `printIntPtrLine` with a valid pointer: `nonzero_values_take_the_good_branch`.
- `bad()` → `printIntPtrLine` with the uninitialized pointer: `empty_input`,
  `whitespace_only_inputs`, `matching_failure_*`, `zero_values_take_the_bad_branch`,
  `long_to_int_truncation_to_zero_takes_the_bad_branch`, the negative half of
  `long_boundaries_and_overflow_saturation`.
- `printIntPtrLine`: exercised by both callers; it has no branches.

## Files

- `translation/tests/harness/mod.rs` — builds the C program if needed (out-of-tree, so
  `c_src/` is never written to), spawns both binaries, compares stdout/stderr/status.
- `translation/tests/differential.rs` — 18 tests over the input classes above. None is
  `#[ignore]`d, skipped or otherwise disabled.

Nothing under `c_src/` was modified; `md5sum` of `c_src/src/main.c` and
`c_src/CMakeLists.txt` is unchanged. The only addition there is the `build/` directory
produced by the prescribed CMake invocation, which contains build output only.
