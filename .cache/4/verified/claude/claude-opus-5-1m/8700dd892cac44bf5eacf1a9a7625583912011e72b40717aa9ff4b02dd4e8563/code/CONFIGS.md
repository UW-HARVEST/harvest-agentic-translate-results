# CONFIGS.md — configuration-surface table (Phase B)

## Build-time configuration

* `Cargo.toml` has **no `[features]` section** → exactly **one** feature
  combination exists. `--no-default-features` is therefore identical to the
  default build. Both are still run in `run_all.sh` for completeness.
* `c_src/CMakeLists.txt` has **no `option()`, no `add_definitions`, no
  `target_compile_definitions`, and no `#ifdef` anywhere in `main.c`** → the C
  side has exactly **one** build configuration too.
* Rust profiles do differ meaningfully (`[profile.release] panic = "abort"`,
  plus debug assertions / bounds checks in `dev`), so **both `debug` and
  `release`** Rust builds are compared against the C build.

## Runtime configuration axes (derived from the C source)

`main()` takes no arguments, reads no environment variables, and has no flags.
The axes it actually branches on are:

| axis | values the C code distinguishes | source |
|------|--------------------------------|--------|
| A. `fgets` outcome | non-NULL (≥1 byte read) / NULL (EOF or read error) | `main.c:42` |
| B. parsed `data` class | `< 0` → fault; `0`; `1..=98`; `99`; `100`; `> 100` | `main.c:57`, `59`, `60` |
| C. `atoi` input shape | digits / leading whitespace / `+`/`-` sign / non-numeric / digits+trailing junk / embedded NUL / `int` overflow-truncation | glibc `atoi` = `(int)strtol` |
| D. input length vs the 14-byte buffer | `< 13`, exactly 13, `> 13` (truncated), with / without trailing `'\n'` | `main.c:41-42` |
| E. stdout buffering discipline | line buffered (tty) / fully buffered (pipe, file) — only observable when the process faults before a flush | glibc `printf` + `main.c:32` |
| F. stdin kind | pipe / regular file / `/dev/null` / closed fd | `main.c:42` |
| G. entry point | `main` (whole program, process level) / `printLine` (lowest-level exported function, via `dlopen`) | `main.c:28`, `main.c:36` |
| H. stdout descriptor kind / write outcome | pipe with a live reader / pipe with **no** reader (`SIGPIPE`) / closed descriptor (`EBADF`) / `/dev/full` (`ENOSPC`) / regular file / pty — `printf`'s return value is never checked, so only the signal is observable | `main.c:32` |
| I. stdin descriptor sharing | seekable (offset left behind at exit) vs non-seekable (bytes consumed from the pipe) — glibc buffers `st_blksize` and rewinds seekable streams at `exit` | `main.c:42` |

## Configuration rows

Pruned cross-product — one row per combination the C code treats differently.
Every row is driven with **many randomized inputs** (fixed-seed LCG, seed
`0x5eed_1234`), not a single hand-picked value.

### Entry point `main` — full program, process level (`tests/differential_process.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `main` | A=non-NULL, B=`1..=98`, C=plain digits, D=short+`\n`, E=full, F=pipe — the nominal copy path; **exhaustive** `1..=98` plus randomized zero-padded spellings | [x] |
| C2 | `main` | B=`0` (`"0"`, `"000"`, `"0\n"`) — zero-length `strncpy` | [x] |
| C3 | `main` | B=`99` — largest copying value, `dest[99]` is the last in-bounds byte | [x] |
| C4 | `main` | B=`100` — first value rejected by `data < 100` | [x] |
| C5 | `main` | B=`> 100`, randomized in `101..=9_999_999` and `101..=i32::MAX` | [x] |
| C6 | `main` | B=`< 0`, randomized in `-1..=-300` and across `i32::MIN..0` → SIGSEGV path | [x] |
| C7 | `main` | B boundary sweep: **exhaustive** `-300..=300` spelled as decimal | [x] |
| C8 | `main` | B sampled: 512 random `i32` values (whole domain, incl. `i32::MIN`, `i32::MAX`) | [x] |
| C9 | `main` | C=leading whitespace (`' '`, `\t`, `\v`, `\f`, `\r`, mixed) before digits, randomized | [x] |
| C10 | `main` | C=explicit `+` sign, randomized values | [x] |
| C11 | `main` | C=explicit `-` sign, randomized values (fault path) | [x] |
| C12 | `main` | C=non-numeric garbage (`"abc"`, `"0x1F"`, `"."`, `"+"`, `"-"`, `"++5"`, `"- 5"`), randomized ASCII junk | [x] |
| C13 | `main` | C=digits followed by trailing junk (`"50abc"`, `"7 8"`, `"12.9"`), randomized | [x] |
| C14 | `main` | C=embedded NUL byte at various offsets (`"\0"`, `"5\0 9"`, `"\0 -1"`) | [x] |
| C15 | `main` | C=`int`-overflow truncation: values `> INT_MAX` but ≤ 13 digits (`"4294967296"`→0, `"4294967301"`→5, `"9999999999999"`), randomized 10–13 digit numbers | [x] |
| C16 | `main` | D=exactly 13 bytes (fills the buffer, no room for `'\n'`) | [x] |
| C17 | `main` | D=`> 13` bytes → silent truncation; randomized long digit strings and long junk | [x] |
| C18 | `main` | D=no trailing newline at all (`printf '7'`) | [x] |
| C19 | `main` | D=empty line only (`"\n"`) → `atoi("\n")` = 0 | [x] |
| C20 | `main` | A=NULL, F=`/dev/null` (immediate EOF) → `"fgets() failed."` + fault | [x] |
| C21 | `main` | A=NULL, F=closed stdin fd (`EBADF` read error) → same as C20 | [x] |
| C22 | `main` | F=regular file (not a pipe) as stdin, over a mix of the value classes | [x] |
| C23 | `main` | E=**line buffered / tty** (stdout on a pty) × A=NULL → the diagnostic *is* flushed before the fault (opposite of C20) | [x] |
| C24 | `main` | E=**line buffered / tty** × B=`1..=98`, `0`, `100` → normal output over a pty | [x] |
| C25 | `main` | E=fully buffered to a **regular file** (not a pipe) × mix of value classes | [x] |
| C26 | `main` | multi-line stdin (only the first line is consumed; the rest must be ignored) | [x] |
| C27 | `main` | random raw byte lines (any of `0x00..=0xFF`, length 0..40) — 1024 fuzz cases | [x] |
| C28 | `main` | H=stdout on a pipe with a **live** reader — the baseline every row above uses | [x] |

