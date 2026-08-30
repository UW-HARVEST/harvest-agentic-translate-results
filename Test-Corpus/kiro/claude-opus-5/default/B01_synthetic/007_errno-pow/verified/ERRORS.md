# Differential verification of `c_src/src/main.c` against `translation/`

The C program is the ground truth. `translation/tests/differential.rs` builds
both executables, runs them as subprocesses with identical `argv`, and requires
byte-identical stdout, byte-identical stderr and an identical exit status.

* C build: `cmake -S c_src -B c_src/build && cmake --build c_src/build`
  → `c_src/build/driver`
* Rust build: `cd translation && cargo build --release`
  → `translation/target/release/driver`
* Both executables are copied into sibling sandbox directories as `driver` and
  invoked as `./driver`, so `argv[0]` — which the usage message prints — is the
  same string for both programs.

Beyond `cargo test`, roughly 25,000 additional argument pairs were compared with
an out-of-band fuzz harness: the exhaustive cross product of ~60 hand-picked
tokens, random decimal/hex/garbage strings, random 64-bit doubles fed in as C
hex-float literals, and dense sweeps across the subnormal, overflow and
underflow boundaries of both `strtod` and `pow`.

## Mismatches found and fixed

### 1. `SIGPIPE`: Rust exited 0 where C is killed by signal 13

**Symptom.** With the read end of stdout (or stderr) closed, the C program dies
from `SIGPIPE` — shell status 141 — while the Rust program exited 0 and produced
no output:

```
$ ./driver 10 308 | true ; echo ${PIPESTATUS[0]}
141        # C
0          # Rust, before the fix
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. A C program inherits the default disposition (`SIG_DFL`), so its first
failing `write` terminates the process. The Rust code additionally discarded the
`io::Error` from `write_all`, so nothing surfaced at all.

**Fix.** `restore_default_sigpipe()` in `translation/src/main.rs` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, restoring the C
disposition. Discarding the write error afterwards is correct: C's `printf`
likewise ignores a failed write and still `return`s 0/1 (verified by redirecting
to `/dev/full`, where both programs exit 0 for the success path and 1 for the
error path).

**Test.** `closed_stdout_pipe_kills_both_programs_with_sigpipe` in
`translation/tests/differential.rs`. It uses `std::io::pipe()` and drops the read
end *before* spawning, so the child's first write is guaranteed to hit `EPIPE` —
the test is deterministic rather than racing the child.

Note that this class of mismatch is invisible to a test that compares
`ExitStatus::code()`, because a signalled process reports `None` on both sides.
`assert_same` therefore compares the whole `ExitStatus`.

## Behaviours that were verified and did *not* mismatch

These are the places a translation of this program is most likely to go wrong.
Each was probed specifically; the existing Rust code already matched glibc, so
they are recorded here as verified rather than as defects.

| Behaviour | Why it is a trap | Verdict |
| --- | --- | --- |
| Check order per argument: `ERANGE` before `*endptr != '\0'` | `1e999abc` overflows *and* has trailing junk. C reports the range error, not the invalid-input error. | matches |
| Base validated fully before the exponent is touched | `driver abc def` must print only the base message. | matches |
| Empty argument accepted as `0` | `strtod("")` performs no conversion, so `endptr == nptr` and `*endptr == '\0'`; the C therefore treats `""` as a valid `0.0`. `driver "" ""` prints `Result: 1.00`. | matches |
| `endptr` reset to `nptr` when no conversion happens | `" "`, `"+"`, `"--3"`, `"."`, `"0x"`, `"1e"` all leave `endptr` at the *start*, so the echoed string in the error message is the whole argument. | matches |
| glibc `strtod` extensions | `inf`/`infinity`/`nan`/`nan(chars)` case-insensitively, C99 hex floats, hex with an optional `p` exponent (`0x10` → 16), `0x` alone consuming just the `0`. | matches |
| Underflow `ERANGE` = tininess **after rounding** **and** inexactness | `strtod("0x1p-1074")` is an exact subnormal → no `ERANGE`; `strtod("1e-320")` is inexact and tiny → `ERANGE`. Getting this wrong flips exit status on a whole band of inputs. | matches |
| `2.2250738585072012e-308` vs `...13e-308` vs `...14e-308` | Adjacent decimals straddling `DBL_MIN`; only the first sets `ERANGE`. | matches |
| glibc `pow` errno rules | `EDOM` only for a negative finite base with a non-integral finite exponent; `ERANGE` for the pole `pow(±0, y<0)`, for overflow, and for underflow **to zero** — a merely subnormal non-zero result sets nothing. No errno at all for NaN operands, `y == 0`, `x == 1`, or any infinite operand. | matches |
| `pow` underflow boundary | Swept `2^e` for `e ∈ [-1080, -1060]` and `10^e` for `e ∈ [-330, -300]`: `10^-323` is subnormal and quiet, `10^-324` is a range error. | matches |
| `printf("%.2f")` exactness and tie-breaking | glibc converts the exact binary value and rounds half to even (`0.125` → `0.12`, `0.375` → `0.38`). Verified over `k/2^n` ties and 2,000 random doubles. | matches (Rust's `{:.2}` is exact and rounds half to even) |
| `%.2f` of huge magnitudes | `10^308` prints all 309 integer digits of the exact double, not `1e308`. | matches |
| `%.2f` of specials and signed zero | `inf`, `-inf`, `nan`, `-nan` (glibc prints the NaN sign), `-0.00`. | matches |
| `argv` is bytes, not UTF-8 | Error messages echo the argument verbatim; invalid UTF-8 such as `\xff\xfe` must pass through unchanged. Handled via `OsStrExt`. | matches |
| Locale | The C never calls `setlocale`, so it stays in the `C` locale and always prints `.` as the decimal point regardless of `LC_NUMERIC`. No divergence to reproduce. | matches |
| Very long inputs | 100,000-digit integers, 1,000 nines, 300 hex fraction digits, 30-digit exponents — no overflow of the exponent accumulator, no panic. | matches |

## Completion status

* Both programs build without errors.
* `cargo test` passes in `translation/`: 5 unit tests + 31 differential tests,
  0 failed, 0 ignored. No test is disabled, skipped or `#[ignore]`d.
* Nothing under `c_src/` was modified; the only addition is the `c_src/build/`
  output directory produced by CMake. The test harness itself configures the C
  build out-of-tree into `translation/target/c_build` when
  `c_src/build/driver` is absent, so it never writes into the C source tree.
