# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

Mechanically derived from `c_src/src/main.c` (61 lines, the whole program).
Exhaustive grep of the source for every rejection construct:

```
$ grep -nE 'return|assert|NULL|if|else|scanf|printf|<|>|==|!=' c_src/src/main.c
28:    if (line != NULL)          <- the ONLY explicit check in the program
30:        printf("%s\n", line);  <- return value discarded
36:    char *data;                <- read while uninitialised (UB) in bad()
50:    scanf("%d", &x);           <- return value discarded (1 / 0 / EOF)
52:    if (x)                     <- zero vs non-zero dispatch
60:    return 0;                  <- the only exit status
```

Findings that shape this table:

* There is **no** `RETURN_ERROR`-style macro, **no** `assert`, **no** error
  enum, **no** `return -1` / `return NULL`, and **no** named min/max constant
  in the source. `grep -c 'assert\|errno\|exit(' c_src/src/main.c` → `0`.
* There are **no enum types anywhere in the C source**, so the "out-of-range
  enum value across FFI" class has no instance here. The FFI surface consists
  of one `const char *` parameter and one `int` return; the pointer's invalid
  value (`NULL`) is row 1 and the `int` return is row 21.
* The program never *reports* an error: `scanf`'s status is thrown away and
  `printf`'s status is thrown away, so every "error" manifests only as a
  different branch/output. Each row therefore states the exact observable C
  result (stdout bytes + exit status), which is what the differential test
  asserts — not a vague "both failed".

`x` is pre-initialised to `0`, so **any** `scanf` failure ⇒ `x == 0` ⇒ `bad()`.
In the executable build `bad()` prints exactly `"\n"` (see row 22 and
`prog::BAD_DATA`).

## Table

