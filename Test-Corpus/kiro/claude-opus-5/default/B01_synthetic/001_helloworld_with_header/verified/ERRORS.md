# Differential testing findings

Reference: `c_src/src/main.c` + `c_src/src/sillymain.c`.
The whole program is `printf("Hello World!\n"); return 0;` — no stdin is read,
`argc`/`argv` are ignored, and there is no error path inside the program itself.
The observable input classes are therefore the ways the process is invoked and
the states of its standard descriptors, all covered in
`translation/tests/differential.rs`.

## Commands

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → run `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → run `translation/target/release/driver`
- Tests: `cd translation && cargo test` (25 differential tests, none ignored)

## Mismatches found and fixed

### 1. SIGPIPE: Rust exited 0 where C is killed by signal 13

- Input class: stdout is a pipe whose read end closes before the write lands,
  e.g. `driver | true`.
- C behavior: SIGPIPE keeps its default disposition, so the failing write kills
  the process. The shell reports status 141 (128 + 13); `stdout` is empty.
- Rust behavior before the fix: the Rust runtime installs `SIG_IGN` for SIGPIPE
  before `main` runs, so the write returned `EPIPE`, `helloworld` ignored the
  error exactly as the C ignores `printf`'s return value, and the process exited
  **0** instead of dying from signal 13.
- Observed diff (shell reporting `${PIPESTATUS[0]}`): C `status=141`,
  Rust `status=0`.
- Cause: a Rust-runtime default that has no counterpart in C, not a difference
  in the translated logic.
- Fix (`translation/src/main.rs`): call `signal(SIGPIPE, SIG_DFL)` via the libc
  symbol at the top of `main`, restoring the C disposition before any output is
  written. Verified by re-running the suite with the call commented out: the
  test `piped_to_a_reader_that_exits_without_reading` fails
  (`status=141` vs `status=0`), and passes with the call in place.

## Input classes checked that already matched

No difference was observed in any of the following, so no change was needed:

- No arguments, stdin `/dev/null`: stdout is exactly the 13 bytes
  `Hello World!\n`, stderr empty, exit 0 for both.
- Empty stdin; one line; a line with no trailing newline; 1000 lines; 1 MiB of
  data (larger than the pipe buffer, and never read by either program, so
  neither deadlocks); invalid UTF-8 bytes including a NUL.
- Arguments: one, 64, and flag-shaped/empty/whitespace/non-ASCII arguments —
  ignored identically because `main` takes no parameters.
- Unusual `argv[0]`, including the empty string.
- Empty environment.
- Five repeated runs (byte-identical, deterministic).
- stdout redirected to a new file, appended to a non-empty file, sent to
  `/dev/null`, or merged into stderr (`1>&2`).
- stdout closed (`>&-`): the write fails, and both programs still exit 0 with no
  message on stderr, because the C ignores `printf`'s return value.
- stderr closed (`2>&-`), both stdout and stderr closed, stdin closed (`<&-`).
- Piped into `cat` (reader consumes everything) and into a reader that sleeps
  before exiting.
- Run from a different working directory.

## Notes

- Nothing in `c_src/` was modified; only `c_src/build/` was created by CMake as
  the build directory. Source checksums after testing:
  `main.c 1d9f1468a7fa21084e43ac0e63f464cd`,
  `sillymain.c c5102c33081f51b161d30c7e7651cb25`,
  `sillymain.h 748876bf67e67d715d4a5dffdbd521d0`,
  `CMakeLists.txt 46e2aa363a11de11088f8d2691668964`.
- The tests drive the built binaries as subprocesses only. The Rust code is
  never loaded as a library, and no `#[no_mangle]`/cdylib exports were added.
- Buffering differs internally (glibc buffers a piped stdout until exit; the
  Rust version writes and flushes immediately), but this is not observable:
  the program writes a single 13-byte line and nothing to stderr, so there is no
  interleaving to differ on.
