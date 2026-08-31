# Differential verification of the C → Rust translation

Ground truth: `c_src/src/main.c`. The Rust program must produce byte-identical
stdout, byte-identical stderr and the same exit status for every input.

## Result

**No mismatches were found.** The translation in `src/main.rs` already matched
the C program on every input class enumerated below, and no change to
`src/main.rs` was required.

Because "no mismatches" is also what a suite that measures nothing reports, the
test suite itself was validated by fault injection (see *Suite validation*).

## What the C program does

```c
int foo(const char *in, char c) {
    int res = 0;
    for (const char *s = in; s = strchr(s, c); s++) res++;
    return res;
}
void driver(const char *in) {
    printf("A: %d\n", foo(in, 'A'));
    printf("x: %d\n", foo(in, 'x'));
}
int main() {
    char in[1000] = "";
    fread(in, 1, sizeof(in), stdin);
    driver(in);
    return 0;
}
```

There are no error paths: `main` always returns 0, nothing is ever written to
stderr, and the `fread` return value is discarded. The only branching is inside
`foo` (the `strchr(...) == NULL` loop exit) and the implicit branching created by
the buffer size and the NUL terminator.

## Input classes enumerated and covered

Every case below asserts stdout, stderr and exit status.

| Input class | Why the C branches on it | Test |
|---|---|---|
| Empty input | `fread` stores nothing; the zeroed buffer is scanned as `""` | `empty_input` |
| Single `A` / single `x` | one loop iteration then `strchr` → NULL | `single_a`, `single_x` |
| Single unrelated byte | `strchr` returns NULL on the *first* iteration | `single_unrelated_byte` |
| Adjacent matches (`AAA`) | `s++` must not skip the next match | `adjacent_matches` |
| Match at first / last position | scan start and the byte before the NUL | `match_at_first_and_last_position` |
| Interleaved / one-character-only | both counters exercised independently | `interleaved_matches`, `only_one_of_the_two_characters_present` |
| Wrong case (`a`, `X`) | `strchr` is an exact byte compare | `counts_are_case_sensitive` |
| Newlines, blank lines, no trailing newline | `fread` has no line semantics; it reads *past* `\n` | `reads_across_newlines`, `no_trailing_newline_vs_trailing_newline` |
| Embedded NUL / leading NUL / several NULs | raw bytes are read, but scanned as a C string, so the first NUL truncates | `embedded_nul_truncates_the_scan`, `leading_nul_yields_empty_string`, `multiple_nuls`, `nul_after_all_matches` |
| 999 bytes | last byte that leaves room for a terminator | `just_under_capacity` |
| Exactly 1000 bytes | `sizeof(in)` — the maximum the code handles, and no room for a terminator | `exactly_at_capacity` |
| 1001 / 6000 bytes | bytes past the buffer are never stored, so never counted | `one_byte_over_capacity`, `far_over_capacity_is_truncated` |
| Matches straddling byte 1000 | truncation boundary | `matches_straddling_the_capacity_boundary` |
| NUL at byte 1001 | terminator itself is truncated away | `capacity_reached_then_nul` |
| Bytes ≥ 0x80, invalid UTF-8, multi-byte UTF-8, control bytes | no UTF-8 or locale awareness; `Ä` (0xC3 0x84) is not `A` | `high_bytes_are_not_special`, `invalid_utf8_only`, `multibyte_utf8_containing_no_ascii_matches`, `control_bytes` |
| All 256 byte values, NUL first vs NUL last | full byte-domain sweep | `binary_payload_all_byte_values` |
| stdin delivered in slow chunks | a short read must not be mistaken for EOF | `input_delivered_in_several_chunks` |
| stdin at EOF (`/dev/null`) | `fread` returns 0 | `stdin_at_eof_immediately` |
| **stdin unreadable** (directory fd, `EISDIR`) | `fread` fails and the C **ignores the error** | `unreadable_stdin_is_ignored` |
| Command-line arguments present | `main()` takes no parameters, so argv is irrelevant | `ignores_command_line_arguments` |
| Output formatting | `"A: %d\n"` then `"x: %d\n"`: that order, single space, no padding, LF, exactly two newlines | `output_format_is_exact` |
| 156 pseudo-random inputs across 13 lengths | catches anything the hand-written cases miss | `randomised_sweep` |

