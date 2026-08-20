# ERRORS.md — error-surface table (Phase C)

Derived mechanically from `c_src/src/main.c`. The program has **no error
codes, no error enums, no `assert`, no `return -1`/`return NULL` paths, and no
`errno` reporting** — `main` unconditionally `return 0`s. Every rejection it
performs is one of:

* a null check (`printLine`),
* the `fgets` failure branch,
* the single explicit range check `data < 100`,
* the fixed buffer bound `14` passed to `fgets` (silent truncation),
* undefined behavior that the C compiler/glibc turn into a **fatal signal**
  (the CWE this test case demonstrates: a negative length reaching `strncpy`).

Every grep-able rejection site in the file:

```
main.c:30   if(line != NULL)                       -> guard, no output when NULL
main.c:42   if (fgets(inputBuffer, 14, stdin) != NULL)  ... else -> failure branch
main.c:49   printLine("fgets() failed.");          -> the only diagnostic message
main.c:42   fgets(..., 14, ...)                    -> bound: max 13 bytes consumed
main.c:57   if (data < 100)                        -> range check, guards strncpy
main.c:59   strncpy(dest, source, data);           -> UB when data < 0  (size_t wrap)
main.c:60   dest[data] = '\0';                     -> UB when data < 0  (OOB store)
main.c:55   memset(source, 'A', 100-1)             -> constants 100, 99, 'A'
main.c:65   return 0;                              -> exit status is always 0
```

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `printLine` | `line == NULL` | no output at all, returns normally (guard at `main.c:30` is false) | [x] |
| E2 | `printLine` | `line` points at `""` (empty NUL-terminated string) | prints a single `"\n"` | [x] |
| E3 | `main` / `fgets` | stdin at EOF with **zero** bytes available (`< /dev/null`) → `fgets` returns `NULL` | `printLine("fgets() failed.")` runs, `data` stays `-1`, then row E6 fires: **SIGSEGV (exit 139)**. With stdout fully buffered (pipe/file) the message is *lost*; with stdout line buffered (tty) `"fgets() failed.\n"` **is** emitted before the fault | [x] |
| E4 | `main` / `fgets` | stdin file descriptor **closed** (`0<&-`) → `read` fails with `EBADF`, `fgets` returns `NULL` | same as E3: **SIGSEGV (exit 139)**, message lost when buffered | [x] |
| E5 | `main` / `atoi` | stdin holds a non-numeric string (`"abc"`, `"0x1F"`, `"+"`, `"-"`, `"."`, `"\n"`, embedded `NUL`) → `atoi` cannot parse, yields `0` — **silently, not an error** | `data = 0`, `strncpy(dest, source, 0)`, `dest[0] = '\0'`, prints `"\n"`, exit `0` | [x] |
| E6 | `main` / `strncpy` | `data < 0` (any negative parse, e.g. `"-1"`, `"-5"`, `"-2147483648"`, `"\t-5"`, or the `fgets`-failure default `-1`): `data < 100` is true and `(size_t)data` wraps to a huge length, so `strncpy`'s NUL-padding walks off the 100-byte stack buffer | fatal **SIGSEGV**, shell exit status **139**, no stdout when fully buffered | [x] |
| E7 | `main` | `data < 0` — the follow-up `dest[data] = '\0'` store at a negative index (OOB below the buffer) | unreachable in practice: the E6 fault happens first; must fault identically | [x] |
| E8 | `main` | `data >= 100` (`"100"`, `"999"`, `"2147483647"`, any 13-digit value that truncates into `>= 100`) — the `data < 100` check **rejects** the copy | `strncpy`/`dest[data]` skipped entirely, `dest` stays `""`, prints `"\n"`, exit `0` | [x] |
| E9 | `main` / `fgets` | input longer than the 14-byte buffer (`"1234567890123456789\n"`) — `fgets` silently truncates to 13 bytes and leaves the rest unread | `atoi` sees only the first 13 bytes; no error reported, exit `0` | [x] |
| E10 | `main` / `atoi` | value that does not fit in `int` but fits in `long` (13 digits max, e.g. `"4294967296"`, `"9999999999999"`) — `atoi` is `(int)strtol`, so the result is **truncated**, not rejected | truncated `int` (e.g. `4294967296` → `0`, `4294967301` → `5`); whether it then faults depends on the truncated sign | [x] |
| E11 | `main` / `atoi` | leading-whitespace-only or sign-only input after truncation (`"   "`, `"+"`, `"-"`) | `atoi` → `0`, prints `"\n"`, exit `0` | [x] |
| E12 | `main` | `data == 99` (largest value that still copies) — boundary one step below the `100` check | copies 99 `'A'`s, `dest[99] = '\0'`, prints 99 `'A'`s, exit `0` | [x] |
| E13 | `main` | `data == 100` — first value the range check rejects (one past the valid range) | prints `"\n"`, exit `0` | [x] |
| E14 | `main` | `data == 0` — degenerate zero length | `strncpy(dest, source, 0)` copies nothing, prints `"\n"`, exit `0` | [x] |