Legend for "expected C result": `stdout` bytes, then exit status where relevant.
`E` = tested against the executable (`c_src/build/driver`), `S` = tested against
the shared library (`target/csrc/libcdriver.so`) through `libloading`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `printLine` | `line == NULL` — the `if (line != NULL)` guard at main.c:28 fails | writes **nothing** at all (0 bytes), returns void, no crash | S `err_printline_null` | [x] |
| 2 | `printLine` | `line` points to `""` (empty string, i.e. immediately NUL) — passes the NULL guard, `printf("%s\n","")` | exactly `"\n"` (1 byte) | S `err_printline_empty` | [x] |
| 3 | `printLine` | `line` contains printf conversion specifiers, e.g. `"%s %d %n %%"` — data, not format | the bytes verbatim + `"\n"` (no interpretation, no crash) | S `err_printline_format_specifiers` | [x] |
| 4 | `printLine` | `line` is not valid UTF-8 (e.g. `"\xff\xfe\x80"`) — `printf("%s")` is byte-oriented | the raw bytes verbatim + `"\n"` | S `err_printline_non_utf8` | [x] |
| 5 | `printLine` | `line` contains embedded `'\n'`/`'\r'`/`'\t'`/`'\x0b'` bytes | the bytes verbatim + one extra `"\n"` | S `err_printline_embedded_newline` | [x] |
| 6 | `printLine` | oversized `line` (64 KiB, far past any stdio buffer) | all 65536 bytes + `"\n"` | S `err_printline_oversized` | [x] |
| 7 | `printLine` | every single non-NUL byte value 1..=255 as a 1-byte string | that byte + `"\n"` (255 cases) | S `err_printline_every_byte` | [x] |
| 8 | `scanf("%d")` | **EOF before any character** — stdin empty / `/dev/null`. `scanf` returns `EOF`, stores nothing | `x` keeps `0` ⇒ `bad()` ⇒ `"\n"`, exit `0` | E `err_scanf_eof_empty` | [x] |
| 9 | `scanf("%d")` | **whitespace only, then EOF** (`" \t\n\v\f\r"`) — the `%d` skip loop hits EOF | `"\n"`, exit `0` | E `err_scanf_whitespace_only_eof` | [x] |
| 10 | `scanf("%d")` | **matching failure**: first non-space char is neither sign nor digit (`"abc"`, `"."`, `"x"`, `"/"`, `":"`, `"\x00"`) | nothing stored, `x == 0` ⇒ `"\n"`, exit `0` | E `err_scanf_matching_failure` | [x] |
| 11 | `scanf("%d")` | **sign then EOF** (`"-"`, `"+"`) — sign consumed, no digit available | matching failure, `x == 0` ⇒ `"\n"`, exit `0` | E `err_scanf_sign_then_eof` | [x] |
| 12 | `scanf("%d")` | **sign then non-digit** (`"-a"`, `"+."`, `"--1"`, `"++1"`, `"- 5"`, `"-\n5"`) | matching failure, `x == 0` ⇒ `"\n"`, exit `0` | E `err_scanf_sign_then_nondigit` | [x] |
| 13 | `scanf("%d")` | **positive overflow of the `long` accumulator**: `"9223372036854775808"` = `LONG_MAX+1`, up to 5000-digit inputs | glibc clamps to `LONG_MAX`, `%d` stores the low 32 bits ⇒ `-1` ⇒ non-zero ⇒ `"string\n"`, exit `0` | E `err_scanf_overflow_positive` | [x] |
| 14 | `scanf("%d")` | **negative overflow**: `"-9223372036854775809"` = `LONG_MIN-1` | clamps to `LONG_MIN`, low 32 bits ⇒ `0` ⇒ **zero** ⇒ `"\n"`, exit `0` | E `err_scanf_overflow_negative` | [x] |
| 15 | `scanf("%d")` | **value out of `int` range but in `long` range with low word 0**: `"4294967296"` (2³²), `"-4294967296"`, `"8589934592"`, `"2147483648"`+`"-2147483648"` | truncation to `int`, *not* saturation: `2³²`→`0`⇒`"\n"`, `2³¹`→`INT_MIN`⇒`"string\n"` | E `err_scanf_int_truncation` | [x] |
| 16 | `scanf("%d")` | **explicit `int` boundary values** `2147483647`, `-2147483648`, and one step past: `2147483648`, `-2147483649` | no rejection at all — `%d` never range-checks; all four are non-zero ⇒ `"string\n"` | E `err_scanf_int_boundaries` | [x] |
| 17 | `scanf("%d")` | `"0"`, `"-0"`, `"+0"`, `"0000…0"`, `"0x10"` (stops at `x`), `"0abc"` | value `0` ⇒ falsy ⇒ `bad()` ⇒ `"\n"`, exit `0` | E `err_scanf_zero_forms` | [x] |
| 18 | `scanf("%d")` | **stdin closed** (fd 0 not open) ⇒ `read` fails `EBADF` | `scanf` fails, nothing stored ⇒ `"\n"`, exit `0` (no crash, no diagnostic) | E `err_stdin_closed` | [x] |
| 19 | `printf` | **stdout closed** (fd 1 not open) ⇒ write fails `EBADF`; return value discarded at main.c:30 | no diagnostic on stderr, exit status still `0` | E `err_stdout_closed` | [x] |
| 20 | `printf` | **stdout is a full/closed pipe** — reader gone | process is killed by `SIGPIPE`/write fails identically for both builds | E `err_stdout_broken_pipe` | [x] |
| 21 | `main` | any input whatsoever, including all of the failures above | returns **`0` unconditionally** (`return 0;`, main.c:60) — there is no non-zero exit path | E+S `err_exit_status_always_zero` | [x] |
| 22 | `bad` | **reads the uninitialised `char *data`** (main.c:36) — undefined behaviour, the value is whatever the caller left in that stack slot | *caller-dependent, proven so*: **three different outcomes** measured from this one `main.c` — `"\n"` (executable), a run of the library's own machine code, and **`SIGSEGV`** (isolated call from the release-profile test binary). Deterministic per binary/caller, not a property of the source. See note below. | E `err_bad_uninitialised_via_exe` / S `err_bad_uninitialised_ub_documented` | [x] |