## Quirks that had to be preserved, and were

1. **`fread`, not `fgets` or `scanf`.** Reading crosses newlines and whitespace
   and continues to EOF or 1000 bytes. Verified by `reads_across_newlines` and
   `input_delivered_in_several_chunks`.
2. **Read as raw bytes, scanned as a C string.** `fread` never appends a NUL and
   the buffer is zero-initialised, so an *embedded* NUL byte silently hides the
   rest of the input. `AA\0xxAA` prints `A: 2` / `x: 0`, not `A: 4` / `x: 2`.
3. **The `fread` return value is discarded.** An unreadable stdin is
   indistinguishable from empty input: the program still prints `A: 0` / `x: 0`,
   writes nothing to stderr and exits 0. The Rust code must swallow the I/O
   error rather than reporting it. This is the one behaviour that only the
   `unreadable_stdin_is_ignored` test detects.
4. **Hard 1000-byte cap.** Input 1001+ bytes long is silently truncated; the
   1001st byte cannot be counted even when it is an `A` or an `x`.
5. **Byte comparison, not character comparison.** `strchr` compares single
   bytes, so no multi-byte or non-ASCII sequence can ever match `A` or `x`, and
   invalid UTF-8 passes through without complaint. The Rust side must not touch
   `String`/`str` on the input path.
6. **No error paths at all.** stderr is always empty and the exit status is
   always 0, for every input including the pathological ones.

## Undefined behaviour in the C, and how it was handled

When stdin supplies exactly 1000 or more bytes, the buffer is filled completely
and **`in` is not NUL-terminated**, so `strchr` reads past the end of the array.
This is undefined behaviour in the C, which cannot simply be "translated".

Rather than guess, the observed behaviour was measured. The C was rebuilt at
`-O0`, `-O2`, and `-O3 -fstack-protector-strong` in addition to the CMake build,
and each was run repeatedly and with the stack shifted (a 60 KB environment
variable, and 500 extra environment variables). All builds and all layouts
reported `A: 1000` — the byte immediately after the array is consistently zero,
so the scan stops exactly at the array end.

`src/main.rs` reproduces this by treating position 1000 as an implicit
terminator (`position(|&b| b == 0).unwrap_or(BUF_SIZE)`), which is the behaviour
observed from the C and is also memory-safe. Pinned by `exactly_at_capacity`,
`one_byte_over_capacity` and `capacity_reached_then_nul`.

## Suite validation (fault injection)

To confirm the suite would actually catch a bad translation, six faults were
injected into `src/main.rs` one at a time; each was caught, and the original
source was restored afterwards (verified byte-identical).

| Injected fault | Caught by |
|---|---|
| `BUF_SIZE` 1000 → 999 | `capacity_reached_then_nul`, `far_over_capacity_is_truncated`, … |
| `s = found + 1` → `found + 2` (skips adjacent matches) | `adjacent_matches`, `one_byte_over_capacity`, … |
| Scan the whole read buffer instead of stopping at the first NUL | `leading_nul_yields_empty_string`, `binary_payload_all_byte_values`, … |
| `"A: {}\n"` → `"A:{}\n"` (dropped space) | `output_format_is_exact`, `just_under_capacity`, … |
| `exit(1)` instead of `return 0` | every test — all three streams are asserted |
| Report the stdin read error to stderr and exit 1 | `unreadable_stdin_is_ignored` (**only** this test) |

The last row is why the error-path cases matter: that fault leaves stdout
identical on ordinary input, so a stdout-only suite would have passed it.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # C binary
cd translation && cargo build --release                                # Rust binary
cd translation && cargo test                                           # 33 differential tests
```

`cargo test` builds the C binary itself via CMake if `c_src/build/driver` is
missing, so the suite is self-contained. Nothing in `c_src/` is modified; the
CMake build is out-of-tree in `c_src/build/`.