## Output-side failures (the C code never checks `printf`'s return value)

`printLine` ignores whatever `printf` returns, so a write failure is only
observable through the signal it raises — or through the absence of one. These
rows are the mirror of the input-side rejections above.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E15 | `main` / `printLine` | stdout is a pipe with **no reader**; the exit-time flush write gets `EPIPE` and the default `SIGPIPE` disposition kills the process | death by **SIGPIPE**, shell status **141**, `code = None`, `signal = Some(13)` | [x] |
| E16 | `main` / `printLine` | stdout **closed** (descriptor 1 not open); the write fails with `EBADF`, which nothing checks | no signal, exit **0**, no output | [x] |
| E17 | `main` / `printLine` | stdout on `/dev/full`; every write fails with `ENOSPC`, unchecked | no signal, exit **0**, no output | [x] |
| E18 | `main` / `fgets` | stdin is shared with another process: glibc consumes a whole `st_blksize` block, and `exit`'s cleanup rewinds a *seekable* stdin to the logical position while a signal death does not | seekable + normal exit → offset = `start + bytes consumed`; seekable + fault → offset = block end; pipe → `st_blksize` bytes consumed either way | [x] |

## Generic FFI boundary cases (required even though not in the C table)

| # | target | condition | expected | status |
|---|--------|-----------|----------|--------|
| G1 | `printLine` via `dlsym` | NULL pointer | no output, no crash (E1) | [x] |
| G2 | `printLine` via `dlsym` | zero-length string | one `"\n"` (E2) | [x] |
| G3 | `printLine` via `dlsym` | oversized payload (4 KiB, 64 KiB — past glibc's `BUFSIZ`/4096 stdout buffer, forcing an internal flush mid-string) | payload + `"\n"` | [x] |
| G4 | `printLine` via `dlsym` | non-UTF-8 / high-bit bytes (`0x80..0xFF`) — must **not** be validated or transcoded, unlike a Rust `str` | bytes passed through verbatim | [x] |
| G5 | `printLine` via `dlsym` | repeated calls (ordering + buffer accumulation across calls) | lines in call order | [x] |
| G6 | `main` | out-of-range "enum"-like values across the boundary: the full `int` domain reaching `data`, including `INT_MIN`, `INT_MAX`, `-1`, `0`, `99`, `100` | see E6/E8/E12/E13/E14 | [x] |

**Note on G6:** this C program declares no `enum` and takes no flags/modes, so
there is no out-of-range *enum* variant to pass. The equivalent "any int the C
accepts" surface is the parsed `data` value, which is swept exhaustively over
`-300..=300` and sampled randomly across the whole `i32` range in
`tests/differential_process.rs`.

All rows are exercised by `tests/differential_process.rs` (E3–E14, G6),
`tests/differential_ffi.rs` (E1, E2, G1–G5) and
`tests/differential_descriptors.rs` (E15–E18), each asserting the C and Rust
builds agree on stdout bytes, stderr bytes, **and** exit status / signal.

Rows **E15** and **E18** initially failed and exposed genuine bugs in the
translation (Rust ignoring `SIGPIPE`; `StdinLock`'s 8 KiB buffer with no
exit-time rewind). Both are fixed and pinned by mutation tests — see
"Divergences found and fixed" in `CONFIGS.md`.
