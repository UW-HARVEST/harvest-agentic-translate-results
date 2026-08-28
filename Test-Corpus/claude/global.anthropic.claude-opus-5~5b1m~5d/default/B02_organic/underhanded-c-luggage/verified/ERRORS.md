# Verification log — `c_src/src/luggage.c` vs. the Rust translation

## What was done

| Phase | Action | Result |
|-------|--------|--------|
| A | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` — builds clean (GCC 11.5.0) |
| A | `cd translation && cargo build --release` | `translation/target/release/driver` — builds clean, no warnings |
| B/C | `cargo test` (`translation/tests/differential.rs`) | 19 tests: 2 468 stdin/argv comparisons + 14 process-environment comparisons, all pass |
| B/C | ad-hoc campaigns (`tests/difftest.py` plus two 12 000-case fuzzers) | ~35 000 comparisons of stdout + stderr + exit status, 0 mismatches |
| D | mutation testing of the Rust source (41 mutants) | every non-equivalent mutant is caught by the suite |

Run commands (both take four argv arguments and read records from stdin):

```
c_src/build/driver               <luggage_id> <flight_id> <departure> <arrival>
translation/target/release/driver <luggage_id> <flight_id> <departure> <arrival>
```

## Mismatches found

**None.** No input was found for which the two programs differ in stdout, in
stderr, or in exit status. Nothing in `translation/src/` had to be changed as a
result of this verification, and nothing in `c_src/` was touched.

Because "a mismatch you fixed without recording is a mismatch the next reader
cannot check", the sections below record every candidate mismatch that was
investigated — the places where a translation of this program *would* normally
diverge — together with the C behaviour that was measured and the code in
`translation/src/` that already reproduces it. Each one is pinned down by a test
in `tests/differential.rs`.

### 1. `%80[^\n]` captures the blank in front of the comment → double space

`printMatchingDirectives()` prints `"... %s %s\n"` with a space of its own, and
`scanf("%3[A-Z] %3[A-Z]")` has *no* trailing whitespace directive, so the blank
that separates the arrival code from the comment ends up inside `comments`.

```
$ printf '1 LUG00001 FL1234 JFK LAX hi\n' | c_src/build/driver - - - -
0000000001 LUG00001 FL1234 JFK LAX  hi
                                   ^^ two spaces
```

Reproduced by `Reader::scanset()` never skipping leading whitespace
(`translation/src/scan.rs`). Tests: `single_record_variants`,
`field_width_boundaries`. A mutant that emits a single space is caught.

### 2. A record with no comment and no trailing newline is silently dropped

`scanf("%80[^\n]")` returns `EOF` (not `0`) when the stream is already exhausted,
and `main()` breaks *before* `calloc`/`addRoutingDirectiveToList`:

```
$ printf '1 LUG00001 FL1234 JFK LAX\n' | c_src/build/driver - - - -
0000000001 LUG00001 FL1234 JFK LAX          <- kept, empty comment
$ printf '1 LUG00001 FL1234 JFK LAX'   | c_src/build/driver - - - -
                                            <- nothing: the record is dropped
```

Note the trailing space in the first line: `printf` still emits the separator
for the empty `comments` string. Reproduced by `scan_comments()` returning `EOF`
only when `Reader::at_eof()` holds at the start of the conversion, versus `0` for
a matching failure. Tests: `single_record_variants`,
`scan_failure_and_break_paths`. Mutants `comments_eof_kept` and
`comments_fail_is_eof` are both caught.

### 3. Failed conversions leave the previous record's characters in the buffers

The five `char` arrays live in `main()`'s frame; only `comments[0]` is reset each
iteration. `scanf` writes nothing on a matching failure, so a malformed record
inherits fields from the record before it:

```
$ printf '1 AAAAAAAA FL0001 JFK LAX one\n2 BBBBBBBB FL0002 12 34 two\n' \
      | c_src/build/driver - - - -
0000000001 AAAAAAAA FL0001 JFK LAX  one
0000000002 BBBBBBBB FL0002 JFK LAX 12 34 two
```

Record 2's two `%3[A-Z]` conversions both fail on `12`, so it keeps record 1's
`JFK LAX`, and `%80[^\n]` swallows `12 34 two`. There is only *one* space before
`12`, because the trailing whitespace directive of `"%6[A-Z0-9] "` had already
eaten the separator.

When the *luggage id* is the field that fails, the stale copy makes the broken
record supersede the good one, so the earlier line disappears entirely:

```
$ printf '1 AAAAAAAA FL0001 JFK LAX one\n2 lower FL0002 BOS SFO two\n' \
      | c_src/build/driver - - - -
