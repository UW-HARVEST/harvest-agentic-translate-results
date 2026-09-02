# Differential verification of the C → Rust translation

The C program in `c_src/` is the ground truth. This file records what was
compared, every mismatch found, and its cause.

Program under test: `driver <seed>` — seeds a glibc PRNG, fills a 262 144-entry
`int` array with `rand()`, applies a 100-step integer kernel to every element
2 000 times, then prints the XOR of the array with `printf("%d\n", …)`.

## How it was verified

Three layers, because one full run of the workload costs ~8 min (C, `-O0`) and
~5 min (Rust):

1. **Binary-level differential suite** — `translation/tests/differential.rs`.
   Runs both *built executables* as subprocesses and requires byte-identical
   stdout, byte-identical stderr and an identical exit status (code *and*
   termination signal). 61 tests, none `#[ignore]`d. Roughly 15 min wall clock
   on a many-core box; the expensive cases live in `mod full_run` so libtest
   runs them in parallel, and the two subprocesses inside each case are spawned
   concurrently.

   `argv[0]` is forced to the same value for both programs via
   `CommandExt::arg0`, because the C usage message prints `argv[0]` verbatim —
   without that the two binaries would differ only in their own path, which is
   not a translation defect.

2. **Reduced-scale oracles** (`scratch/`, development aids, not part of
   `cargo test`) — these reuse the *real* code from `translation/src/main.rs`
   (its `fn main` is renamed by `sed` and a harness `main` appended), so they
   cannot drift from what ships:
   - `check_oracle.sh` — glibc `rand()` stream (12 outputs × 20 seeds,
     including seeds above `2^31` where `(int32_t) seed` is negative) and the
     arithmetic kernel on 28 hostile `int32` values (`INT_MIN`, `INT_MAX`,
     `±127773`, `±1073741824`, …). **48/48 lines identical.**
   - `check_str.sh` — `strtoul(arg, &endptr, 10)` against glibc for 113
     byte-exact inputs, comparing the returned value, the `endptr` offset, the
     `ERANGE` flag *and* the accept/reject decision. **113/113 identical.**

3. **Extra end-to-end sweep** (`scratch/sweep.sh`) — 20 further seeds beyond
   those in the suite (3, 7, 8, 9, 10, 123, 4096, 65535, 127773, 16807,
   999999999, 1000000000, 1073741823, 1073741824, 2147483646, 2147483650,
   2863311530, 3141592653, 4294967293, 12345), full workload, C vs Rust.
   **20/20 identical.** Combined with the 14-input pre-suite batch, 34 extra
   full-runtime comparisons agree on top of the suite's own.

Nothing in `c_src/` was modified. Only `c_src/build/` was created, which is
what the build instructions call for; `c_src/src/main.c` and
`c_src/CMakeLists.txt` still carry their original checkout timestamps.

## Mismatches found

### 1. `SIGPIPE` was ignored in Rust but fatal in C — **real defect, fixed**

The only behavioural divergence found in the translation.

* **Symptom.** With nothing left reading stdout, the C program is killed by
  `SIGPIPE` when it finally calls `printf` (wait status 141, `code = None`,
  `signal = Some(13)`), while the Rust program exited 0.

      $ ./sp_c   | true   # C stand-in
      PIPESTATUS[0]=141
      $ ./sp_rs  | true   # Rust stand-in, before the fix
      PIPESTATUS[0]=0

