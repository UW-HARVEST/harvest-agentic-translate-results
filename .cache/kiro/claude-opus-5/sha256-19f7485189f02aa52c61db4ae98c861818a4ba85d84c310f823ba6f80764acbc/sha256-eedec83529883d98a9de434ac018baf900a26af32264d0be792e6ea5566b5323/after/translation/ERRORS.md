# Differential verification of `c_src/src/main.c` vs `translation/src/main.rs`

## Result

**No mismatches were found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr and an identical exit status
(including termination by signal) from both programs.

The Rust translation already reproduced each of the non-obvious C behaviors
listed under "Divergence candidates" below. Because nothing diverged, no change
was made to `translation/src/main.rs`; the work in this pass was building both
programs, enumerating the branch space, and building the differential suite in
`translation/tests/differential.rs` (34 tests).

## How it was verified

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  -> `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  -> `translation/target/release/driver`
- Comparison: both binaries are spawned as subprocesses, fed the same bytes on
  stdin, and stdout / stderr / exit status are compared. The Rust code is never
  linked as a library.
- Beyond the committed tests, roughly 3,000 further inputs were compared
  ad hoc: random binary blobs of 0-30 bytes, a numeric sweep from -3000 to
  3000, and every integer from 0 to 120. Zero mismatches.

## What the C program branches on

`main()` contains only three decisions, and `data` (an `int`) drives two of them:

1. `fgets(inputBuffer, 14, stdin) != NULL` - success, or the
   `"fgets() failed."` path with `data` left at its initializer `-1`.
2. `data < 100` - `strncpy(dest, source, data); dest[data] = '\0';`, or the
   copy skipped entirely so `dest` keeps its all-zero initializer.
3. `printLine`'s `line != NULL` - unreachable as false; both call sites in
   `main()` pass non-null buffers.

Resulting input classes, all covered:

| Class | Example input | Observable result |
|---|---|---|
| EOF before any byte | empty stdin, `/dev/null`, closed fd 0 | `data == -1`, killed by SIGSEGV, stdout empty |
| `data == 0` | `0`, `\n`, `abc`, `   ` | one `\n` |
| `0 < data < 100` | `5`, `98` | `data` `A`s then `\n` |
| `data == 99` (max in range) | `99` | 99 `A`s then `\n` |
| `data >= 100` | `100`, `1000`, `2147483647` | copy skipped, one `\n` |
| `data < 0` | `-1`, `-100`, `-2147483648` | killed by SIGSEGV, stdout empty |
| `atoi` truncation to `int` | `4294967296` -> 0, `4294967396` -> 100, `2147483648` -> `INT_MIN` | follows the wrap |
| `fgets` 13-byte limit | `999999999999999` -> `9999999999999` | 14th byte onward dropped |
| `fgets` stops at newline | `7\n50\n` | second line never read |

## Divergence candidates that were checked and already correct

These are the places a translation would most plausibly have gone wrong. Each
was confirmed to match; none required a fix.

1. **Negative length passed to `strncpy` must crash, not clamp.**
   `data` is a signed `int` implicitly converted to `size_t`, so `data == -1`
   becomes `SIZE_MAX`. glibc copies the 99 `A`s from `source` and then pads with
   NUL bytes until it walks off the 100-byte stack buffer. Both programs are
   killed by SIGSEGV (signal 11, shell status 139). A translation that clamped
   the count, saturated it, or panicked would have shown up here: a Rust panic
   exits 101 and writes to stderr, whereas the C writes nothing to stderr.

2. **`"fgets() failed."` is printed but never observed when stdout is a pipe.**
   glibc fully buffers stdout when it is not a terminal, so the message is still
   in the buffer when SIGSEGV kills the process, and is lost. The C program's
   stdout is therefore *empty* on this path despite the `printf`. The
   translation reproduces this by buffering and deliberately not flushing before
   the fault.

3. **The same path *does* emit the message when stdout is a terminal.**
   Under line buffering the `printf` reaches the fd before the crash. Driving
   both binaries through a pty (`script -q -c ... /dev/null`) confirms both
   print `fgets() failed.\r\n` and then die. This is the one case that
   distinguishes a correct buffering model from a translation that either always
   flushes or never flushes.

4. **`atoi` is `(int)strtol(s, NULL, 10)`, not a checked parse.**
   Leading whitespace skipped, one optional sign, conversion stops at the first
   non-digit, no error on trailing junk, and the `long` result is *truncated* to
   `int`. So `50abc` -> 50, `abc50` -> 0, `0x1F` -> 0, `0.9` -> 0,
   `4294967396` -> 100, `2147483648` -> `INT_MIN` (which then crashes).
   A translation using `str::parse::<i32>()` would fail on all of these.

5. **`fgets` with a 14-byte buffer keeps at most 13 payload bytes, and does not
   read across newlines.** `00000000000009` truncates to `0000000000000`
   (`data == 0`), and in `7\n50\n` the `50` is never consumed. This is the
   `scanf`-vs-`fgets` distinction; a `read_to_string`-based translation would
   have consumed the whole stream.

6. **`strncpy` does not NUL-terminate when the count is consumed, and pads when
   the source is shorter.** Only reachable benignly here because `dest` is
   zero-initialized and `dest[data] = '\0'` follows, but the copy semantics were
   still matched byte for byte, verified for every length 0..=99.

7. **Empty `dest` still prints a newline.** `printLine("")` is
   `printf("%s\n", "")`, one byte of output - not zero bytes. Both the
   `data == 0` and `data >= 100` paths depend on this.

## Negative control

To confirm the suite is not vacuously passing, `translation/src/main.rs` was
temporarily perturbed to `exit(1)` on the `data >= 100` path - a change that
alters *only* the exit status and leaves stdout and stderr identical. Four tests
failed immediately. The perturbation was reverted and the full suite passes
again. This demonstrates the harness compares exit status, not just stdout.

## Completion state

- Both programs build with no errors.
- `cargo test` and `cargo test --release` pass: 34 passed, 0 failed, 0 ignored.
- No test is disabled, skipped or `#[ignore]`d.
- Nothing in `c_src/` was modified; only the untracked `c_src/build/` output
  directory was created, as the build instructions direct.
