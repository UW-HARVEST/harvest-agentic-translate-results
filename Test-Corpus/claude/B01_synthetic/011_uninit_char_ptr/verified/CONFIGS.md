# CONFIGS.md — configuration surface table (Phase A, gates Phase B)

## Build-time configuration axes: exactly one combination

| source | axes found | combinations |
|---|---|---|
| `Cargo.toml` `[features]` | **none declared** (the section is empty) | 1 — the default/empty feature set |
| `c_src/CMakeLists.txt` | `cmake_minimum_required` + `project` + `add_executable`. No `option()`, no `add_definitions`, no `target_compile_definitions`, no `CMAKE_BUILD_TYPE` | 1 |
| `c_src/src/main.c` | `grep -c '#if\|#ifdef\|#ifndef\|#define' c_src/src/main.c` → **0**. The only preprocessor line is `#include <stdio.h>` | 1 |

So the *whole* cross-product of build configurations is:

| # | cargo invocation | note |
|---|---|---|
| 1 | `cargo check/test --no-default-features` | there are no features, so this is the empty set |
| 2 | `cargo check/test` (default) | identical to #1 — `default` is empty |
| 3 | `cargo check/test --all-features` | identical to #1 — no features exist |

All three are verified by `./verify.sh`, plus both cargo profiles
(`dev` and `release`, the latter carrying `panic = "abort"`) and both Rust
artifacts (`[[bin]] driver` and the `[lib]` cdylib).

## Runtime configuration axes the C code actually branches on

Derived from the source, not guessed:

* **A1 — `printLine`'s NULL guard** (`main.c:28 if (line != NULL)`): `NULL` vs
  non-`NULL`. This is the only `if` outside `main`.
* **A2 — `main`'s zero test** (`main.c:52 if (x)`): `x == 0` → `bad()`,
  `x != 0` → `good()`.
* **A3 — `scanf("%d")` outcome** (`main.c:50`): success (1) / matching failure
  (0) / EOF (-1). On the latter two, `x` keeps its initialiser `0`, so A3 feeds
  A2.
* **A4 — `%d` input shape**, i.e. the sub-axes glibc's converter itself branches
  on: leading-whitespace run (kind × length), sign (absent / `+` / `-`), digit
  count, magnitude class (fits `int` / fits `long` only / overflows `long` →
  clamp), the low-32-bit word of the accumulated `long` (zero vs non-zero, which
  decides A2), and the terminator (EOF / whitespace / non-digit / more input).
* **A5 — stream plumbing**: what fd 0 is (pipe / regular file / `/dev/null` /
  closed) and what fd 1 is (pipe / regular file / `/dev/null` / closed / broken
  pipe). `printf`/`scanf` buffering behaviour changes with these.
* **A6 — entry point**: the four exported symbols `printLine`, `good`, `bad`,
  `main` — the lowest-level ones (`printLine`) are driven directly through the
  `.so`, not only through the `main` one-shot wrapper.
* **A7 — stream state that outlives a call**: `scanf` runs on libc's
  process-global `stdin` `FILE`, so the number of times `main` is invoked and
  what the previous conversion pushed back / buffered are configuration axes in
  their own right. Two of the three divergences this verification found live
  here; see the note at the end.

## Table — one row per combination the C treats differently

