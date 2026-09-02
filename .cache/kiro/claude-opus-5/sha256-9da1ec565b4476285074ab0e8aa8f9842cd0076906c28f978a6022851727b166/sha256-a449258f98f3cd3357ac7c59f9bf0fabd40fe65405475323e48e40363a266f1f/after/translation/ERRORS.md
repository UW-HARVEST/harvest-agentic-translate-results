# Differential verification record

C reference: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust translation: `translation/src/main.rs`, built to `translation/target/{debug,release}/driver`.

Both programs are compared by execution only (`translation/tests/differential.rs`
spawns each binary, writes the same bytes to stdin, and compares stdout, stderr
and exit status). Nothing loads the translation as a library.

Commands that run each program:

```
c_src/build/driver                          # C
translation/target/release/driver           # Rust
```

## What the C program branches on

```c
int x = 0;
scanf("%d", &x);
if (x) good(); else bad();
```

`scanf`'s return value is discarded, so a failed conversion leaves `x` at its
initialiser `0`. That gives exactly three input classes:

1. no conversion (EOF or matching failure) -> `x == 0` -> `bad()`
2. conversion succeeds with value `0` -> `bad()`
3. conversion succeeds with a non-zero value -> `good()`

Both `bad()` and `good()` copy a zero-filled `int source[10]` into an
`alloca`-backed buffer and print `data[0]`, so every path prints `0\n` and
returns 0. The observable behaviour is therefore identical across all three
classes; the branch is only detectable through instrumentation, not output.

`bad()` calls `alloca(10)` but stores ten 4-byte ints through the pointer,
writing 30 bytes past the request. This is the original defect. It is preserved
in the translation (`StackFrame` models a byte-addressed frame region so the
out-of-bounds stores land in neighbouring frame bytes instead of panicking), and
in practice neither program crashes or produces different output because of it.

## Phase A — compile errors

None. `cargo build --release` succeeded on the first attempt; no fixes were
needed before testing began.

## Mismatches found

### 1. SIGPIPE disposition changed the exit status (fixed)

**Symptom.** With stdout connected to a pipe that had no reader, the C program
was killed by `SIGPIPE` (signal 13, shell status 141) while the Rust program
exited 0.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. The C program does nothing to `SIGPIPE`, so it uses whatever disposition
it inherited across `exec`. Under `SIG_DFL` the failing `write` from stdout's
exit-time flush kills the C process; the Rust process instead saw `EPIPE`, which
the translation ignores (as does the C code, which never checks `printf`'s
return value), and exited 0.

**First attempt, which was wrong.** Unconditionally calling
`signal(SIGPIPE, SIG_DFL)` at the top of `main`. This over-corrected: the
disposition C uses is the *inherited* one, and it is not always `SIG_DFL`.
`SIG_IGN` is inherited across `exec`, so when the parent ignores `SIGPIPE` --
which is the case for any parent that is itself a Rust program, including
`cargo test`'s own harness -- the C program inherits `SIG_IGN` and exits 0.
Forcing `SIG_DFL` made the Rust program die with signal 13 where the C program
exited 0, i.e. the same mismatch with the sign flipped.

**Fix.** `translation/src/main.rs` now records the inherited disposition in an
`.init_array` constructor, which the loader runs before `main` and therefore
before the standard library's initialisation replaces it, and restores that
recorded value as the first action in `main`. The translation then matches the C
program under either inherited setting.

**Verified.** `readerless_stdout_with_default_sigpipe` and
`readerless_stdout_with_ignored_sigpipe` set the disposition in the forked child
before `exec`, so both binaries start from an identical inherited value.
Independently confirmed outside the Rust harness: under `SIG_DFL` both processes
report signal 13, under `SIG_IGN` both report exit 0.

## Test-harness defect found while verifying (not a translation mismatch)

The first version of the readerless-stdout test used `Stdio::piped()` for stdout
and dropped the read end after spawning. It passed serially but failed
intermittently under `cargo test`'s default parallelism, reporting the C program
as exiting 0. Cause: the read-end descriptor was transiently held open by a
child process spawned concurrently by another test, so the C program's write
into the pipe buffer succeeded and no `SIGPIPE` was raised. The test now uses a
per-invocation FIFO whose read end is opened and closed by the test process
alone; `open` sets `O_CLOEXEC`, so no other child can inherit it. Stable over
repeated parallel runs of the whole suite in both debug and release profiles.

## Input classes checked, all matching

Every case below was compared on stdout, stderr and exit status. All matched.

| Class | Inputs |
| --- | --- |
| empty / EOF at once | `""`, stdin closed, stdin from `/dev/null`, stdin a directory |
| whitespace only | `"   \n\t \n"`, `"\n\n"`, `" "`, `"\x0b\x0c"` |
| matching failure, non-numeric | `"abc"`, `"  abc"`, `".5"`, `","`, `"#1"` |
| matching failure, sign only | `"-"`, `"+"`, `"-a"`, `"+\n"`, `"--1"` |
| converts to zero (`bad()`) | `"0"`, `"-0"`, `"+0"`, `"0\n"`, nineteen `0`s, `"0x10"`, `"0\n1\n"`, `"4294967296"`, `"-4294967296"`, `"-99999999999999999999"` |
| converts to non-zero (`good()`) | `"1"`, `"-1"`, `"+7"`, `"007"`, `"   \n 5"`, `"\t9"`, `"1abc"`, `"42 99"` |
| integer limits and overflow | `INT_MAX`, `INT_MIN`, `INT_MAX+1`, `INT_MIN-1`, `2^32+1`, twenty `9`s, `LONG_MAX`, `LONG_MAX+1`, `LONG_MIN` |
| binary and NUL bytes | `"\x00"`, `"\x005"`, `"\xff\xfe"`, `"\x00\x01\xff\xfe1"`, `"\r\n3"` |
| newline termination | `"7"`, `"7\n"`, `"7\r\n"` |
| larger than the pipe buffer | 100k `9`s, 100k `0`s, `"1"` + 200k unread bytes, 100k spaces then `"4"` |
| swept decimal values | 21 boundary values x {bare, newline-terminated, space-padded}, plus values whose low 32 bits are zero |
| readerless stdout | SIGPIPE inherited as `SIG_DFL` and as `SIG_IGN` |

Notable `scanf("%d")` behaviours that the translation reproduces, each covered
above:

- whitespace skipping crosses newlines, unlike `fgets`
- a sign with no digit after it is a matching failure, and the destination is
  left untouched rather than zeroed
- `0x10` converts as `0` and leaves `x10` in the stream
- glibc runs the digits through `strtol`, saturating at `LONG_MAX` / `LONG_MIN`,
  and then narrows to `int`: twenty `9`s yields `-1`, twenty negative `9`s
  yields `0`, and `4294967296` truncates to `0` (which flips `main`'s branch)
- the return value is discarded, so EOF and matching failure are
  indistinguishable from a successful conversion of `0`

## Additional sweep beyond the test suite

An out-of-band differential run compared 4,491 further inputs: all strings of
length 1-3 over `{0, -, +, space, 1, \n, a}` exhaustively, 4,000 random strings
of length 0-24 over a digit / sign / whitespace / letter / NUL / `0xff`
alphabet, and values within +-3 of `0`, `2^31`, `2^32`, `2^63`, `2^64` and
`10^19` in both signs. Zero mismatches.

## Status

- both programs build with no errors
- `cargo test` passes in `translation/` in both debug and release
- no test is disabled, skipped or `#[ignore]`d
- `c_src/` sources are unmodified; only the generated `c_src/build/` directory
  was added
