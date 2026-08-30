# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

## How the two programs are run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `./c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `./translation/target/release/driver` |

Tests live in `translation/tests/differential.rs`. They spawn **both binaries as
subprocesses** (never the Rust code as a library) and compare stdout bytes,
stderr bytes, exit code and terminating signal for every input class.

## Input classes enumerated from the C source

`main(int argc, char *argv[])` never inspects `argc`/`argv`, never reads stdin,
and there is no `scanf`/`fgets`/`getchar` anywhere. The only conditional branch
in the whole program is `if (line != NULL)` inside `printLine`, and all six call
sites pass string literals, so the `NULL` arm is **dead code**. Consequently the
program is a constant-output, exit-0 executable and the input space to probe is
the process environment rather than parsed data:

* argv: none, one, two, empty string, flag-like (`-h --help -0 --`), whitespace
  and embedded newline/tab, printf format specifiers (`%s%d%n%%`), non-ASCII
  UTF-8, 512 arguments, one 64 KiB argument, differing `argv[0]` (renamed copy)
* stdin: empty, one line, several lines without a trailing newline, NUL/binary
  bytes, `/dev/null`, and a check that stdin is left **completely unconsumed**
  (`prog; cat` must still see every byte)
* environment/locale: `LC_ALL=C`, `LC_ALL=en_US.UTF-8`, `LC_NUMERIC=de_DE.UTF-8`
  (would expose locale-dependent integer grouping), and empty environment
* output destination: pipe, regular file (stdio switches to full buffering),
  `2>&1` merged stream, **closed stdout** (`exec 1>&-`), and an early-closing
  reader (`| head -c 3`, i.e. potential `SIGPIPE`)
* determinism: 8 repeated runs must be byte-stable

## Mismatches found

**None.** Every input class above produced byte-identical stdout, byte-identical
stderr (always empty) and exit status 0 from both programs on the first
comparison. No change to `translation/src/main.rs` was required, and nothing in
`c_src/` was modified.

The exact expected output, verified byte for byte (`cat -A` shows no trailing
spaces or `\r`):

```
Calling good()...
0
2
Finished good()
Calling bad()...
0
0
Finished bad()
```

### Behaviors that were specifically checked *not* to have been "fixed"

1. **`bad()`'s discarded expression.** The C writes `intOne + intTwo;` as a bare
   statement, so `intSum` is never assigned and `bad()` prints `0` twice. The
   Rust reproduces this with `let _ = int_one + int_two;` and keeps `int_sum`
   immutable. `bad_does_not_update_int_sum_but_good_does` asserts the integer
   sequence across the whole run is exactly `0, 2, 0, 0` — it fails if the Rust
   ever "corrects" `bad()` to print `2`.
2. **`printLine`'s NULL guard.** Modeled as `Option<&str>`, with `None` printing
   nothing. Unreachable from `main` in both versions, so it cannot cause an
   output difference; no test can reach it without modifying the C.
3. **`printf("%d\n")` formatting.** Plain, unpadded decimal with a single
   trailing newline per value, and no extra blank line at end of output.
   `line_by_line_structure_matches_the_c_source` pins line count, per-line
   content and the trailing-newline shape.
4. **Exit status.** C's `return 0` from `main`; the Rust ends with
   `std::process::exit(0)` after flushing stdout, so both report code 0 and no
   signal.

## Test-harness issue fixed (not a translation mismatch)

`different_argv0_program_name` initially failed with
`ExecutableFileBusy` / `ETXTBSY` (errno 26). Cause: `cargo test` runs test
functions on parallel threads, and a *different* test's `fork` can inherit the
still-open write file descriptor of the freshly copied binary, which makes
`execve` refuse to run it. This is a race in the test scaffolding, not a
behavioral difference, so the fix was a bounded retry on errno 26 in the test.
No production code and no C code was touched.

## Result

* Both programs build with no errors and no warnings.
* All 27 tests pass, in both the debug and `--release` profiles, repeatedly.
* No test is disabled, skipped or `#[ignore]`d.
* `c_src/` is unmodified (only the untracked `c_src/build/` output was added by
  the documented CMake build).