`E` = executable-vs-executable (`c_src/build/driver` vs `target/*/driver`,
subprocess, the artifact the C project builds).
`S` = `.so`-vs-`.so` through `libloading` (`target/csrc/libcdriver.so` vs
`target/*/libdriver.so`), calling the exported symbol directly.
"rand N" = N property-style randomized inputs from a fixed-seed PRNG
(`tests/common/mod.rs::Rng`, seed noted in the test).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `printLine` (S) | A1 non-NULL, length 0 (`""`) | `cfg_printline_empty` | [x] |
| 2 | `printLine` (S) | A1 non-NULL, length 1, **every** byte value `1..=255` (255 cases) | `cfg_printline_single_byte_all_values` | [x] |
| 3 | `printLine` (S) | A1 non-NULL, printable-ASCII payload, random length 1..=64, rand 1000 | `cfg_printline_random_ascii` | [x] |
| 4 | `printLine` (S) | A1 non-NULL, arbitrary non-NUL bytes (incl. ≥0x80, invalid UTF-8), random length 1..=256, rand 1000 | `cfg_printline_random_bytes` | [x] |
| 5 | `printLine` (S) | A1 non-NULL, payload containing embedded `\n \r \t \v \f` runs | `cfg_printline_embedded_whitespace` | [x] |
| 6 | `printLine` (S) | A1 non-NULL, payload that is a printf format string (`%s %d %n %% %999999d`) — data, never a format | `cfg_printline_format_specifiers` | [x] |
| 7 | `printLine` (S) | A1 non-NULL, sizes straddling stdio buffering: 1023, 1024, 4095, 4096, 8191, 8192, 65536, 1 MiB | `cfg_printline_buffer_boundaries` | [x] |
| 8 | `printLine` (S) | A1 non-NULL, called repeatedly (100×) — no cross-call state, outputs concatenate | `cfg_printline_repeated_calls` | [x] |
| 9 | `good` (S) | fixed payload `"string"`, single call and 100 repeated calls | `cfg_good_direct` | [x] |
| 10 | `bad` (S) | A1 non-NULL (uninitialised-pointer UB), single + repeated calls. The C has no defined behaviour here, so only the Rust side is pinned (`"\n"` per call, clean exit) and the C outcome is recorded — see the note at the end and ERRORS.md row 22 | `cfg_bad_direct_ub_unspecified` | [x] |
| 11 | `main` (S) | A3 success × A2 `x != 0`, via `dlopen`+call in a hermetic subprocess, values `1, -1, 7, 2147483647, -2147483648` | `cfg_so_main_good_path` | [x] |
| 12 | `main` (S) | A3 success/failure × A2 `x == 0` (`"0"`, `"abc"`, `""`) — `bad()` UB invariant through the `.so` | `cfg_so_main_bad_path` | [x] |
| 13 | `main` (E) | A4 no whitespace, no sign, single digit — all of `0..=9` | `cfg_exe_single_digit` | [x] |
| 14 | `main` (E) | A4 leading whitespace: each of `' ' \t \n \v \f \r` × run length {1, 3, 1000} × {value `0`, value `5`}, plus 100 randomized mixed-whitespace runs | `cfg_exe_leading_whitespace_kinds` | [x] |
| 14b | `main` (E) | A4 **exhaustive byte classification**: for all 256 byte values, `[b,'5']`, `['-',b,'5']`, `['1',b,'2']` — pins down which bytes are whitespace / sign / digit / terminator (768 cases) | `cfg_exe_byte_classification_exhaustive` | [x] |
| 14c | `main` (E) | A4 exhaustive sweep where the byte interacts with the sign: `[b,'-','1']`, `[b,'+','1']`, `[b,b,'7']` for all 256 values (768 cases) | `cfg_exe_byte_classification_sign_and_runs` | [x] |
| 15 | `main` (E) | A4 sign axis: `""` / `"+"` / `"-"` × magnitude {0, 1, 9, 2147483647} | `cfg_exe_sign_matrix` | [x] |
| 16 | `main` (E) | A4 digit-count axis 1,2,3,9,10,11,18,19,20,21,39,64,1000,5000 random digit strings, rand 20 each | `cfg_exe_digit_counts` | [x] |
| 17 | `main` (E) | A4 magnitude class "fits `int`": uniformly random `i32` incl. `INT_MIN`/`INT_MAX`, rand 1500 | `cfg_exe_random_i32` | [x] |
| 18 | `main` (E) | A4 magnitude class "fits `long`, not `int`": random `i64` outside `i32` range, rand 1000 | `cfg_exe_random_i64_beyond_i32` | [x] |
| 19 | `main` (E) | A4 magnitude class "overflows `long`" → glibc clamp: random 20..40-digit strings, both signs, rand 1000 | `cfg_exe_random_long_overflow` | [x] |
| 20 | `main` (E) | A4 low-32-word == 0 with non-zero `long` (⇒ A2 takes `bad()`): `k * 2^32` for random `k`, rand 500 | `cfg_exe_low_word_zero` | [x] |
| 21 | `main` (E) | A4 low-32-word != 0 for values > `INT_MAX` (⇒ A2 takes `good()`): random `k*2^32 + r`, rand 500 | `cfg_exe_low_word_nonzero` | [x] |
| 22 | `main` (E) | A4 exact boundary constants: `0, ±1, 2147483647, 2147483648, -2147483648, -2147483649, 4294967295, 4294967296, 4294967297, 8589934592, 9223372036854775807, 9223372036854775808, -9223372036854775808, -9223372036854775809, 18446744073709551615, 18446744073709551616` | `cfg_exe_boundary_constants` | [x] |
| 23 | `main` (E) | A4 leading zeros: `0`,`00`,`0…0` (5000 zeros), `007`, `-007`, `0…04294967296` | `cfg_exe_leading_zeros` | [x] |
| 24 | `main` (E) | A4 terminator axis: value followed by EOF / `\n` / `\r\n` / `' '` / `'a'` / `'.'` / `'-'` / a second number, rand 300 | `cfg_exe_terminators` | [x] |
| 25 | `main` (E) | A4 multiple whitespace-separated numbers — only the first is converted | `cfg_exe_multiple_numbers` | [x] |
| 26 | `main` (E) | A5 fd 0 = **pipe** (the default in every other row) × both A2 branches | covered by all `E` rows | [x] |
| 27 | `main` (E) | A5 fd 0 = **regular file** × both A2 branches | `cfg_exe_stdin_regular_file` | [x] |
| 28 | `main` (E) | A5 fd 0 = **`/dev/null`** (immediate EOF) | `cfg_exe_stdin_devnull` | [x] |
| 29 | `main` (E) | A5 fd 1 = **regular file** × both A2 branches | `cfg_exe_stdout_regular_file` | [x] |
| 30 | `main` (E) | A5 fd 1 = **`/dev/null`** × both A2 branches | `cfg_exe_stdout_devnull` | [x] |
| 31 | `main` (E) | A4 fully arbitrary bytes (incl. NUL, 0x80..0xff, control chars), random length 0..=32, rand 2000 — the unconstrained fuzz | `cfg_exe_fuzz_arbitrary_bytes` | [x] |
| 32 | `main` (E) | A4 "numeric-ish" grammar fuzz: random mixes of whitespace / signs / digits / letters / dots, rand 2000 | `cfg_exe_fuzz_numericish` | [x] |
| 33 | `main` (E) | A5 exit status **and terminating signal** compared on every row above (always `0` / none) | asserted inside every `E` helper + `cfg_exit_status_invariant` | [x] |
| 34 | `main` (E) | A4 multi-megabyte stdin via a regular file: 1 MB of `9`s / `0`s / spaces+`7` / letters / `-`+1 MB of `9`s | `cfg_exe_multi_megabyte_inputs` | [x] |
| 35 | `main` (E) | A5 process environment: 4 argv sets × 6 locale environments (`LC_ALL`/`LANG`/`LC_NUMERIC`/`LC_CTYPE`, incl. `tr_TR.UTF-8` and `de_DE.UTF-8`) × 6 inputs — `main.c` never calls `setlocale`, so the "C" locale must hold regardless | `cfg_exe_argv_and_env_invariance` | [x] |
| 36 | `printLine` (S) | A1 non-NULL payload containing an **embedded NUL**: `printf("%s")` stops there, so the tail must be invisible. 8 fixed + 200 randomized placements | `cfg_printline_embedded_nul` | [x] |
| 37 | `main` (S) | A7 the export called **1/2/3/5 times on one stream**, compared by the **stream position** each library leaves on a shared seekable fd 0 (UB-free: `bad()`'s output is never observed). 24 fixed + 250 randomized over `0-9 + - . x SP TAB NL` | `cfg_so_main_repeated_calls_share_the_stream` | [x] |
| 37b | `main` (S) | A7 repeated calls compared **byte-for-byte on stdout**, restricted to inputs where every call converts a non-zero value so `bad()` never runs and the output is fully defined. 9 fixed + 120 randomized | `cfg_so_main_repeated_calls_all_good_output` | [x] |
| 38 | `main` (E) | A5/A7 what is left on a **shared fd 0** after exit, for a seekable file *and* a pipe: 8 fixed shapes, the 4095/4096/4097/8191/8192/50000 refill boundaries, and 120 randomized | `cfg_exe_shared_stdin_leftover` | [x] |

## The divergences these rows actually found

Rows 37 and 38 are not formalities — they are the rows that failed first, and the
reason the axes above include stream state:

| # | what the C does | what the untreated translation did | fix |
|---|---|---|---|
| 37 | `scanf` pushes **exactly one character back** (`ungetc`): the terminator on success, the offending character on a matching failure, without restoring a consumed sign. Measured via `so_runner <lib> main 3`: `"--5"` → bad/good/bad, `"-a5"` → bad/bad/bad, `"5x7"` → good/bad/bad | consumed the terminator, so `"1-9223372036854775809"` × 2 gave good/**good** where the C gives good/**bad** (its second conversion sees the `-`, clamps to `LONG_MIN`, truncates to 0) | `prog::ByteSource::push_back` + `prog::CStdin`'s `ungetc` slot |
| 37 | libc's `stdin` is **one process-global `FILE`**, shared by every call | a fresh reader per call | `stdin_state()` in `src/lib.rs` |
| 38 | refills stdin in **`st_blksize`** (4096) chunks, and libc's exit-time cleanup **rewinds a seekable fd** to the logical position | `std::io::stdin()`'s 8192-byte buffer and no rewind: on a 100 002-byte pipe the C leaves 95 906 bytes and Rust left 91 810; on `"1 hello world"` from a file `{ ./driver; cat; }` printed `" hello world"` for C and nothing for Rust | `prog::CStdin` + `reposition_if_seekable()` |

## Note — what this program can and cannot distinguish

Two things constrain what may be asserted, and both were learned the hard way.

**First**, `bad()`'s output is undefined and *caller*-dependent, so it must never
appear in a byte comparison that is supposed to prove something about the
translation. Row 37 originally compared the good/bad branch sequence read off
stdout; it passed in the `dev` profile and failed in `release`, because with a
release-profile `so_runner` the C's third call printed `"string"` — the pointer
the preceding `good()` had left in the very stack slot `bad()` reads. The row was
rebuilt on the **stream position** instead (fully defined), with a separate
all-`good()` row for the byte-exact stdout claim. The remaining `.so` rows that
do compare a `bad()` line byte-for-byte (`cfg_so_main_bad_path`,
`assert_so_main_same`) hold in both profiles because a *single* call has no
preceding `good()` to leave that pointer behind; the authoritative bad-path
assertions are the executable ones, which are exact for every input.

**Second**, only the push-back *count* is observable, never the pushed-back
byte's *identity*: on a seekable fd the exit-time rewind hands the byte back to
the file, so a correct implementation re-reads exactly that byte, and every path
where the identity could differ is a matching failure — i.e. a `bad()` path whose
C output is undefined. `mutation_check.sh` records this: an injected "push back
the sign instead of the offending character" bug is undetectable through any
defined channel, and was replaced with an off-by-one rewind bug, which *is*
caught.

**Third**, `main` reduces everything it reads to a single bit:
`if (x) good(); else bad();`.
The **only** observable consequence of the whole `%d` conversion is therefore
whether `x` is zero. A translation bug that changes the *value* of `x` without
changing its zero-ness is invisible to any observer of the C program, including
this suite — which is why rows 20, 21, 22 and 23 (the shapes that flip
zero-ness: `k·2³²`, `k·2³²+r`, `LONG_MIN` truncation, leading zeros) carry the
real weight, and why `mutation_check.sh` targets them. This was confirmed
empirically: the injected "stop after the first digit" bug is invisible to
random-`i32` inputs and is caught only by the leading-zero row.

## Result

- [x] Every row passes across its randomized inputs, under every build
      configuration from the first table (verified by `./verify.sh`).
- [x] `./mutation_check.sh` confirms the suite detects all 22 injected bugs, so
      the green runs above are meaningful rather than vacuous.