### Entry point `main` — descriptor-level behavior (`tests/differential_descriptors.rs`)

These rows were **added after** the first pass: axes H and I are invisible to
stdout comparison, and both initially diverged. See "Divergences found" below.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C29 | `main` | H=stdout is a pipe with **no reader** × value classes `0`/`1`/`50`/`99`/`100`/junk → the exit-time flush fails and raises `SIGPIPE` | [x] |
| C30 | `main` | H=stdout **closed** (`EBADF`) × several value classes | [x] |
| C31 | `main` | H=stdout on `/dev/full` (`ENOSPC`) × several value classes | [x] |
| C32 | `main` | I=**seekable** stdin shared with the parent × {file longer than one block, file shorter than one block, empty file} × {normal exit, fault} × {offset 0, pre-seeked offset} — checks where the offset is left | [x] |
| C33 | `main` | I=**pipe** stdin shared with the parent × {payload longer than one block, payload shorter} × {normal exit, fault} — checks how many bytes are consumed | [x] |

### Entry point `printLine` — lowest-level exported function, via `dlopen` (`tests/differential_ffi.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| F1 | `printLine` | NULL argument (the guard's false branch) | [x] |
| F2 | `printLine` | empty string `""` | [x] |
| F3 | `printLine` | short ASCII strings, randomized | [x] |
| F4 | `printLine` | string already containing `'\n'` (interacts with line buffering) | [x] |
| F5 | `printLine` | high-bit / non-UTF-8 bytes (`0x80..=0xFF`), randomized | [x] |
| F6 | `printLine` | payload larger than glibc's 4096-byte stdout buffer (4 KiB, 64 KiB) | [x] |
| F7 | `printLine` | many sequential calls — ordering and cross-call buffer accumulation | [x] |
| F8 | `printLine` | 256 randomized byte payloads, length 0..300 | [x] |

## Coverage note

Rows C1–C28 drive the program end to end exactly as a shell user would; rows
C29–C33 cover what the program does to the descriptors it was handed; rows
F1–F8 drive the lowest-level exported function directly through `dlopen`/
`dlsym` on **both** shared objects, so the composed pipeline and the individual
primitive are both covered.

## Divergences found and fixed

Rows C1–C28 and F1–F8 passed against the original translation. The two
descriptor axes did **not** — both are cases where Rust's runtime silently
differs from a C program:

1. **C29 — `SIGPIPE`.** Rust's runtime sets `SIGPIPE` to `SIG_IGN` before
   `main` runs, so the failed flush was ignored and the process exited `0`
   where the C program dies from signal 13 (status 141). Fixed by restoring
   `SIG_DFL` at the start of `main` (`src/main.rs`).
2. **C32/C33 — stdin buffering and rewind.** `StdinLock`'s 8 KiB buffer
   consumed 8192 bytes and never rewound, while glibc reads exactly one
   `st_blksize` block (4096) and, at `exit`, seeks a *seekable* stdin back to
   the stream's logical position. Both the read size and the exit-time rewind
   are now reproduced (`CStdin` in `src/lib.rs`); note the rewind correctly
   does **not** happen when the process dies from a signal.

Each fix is pinned by a mutation test: reverting it makes the corresponding row
fail again (12 mutants injected, 12 detected).