0000000002 AAAAAAAA FL0001 JFK LAX lower FL0002 BOS SFO two
```

Reproduced by carrying the five buffers outside the loop in `main()` and only
clearing `comments`. Tests: `stale_buffer_reuse`,
`scan_failure_and_break_paths`. The mutant that clears all five buffers is
caught, as is the one that clears the destination on a matching failure.

On the *first* iteration those buffers are uninitialised in C — formally
undefined behaviour. Measured against this build they read as zero-length
strings and a zero time stamp, which is what the Rust models:

```
$ printf 'A B C D E\n' | c_src/build/driver - - - -
0000000000 A B C D  E
$ printf 'abc\n'       | c_src/build/driver '' '' '' ''
0000000000     abc
```

Tests: `scan_failure_and_break_paths` (`only_letters`, `nothing_matches`),
`filters_and_wildcards` (`empty_field_match`).

### 4. `%d` is read into an `unsigned int`, and glibc saturates before truncating

`scanf("%d ", &time_stamp)` is passed an `unsigned int *`. glibc converts with
`strtol`, saturating at `LONG_MAX`/`LONG_MIN` on overflow, and stores the low 32
bits:

| stdin time stamp | printed `%010u` |
|---|---|
| `-5` | `4294967291` |
| `-2147483649` | `2147483647` |
| `-99999999999999999999` (→ `LONG_MIN`) | `0000000000` |
| `2147483648` | `2147483648` |
| `4294967295` | `4294967295` |
| `12345678901` | `3755744309` |
| `99999999999999999999` (→ `LONG_MAX`) | `4294967295` |
| `9` × 5000 | `4294967295` |

Reproduced by `scan_time_stamp()` accumulating into `i64`, saturating to
`i64::MAX`/`i64::MIN`, then `(as_long as i32) as u32`. Test:
`timestamp_conversions`. The mutant that drops the saturation is caught.

Consequence for sorting: `addRoutingDirectiveToList` compares
`unsigned int`, so `-1` sorts *after* `2147483648`. Test:
`insertion_order::wrapped_stamps`.

### 5. `%d` matching failure consumes a lone sign but is not `EOF`

`- LUG00001 ...` makes glibc consume the `-`, push back the following character
and return `0`, so `main()` does *not* break; the remaining conversions then all
fail and `%80[^\n]` swallows the rest of the line. Reproduced in
`scan_time_stamp()` (the sign is consumed before the digit count is checked, and
`0` — not `EOF` — is returned when `digits == 0`). Tests:
`timestamp_conversions` (`-`, `+`, `--5`, `+-5`, `-x`, `0x1A`, `1.5`, `1e5`,
`5,000`), `scan_failure_and_break_paths`.

### 6. `supersedes()` stops at the first later directive for the same luggage

It is *not* "is there any later directive with the same luggage and departure";
the first same-luggage directive decides the answer even when it says "no":

```
$ printf '1 L1 F1 JFK LAX v1\n2 L1 F2 BOS SFO v2\n3 L1 F3 JFK ORD v3\n' \
      | c_src/build/driver - - - -
