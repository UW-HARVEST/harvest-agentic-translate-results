# Differential verification findings

Ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Program under test: `translation/src/main.rs`, built to
`translation/target/{debug,release}/driver`.

Comparison method: `translation/tests/differential.rs` spawns both binaries as
subprocesses with identical stdin and asserts byte equality of stdout, byte
equality of stderr, and an identical exit status (distinguishing a normal exit
code from death by signal). The Rust code is never loaded as a library.

Commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## Mismatches found and fixed

### 1. Exit status differed when stdout is a pipe with no reader (SIGPIPE)

**Symptom.** With stdin `abc\nz\n` and stdout connected to a pipe whose read end
is already closed:

| | stdout | stderr | status |
|---|---|---|---|
| C | (none) | (empty) | killed by signal 13 (`SIGPIPE`) |
| Rust (before fix) | (none) | (empty) | exit 0 |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. The failing write therefore returned `EPIPE`, which the translation
discarded (`let _ = write!(...)`), and the process exited 0. The C program keeps
the default disposition, so the same write raises `SIGPIPE` and the process is
killed. `printf` output is buffered, so in C the fatal write is the flush at
`exit`, but the observable result is the same signal death.

**Fix.** `restore_default_sigpipe()` in `translation/src/main.rs` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, restoring the C
disposition. No change to the writing code was needed.

**Regression test.** `closed_stdout_is_fatal_via_sigpipe` builds a pipe, closes
the read end, hands the write end to each child as stdout, and requires both to
report signal 13. The test also asserts the C side really is signal 13, so the
test cannot silently degrade into a tautology if the C behaviour ever changes.

This was the only observable divergence found. Everything below was verified as
already matching, and is covered by tests so it stays that way.

## Behaviours confirmed identical (including the ones that look like bugs)

These are the branch points of the C program. There are no `if` statements in
`main`; every branch lives inside `fgets`, `strlen` and `strcspn`, so the input
classes were enumerated from those.

- **`fgets` returns NULL at EOF.** The buffers are `char s1[100] = ""`, so a
  failed `fgets` leaves the previous contents — here, all zeros. Empty stdin
  therefore prints `0`. Covered by `empty_input`, `stdin_at_eof_from_dev_null`,
  and `single_line_with_newline` (second `fgets` fails).
- **`fgets` keeps the trailing newline** and stops after at most 99 bytes.
  A 99-byte line leaves no newline in the buffer, so the rest of the physical
  line is what the *second* `fgets` reads: `s2` can come from the same input
  line as `s1`. Covered by `fgets_length_boundary`, `full_length_sweep`
  (every line-1 length 0..=205 crossed with newline presence and five shapes of
  line 2) and `match_position_across_the_boundary`.
- **`s[strlen(s) - 1] = '\0'` is unconditional.** For a normal line this strips
  the newline. When the line was truncated at 99 bytes or ended at EOF without a
  newline, it strips a **data** byte instead. This is replicated, not fixed:
  `single_line_without_newline`, `second_line_without_newline` and
  `chop_removes_the_matching_char` (the chop removes the byte that would
  otherwise have matched, changing the printed number).
- **`s[strlen(s) - 1]` when `strlen(s) == 0` writes out of bounds** at `s[-1]`.
  This happens on empty stdin and whenever a line's first byte is NUL. The write
  is unobservable in this program: `fgets(s, 100, ...)` can only ever place a
  NUL, never a data byte, at index 99 of either array, and both arrays start
  zeroed, so the byte preceding either buffer is already 0 whichever order the
  compiler lays the two arrays out in. The translation skips the write and
  documents why. Verified empirically to produce identical output in
  `empty_input`, `nul_at_start_of_each_line`, `nul_sweep_over_offsets` and
  `nul_only_input`.
- **Embedded NUL bytes are stored by `fgets` but terminate the C string.**
  `ab\0cd` has `strlen` 2, so `strcspn` only ever sees `ab` — and after the chop,
  `a`. Covered by `nul_sweep_over_offsets` (NUL at every offset 0..=101) and
  `nul_only_input`.
- **`strcspn` set semantics.** The reject set is the bytes of `s2` excluding its
  terminator; the scan stops at `s1`'s terminator; an empty `s2` yields
  `strlen(s1)`; a match at index 0 yields 0. Covered by `match_at_index_zero`,
  `match_in_the_middle`, `no_match_at_all`, `empty_reject_set`, `empty_subject`,
  `reject_set_with_duplicates` and `all_byte_values_as_the_reject_set` (one case
  per byte value 0x01..=0xff).
- **`printf("%zu\n", ...)`** — decimal, no padding, single trailing newline, and
  nothing on stderr on any path. Asserted byte for byte by every test.
- **Bytes are bytes.** Input is not text: invalid UTF-8, lone continuation
  bytes and truncated multi-byte sequences pass through unchanged
  (`high_bytes_and_invalid_utf8`). CR is an ordinary data byte, so CRLF input
  leaves a `\r` that `strcspn` counts (`tabs_and_carriage_returns`).
- **Input past the second line is ignored**, and the programs may exit while
  the writer still has data queued (`third_line_is_ignored`; the harness
  tolerates `EPIPE` when feeding stdin).
- **`fgets` read error.** With stdin pointing at a directory, `read(2)` fails
  with `EISDIR`, `fgets` returns NULL and the buffer keeps its initial `""`.
  Covered by `unreadable_stdin_is_a_read_error`.
- **Arbitrary byte strings.** `deterministic_fuzz` runs 600 pseudo-random inputs
  (0..260 bytes, biased towards `\n`, NUL and a small alphabet) through both
  binaries. The generator is a fixed-seed xorshift so the corpus is identical on
  every machine and every run.

## Status

- Both programs build with no errors and no warnings.
- `cargo test` in `translation/`: 30 tests, all passing, in both the debug and
  release profiles. Nothing is `#[ignore]`d, skipped or disabled.
- Approximately 1,300 distinct inputs are compared across the suite; all agree
  on stdout, stderr and exit status.
- Nothing in `c_src/` was modified. The only addition there is the
  CMake-generated `c_src/build/` output directory.
