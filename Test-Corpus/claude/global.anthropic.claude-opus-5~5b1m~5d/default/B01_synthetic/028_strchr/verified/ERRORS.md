# Differential verification log

C ground truth: `c_src/src/main.c` (built via CMake to `c_src/build/driver`)
Rust under test: `translation/src/main.rs` (built to `translation/target/release/driver`)

Run commands used for comparison:

```
c_src/build/driver                  < input
translation/target/release/driver   < input
```

## Outcome

**No behavioral mismatches were found.** The Rust translation produced
byte-identical stdout, byte-identical stderr and an identical exit status for
every input class enumerated below, including a 200-case randomized sweep.

Because "found no bugs" is a claim that is only as good as the test suite that
backs it, the suite was mutation-tested (see *Suite validation* below) to prove
it is capable of detecting the mismatches it is asserting the absence of.

## Behaviors of the C that had to be replicated exactly

These are the places where a plausible translation could have gone wrong. Each
is preserved correctly in the Rust, and each has a test pinning it.

| # | C behavior | Why it is easy to get wrong | Test |
|---|---|---|---|
| 1 | `fread(in, 1, sizeof(in), stdin)` reads **across newlines** | A translation using `read_line`/`BufRead::lines` or `fgets` semantics would stop at the first `\n` and undercount | `embedded_newlines_are_just_bytes` |
| 2 | `fread` does **not** skip leading whitespace | A translation using `scanf("%s")`-like token reading would drop leading spaces/tabs and stop at the first blank | `whitespace_and_tabs` |
| 3 | `in` is consumed by `strchr`, so the count stops at the **first NUL byte**, even though `fread` copied the bytes after it into the buffer | Treating the buffer as "all bytes read" instead of "the C string" over-counts on input containing `\0` | `nul_byte_terminates_the_counted_region`, `all_nul_input` |
| 4 | `char in[1000]` is zero-initialized by `= ""`, so with no input the string is empty | Reading an uninitialized buffer would produce garbage counts | `empty_input`, `stdin_is_immediately_at_eof` |
| 5 | At most **1000** bytes are ever read; everything after is invisible | An off-by-one buffer size, or reading all of stdin, changes the counts for large inputs | `buffer_boundary_sizes`, `input_longer_than_the_buffer_is_truncated` |
| 6 | `fread` retries short reads until the buffer is full or EOF | A single `read()` call returns only what the pipe currently holds, undercounting a slow producer | `short_reads_are_retried_until_eof` |
| 7 | `strchr` compares as `unsigned char`, so `0xC1` is not `'A'` | On platforms where `char` is signed, a sign-extending comparison could alias high bytes onto ASCII | `high_bytes_are_not_confused_with_ascii` |
| 8 | Matching is case-sensitive: `'a'` is not `'A'`, `'X'` is not `'x'` | An `eq_ignore_ascii_case` style comparison would over-count | `counting_is_case_sensitive` |
| 9 | Output is exactly `"A: %d\n"` then `"x: %d\n"` — label, colon, one space, decimal, newline | Spacing, ordering or a missing trailing newline are all invisible to a stdout-only eyeball check | `output_format_is_exact` |
| 10 | `main` unconditionally `return 0;` — there is **no** error path, and nothing is ever written to stderr | A translation that errors out on unreadable/binary input would exit non-zero where C exits 0 | `always_exits_zero` |

Note on #3 and the `foo` loop: the C `for (const char *s = in; s = strchr(s, c); s++)`
counts *occurrences*, not distinct positions — `s++` after each hit means
overlapping is impossible and every matching byte is counted exactly once. The
Rust's straightforward "count matching bytes in the slice" is equivalent.

## Undefined behavior in the C, and how it is handled

`main` does `fread(in, 1, sizeof(in), stdin)` into `char in[1000]` and then
passes `in` to `strchr`. If stdin supplies **exactly 1000 or more** bytes with
no NUL among the first 1000, the buffer contains **no NUL terminator**, and
`strchr` reads past the end of `in`. This is undefined behavior in the C.

- Observed behavior of the built C binary: the bytes immediately following `in`
  on the stack happen to be zero, so `strchr` stops right at offset 1000 and
  the counts equal "first 1000 bytes only".
- The Rust translation defines this case as exactly that: `unwrap_or(buf.len())`
  bounds the scan at 1000.
- These agree for this build, and `input_exactly_fills_the_buffer` /
  `input_longer_than_the_buffer_is_truncated` assert the agreement.

This is recorded rather than "fixed": the Rust matches the C binary's actual
observed behavior. It is worth knowing that this one input class rests on UB, so
a differently-laid-out C build could in principle read further and disagree.
The Rust's choice is the only self-consistent, memory-safe reading of the
programmer's intent.

## Suite validation (mutation testing)

To confirm the tests are not vacuously passing, eight faults were injected into
`translation/src/main.rs` one at a time and `cargo test` was re-run. All eight
were caught; `src/main.rs` was then restored and the suite re-confirmed green.

| Injected fault | Result |
|---|---|
| `res += 1` → `res += 2` (off-by-one count) | 15 of 20 tests failed |
| Drop the NUL truncation (scan the whole buffer) | 2 failed |
| `"A: {}\n"` → `"A:{}\n"` (spacing) | 19 failed |
| `b'A'` → `b'a'` (wrong char) | 14 failed |
| Add `std::process::exit(1)` | 20 failed |
| Read once instead of looping to EOF | 1 failed |
| Emit a line on stderr | 20 failed |
| Buffer `[0u8; 1000]` → `[0u8; 1024]` | 1 failed |

The three single-test detections are the intended narrow cases: the NUL, the
short-read and the buffer-size faults are each only observable on one specific
input class, which is precisely why those tests exist.

## Inputs covered

Empty; single byte (`A`, `x`, unrelated, newline, space); neither char present;
only one of the two chars; both interleaved; case variants; embedded/leading/
trailing newlines, blank lines, CRLF; spaces and tabs; NUL in first / leading /
middle position and multiple NULs; 1000 NULs; every byte value `1..=255`;
high-bit bytes and UTF-8 text; lengths 1, 2, 998, 999, 1000, 1500; exactly-full
buffer; over-full buffer; 999 bytes plus explicit NUL; dribbled 64×2-byte
flushed writes; `< /dev/null`; closed stdin; and 200 pseudo-random cases with
lengths straddling 1000 and NUL/newline/`A`/`x`/arbitrary bytes mixed.

## Notes

Nothing in `c_src/` was modified. Verified: `c_src/src/main.c` and
`c_src/CMakeLists.txt` retain their original mtimes, and the CMake build is
out-of-tree in `c_src/build/`.