0000000001 L1 F1 JFK LAX  v1      <- NOT hidden, even though record 3 also departs JFK
0000000002 L1 F2 BOS SFO  v2
0000000003 L1 F3 JFK ORD  v3
```

Reproduced by `DirectiveList::supersedes()` returning
`directive.departure == departure` at the first id match instead of continuing
the walk. Tests: `supersession_rules`. The mutant that keeps walking is caught.

### 7. Field widths, and the NUL terminator inside `comments`

`%8[A-Z0-9] %6[A-Z0-9]` / `%3[A-Z] %3[A-Z]` / `%80[^\n]` truncate; the overflow
is *not* discarded, it feeds the next conversion:

```
$ printf '1 ABCDEFGHIJKLMNOP QRS TUV c\n' | c_src/build/driver - - - -
0000000001 ABCDEFGH IJKLMN OP QRS  TUV c
```

(`%8[A-Z0-9]` takes `ABCDEFGH`, `%6[A-Z0-9]` takes the next six, `IJKLMN`, the
first `%3[A-Z]` gets only `OP` because the blank stops it, the second takes
`QRS`, and `%80[^\n]` takes ` TUV c`.)

`[^\n]` also matches `\0`, and the byte is copied into the array, so `strcpy`,
`strcmp` and `%s` only ever see the part before it — while the stream position
still accounts for all 80 bytes. `Reader::scanset()` truncates `dst` at the first
NUL but advances past the whole match. Tests: `field_width_boundaries`,
`binary_input`. Mutants that change any of the four widths, or that skip the NUL
truncation, are caught.

### 8. `matches()` treats any argument starting with `-` as a wildcard

`expected[0] == '-'` — so `-`, `--`, `-x` and `-\xff` are all wildcards, while
the empty argument matches only an empty field. Non-UTF-8 argv has to survive,
hence `std::env::args_os()` + `OsStrExt::as_bytes()` rather than `args()` (which
would panic). Tests: `filters_and_wildcards`.

### 9. `argc != 5`

The message goes to **stderr**, the exit status is **1**, and stdin is never
read. Both too few and too many arguments take the path:

```
$ c_src/build/driver -; echo $?
Command line error: 4 arguments expected      (on stderr)
1
```

Test: `argc_error_path`. Mutants that send the message to stdout, drop the
trailing newline, or exit 0 are all caught.

### 10. `SIGPIPE`: the one place where Rust's runtime differs by default

A C program starts with `SIGPIPE` set to `SIG_DFL`, so a closed stdout kills it
with signal 13. Rust's runtime sets `SIGPIPE` to `SIG_IGN`, which would make the
translation exit **0** where the C dies from a **signal** — a genuine exit-status
mismatch. `main()` calls `restore_default_sigpipe()` before anything else, so
both die identically:

```
both programs, stdout pipe closed before they print: code=None signal=Some(13)
```

Test: `broken_stdout_kills_both_programs_the_same_way` (small output, so the
failing write happens inside the final flush, and >64 KiB of output, so it
happens mid-stream). Commenting out `restore_default_sigpipe()` is caught.
`closed_descriptors_are_survived_identically` additionally checks fd 1 and/or
fd 2 being outright closed: both programs ignore the write errors and keep their
normal exit status (0, resp. 1 on the argc path).

## Mutation testing

41 mutants were injected into `translation/src/` (one at a time, sources restored
afterwards) and `cargo test --release` was run for each. Everything was caught
except two mutants that are *semantically equivalent* to the original, i.e. no
input can distinguish them:

* `*dst = as_long as u32` instead of `*dst = (as_long as i32) as u32` — both
  keep the low 32 bits of the `i64`.
* dropping the `break` on `scan_time_stamp(...) == EOF` — that function returns
  `EOF` only when the reader is already at end of input, in which case the very
  next call (`scan_ids`) returns `EOF` and breaks instead. The same argument
  applies to the `EOF` breaks after `scan_ids` and `scan_airports`; only the
  `scan_comments` break is observable, and it is covered by §2.

Caught mutants included: no zero padding in `%010u`; `>=` instead of `>` when
inserting into the list (would reorder equal time stamps); starting the print
walk at the list sentinel; `superseded()` starting at the directive itself;
dropping the wildcard in `matches()`; treating a scanset matching failure as
`EOF`; adding or removing whitespace skips around each conversion; letting
`[A-Z]` accept digits or `[A-Z0-9]` reject them; letting `[^\n]` accept `\n`;
restricting `isspace` to `' '`; each of the four field widths; clearing the
stale buffers; skipping the NUL truncation; changing the exit status or the
error message.

## Notes for the next reader

* `tests/differential.rs` builds the C reference itself (via `cmake`) the first
  time it needs it, so `cargo test` works from a clean checkout; it never links
  the translation as a library, it only spawns `CARGO_BIN_EXE_driver`.
* No test is `#[ignore]`d, skipped or disabled.
* One trap while re-verifying by hand: `cargo test` rebuilds the binary it uses,
  but a stale `target/release/driver` from an earlier experiment will silently be
  used by any *manual* comparison. Always `cargo build --release` before
  comparing the two binaries outside the test suite.
* The C program is O(n²) in the number of records (`superseded()` is called for
  every record and walks the rest of the list) and recurses once per list node,
  so both programs get slow well before they get wrong; the largest cases in the
  suite use 2 000 records, which both handle identically.