## Note on row 22 (the one genuinely undefined behaviour)

`bad()` is the only place where the C result is *not* a function of the source
text. Empirical proof — same `main.c`, same compiler, same flags, only the
caller's stack differs:

| how `bad()` is reached | C output |
|---|---|
| the `driver` **executable** (`add_executable`, the project's artifact), any input | `0a` = `"\n"` |
| `.so` + `dlopen`, called through **`main`** (C probe, and the Rust `so_runner`) | `0a` = `"\n"` |
| `.so` + `dlopen`, `bad` called **directly** from a small C probe | `03 0a` |
| `.so` + `dlopen`, `bad` called **directly** from the Rust `so_runner` | `55 48 89 e5 48 83 ec 10 c7 45 fc 0a` (the library's own code bytes) |
| `.so` + `dlopen`, `bad` called **directly** from the release-profile test binary | **killed by `SIGSEGV`** |
| `.so` + `dlopen`, `main` called 3× on `"--5"`, release-profile `so_runner` | `string\n` — the pointer the *preceding* `good()` call left in the very same stack slot |

Consequences, and how the suite is built around them:

* The behaviour **being reproduced** is the one every path the program itself
  takes agrees on: reached through `main`, in the executable *and* in the shared
  library, `bad()` prints exactly `"\n"`. `prog::BAD_DATA = b""` reproduces that
  byte-for-byte. Verified stable for every input tried — 5000-byte and 1 MB
  inputs, 3000+ randomized fuzz cases, both cargo profiles.
* Rows/tests that assert **byte equality** for the `bad()` path are the ones
  where the C is reproducible: `err_bad_uninitialised_via_exe`,
  `cfg_so_main_bad_path`, and every executable row whose input yields `x == 0`.
* The **isolated** `.so` call (`err_bad_uninitialised_ub_documented`,
  `cfg_bad_direct_ub_unspecified`) can only assert the Rust side (deterministic
  `"\n"`, clean exit) and *record* the C outcome, because a C result that
  changes — up to crashing — when the same source is merely re-linked is not a
  specification any translation could match. The call is made in a forked child
  so the crash cannot take the test process down. Asserting equality there would
  be asserting an artefact of one particular link.

## Result

- [x] All 22 rows have a passing differential test (`tests/errors.rs`, 25 tests).
- [x] Generic boundaries additionally covered: NULL pointer (row 1), zero length
      (row 2), oversized length (row 6), one step past the documented range
      (rows 13–16), closed/failing fds (rows 18–20), all 255 byte values
      (row 7), and the FFI surface itself
      (`ffi_boundary_surface_is_exactly_pointer_in_int_out`,
      `ffi_repeated_calls_have_no_hidden_state`). No enum exists in the C API
      (checked by that first test, not merely asserted in prose), so the
      invalid-enum class is empty and is documented rather than fabricated.

That last row is why no test may read a good/bad *branch sequence* off stdout
when a `bad()` call is in the mix: the undefined value can coincide exactly with
`good()`'s output. CONFIGS.md row 37 was rebuilt on the stream position instead.

### The divergences this verification actually found

Row 20 was not a formality. Rust's runtime installs `SIG_IGN` for `SIGPIPE`
before `main`, which no C program sees, so on a broken-pipe stdout:

| | before the fix | after the fix | C |
|---|---|---|---|
| exit code | `0` | `0` | `0` |
| signal | *none* | **13** | **13** |

`src/main.rs::restore_c_signal_dispositions` restores `SIG_DFL`. The fix is
deliberately confined to the executable: a shared-library build of `main.c` does
not touch the disposition either, so `src/lib.rs`'s exported `main` leaves
whatever the host installed alone.

Two further divergences were found on the input side and are documented with
their measurements in CONFIGS.md (rows 37 and 38): `scanf`'s **one character of
push-back**, and glibc's **`st_blksize` refill granularity plus the exit-time
rewind of a seekable stdin**. Both are fixed by `prog::CStdin` and
`prog::ByteSource::push_back`.