* **Cause.** The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`
  runs. A C program starts with the default disposition, so the failing
  `write` terminates it instead of returning `EPIPE`. The translation ignored
  the `write!` result (correctly mirroring C's unchecked `printf`), so the
  ignored signal turned a fatal condition into a silent success.

* **Fix.** `restore_default_sigpipe()` in `translation/src/main.rs`, called as
  the first statement of `main`, sets `SIGPIPE` back to `SIG_DFL`. Verified
  with stand-ins: unfixed Rust → `(Some(0), None)`, fixed Rust → `(None,
  Some(13))`, C → `(None, Some(13))`. Normal runs (reader present) are
  unaffected and still exit 0.

* **Regression test.** `full_run::stdout_reader_gone_kills_both_the_same_way`.
  Only the accepting path writes to stdout, so this test pays a full workload
  run.

## Test-harness defects found (not translation defects)

Recorded because both initially showed up as red tests and could be
misread as translation problems.

### 2. A 1 MiB `argv[1]` cannot be exec'd at all

`invalid_seed_very_long_argument` first used `vec![b'7'; 1024 * 1024]` and
failed with `Argument list too long (os error 7)` — for *both* binaries. Linux
caps a single argv entry at `MAX_ARG_STRLEN` (128 KiB), so neither program can
be started, and the case measures the kernel rather than the translation.
Reduced to 100 000 digits, which still exercises `ERANGE` plus a very large
error message.

### 3. The `SIGPIPE` test leaked the pipe's read end into the child

The first version of the regression test above used `pipe(2)`, whose fds are
not close-on-exec. The child therefore *inherited* the read end, kept the pipe
alive, wrote successfully and exited 0 — so the test failed asserting that the
**C** program should have died from `SIGPIPE`. Switched to
`pipe2(fds, O_CLOEXEC)`; the write end survives because the spawn machinery
`dup2`s it onto fd 1 (which clears `O_CLOEXEC`), while the read end is closed
by `exec`.

## Behaviours deliberately replicated (C quirks that are *not* bugs to fix)

Each of these is asserted by the suite.

| C behaviour | Consequence | Where |
|---|---|---|
| `argc != 2` is checked *before* any parsing | `driver 42 42` prints usage; the valid seed is never looked at | `argc_extra_args_are_not_parsed_even_if_first_is_valid` |
| Usage message interpolates `argv[0]` with `%s` | Whatever `argv[0]` holds is echoed verbatim, including `%s%d%n`, tabs, newlines and invalid UTF-8 | `usage_message_echoes_argv0_verbatim` |
| With `argc == 0`, `argv[0]` is `NULL` | glibc renders it as the empty string, giving `"Usage:  <seed>\n"` (two spaces). Rust's empty `argv` produces the same bytes. | `raw_exec::argc_zero_via_raw_execve` (needs a raw `execve`; no shell can produce `argc == 0`) |
| Empty seed string is **accepted** | `strtoul("")` performs no conversion, so `endptr == nptr`, which points at the terminating NUL — the `*endptr != '\0'` guard passes and the seed is 0 | `full_run::seed_empty_string_is_accepted_as_zero` |
| `strtoul` skips leading whitespace | `" 5"`, `"\t5"`, `"\n+1"`, `"\x0b\x0c9"` are all valid seeds | `full_run::seed_*` |
| …but trailing whitespace is not | `"5 "`, `"5\n"`, `"-0 "` are rejected via `*endptr != '\0'` | `invalid_seed_trailing_whitespace` |
| `strtoul` negates modulo 2^64 | `"-1"` → `ULONG_MAX` → rejected for `> UINT_MAX`; but `"-18446744073709551615"` → **1**, an accepted seed | `invalid_seed_negative_values_wrap_above_uint_max`, `full_run::seed_negative_wraps_to_one` |
| Base is 10, not 0 | `"010"` is ten, not eight; `"08"`/`"09"` are valid; `"0x10"` is rejected at `x` | `full_run::seed_010_is_decimal_ten`, `invalid_seed_trailing_garbage` |
| `ERANGE` vs merely-too-large are distinct paths | `"18446744073709551615"` fits `unsigned long` (no `ERANGE`) yet fails `> UINT_MAX`; `"18446744073709551616"` sets `ERANGE` | `invalid_seed_ulong_range_but_above_uint_max`, `invalid_seed_erange_overflow` |
| glibc `srand(0)` substitutes seed 1 | `""`, `"0"`, `"-0"`, `"+0"`, `"1"` and `"-18446744073709551615"` all print the same value (42032659) | the seed 0 / seed 1 `full_run` cases |
| Signed wraparound in the kernel | `x * 3 + 7` overflows; replicated with `wrapping_mul`/`wrapping_add` (Rust would otherwise panic in debug and is UB in C, which gcc compiles as wraparound) | verified over `INT_MIN`/`INT_MAX` by `check_oracle.sh` |
| Arithmetic right shift, truncating division, C's `%` sign | `x ^ (x >> 3)`, `x / 2 + x % 7` on negatives; Rust's `>>` on `i32`, `/` and `%` already match | same |
| `(int32_t) seed` goes negative for seeds ≥ 2^31 inside glibc's `srand` | seeds `2147483648`, `3000000000`, `4294967295` take a different initialisation path | `full_run::seed_int32_max_plus_one` etc. |

## Analysis notes

* **`printf("%d")` with a negative value is unreachable.** The kernel's
  functional graph has its iterates land in cycles of length 52 330 and 11 991
  (tails observed ≈ 3 700–30 000 steps, far below the 200 000 steps the program
  performs). Every element of both cycles has bit 31 set — checked exhaustively
  over both cycles: `neg=52330 pos=0 zero=0` and `neg=11991 pos=0 zero=0`.
  `ARRAY_SIZE` is even, so the XOR clears the sign bit and `xor_result` comes
  out non-negative. This matches all 71 accepted-path runs observed (all
  positive) and is why no test asserts a negative `%d`. It is not a fidelity
  risk in any case: Rust's `{}` and C's `%d` agree on every `i32`, including
  `i32::MIN`.
* **`translation/src/glibc_rand.rs` and `translation/src/cstrtoul.rs` are dead
  code.** They are not declared as modules anywhere, so they are never
  compiled; `main.rs` carries its own copies. Left in place (they are not
  wrong), but note that only the `main.rs` copies are under test. The two
  `srand` variants differ in whether the `word < 0` correction is applied
  before or after truncation to `int32_t`; that is immaterial here because
  `hi` and `lo` always share the sign of `word`, so `16807 * lo - 2836 * hi`
  cannot leave the `int32_t` range.
* `panic = "abort"` is set for the release profile, so a Rust panic would abort
  (SIGABRT) rather than exit 1. No reachable panic path was found: all kernel
  arithmetic uses wrapping operations, and both `write!` results are discarded
  exactly as C discards `printf`/`fprintf` return values.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test --release
```

Fast subset only (skips every full-workload case, ~0.3 s):

```sh
cd translation && cargo test --release --test differential -- --skip full_run::
```
