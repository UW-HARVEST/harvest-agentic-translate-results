# Differential verification of the C → Rust translation

Program under test: reads two lines with `fgets` into 100-byte buffers, deletes
the last byte of each, and prints `strcspn(s1, s2)`.

## Result

**No behavioural mismatch was found.** The Rust translation in `src/main.rs`
already matched the C binary on every input tried: stdout byte for byte, stderr
byte for byte, and exit status. No change to `src/main.rs` was needed, and
nothing in `c_src/` was modified.

Because "no mismatch found" is only as good as the search behind it, the rest of
this file records what was checked, and how the test suite itself was validated.

## Build and run commands

| | command | binary |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver` |

Both build with no errors and no warnings. `tests/differential.rs` builds each on
demand, and falls back to configuring the C program into
`translation/target/c_build` if `c_src/build/driver` is absent, so the test run
never has to write into `c_src/`.

## Semantic hazards checked, and why each one is already correct

These are the places where a translation of this program would normally drift.
Each was confirmed against the compiled C binary rather than reasoned about
alone.

### 1. `fgets` keeps the newline and does not span lines

`fgets` stops *after* storing `'\n'`, unlike `scanf`, which skips leading
whitespace and reads across newlines. `c_fgets` replicates this: it breaks out of
the loop after pushing the newline, so the byte the subsequent
`s[strlen(s)-1] = '\0'` deletes is the newline. Covered by `crlf_line_endings_...`
(the `'\r'` survives and becomes part of the reject set, giving 3 rather than 4)
and `whitespace_is_an_ordinary_character_...`.

### 2. The last byte is deleted unconditionally, not only when it is a newline

When the final line has no trailing newline, `fgets` stops at EOF and the store
eats a real character: `"abc"` → `"ab"` → prints `2`, and `"a"` → `""` → prints
`0`. This looks like a bug but is the specified behaviour and is reproduced.
Covered by `single_line_without_newline` and `single_char_no_newline`.

### 3. The 99/100-byte `fgets` boundary

`fgets(s, 100, stdin)` stores at most 99 bytes plus a NUL. So a 99-character
first line consumes the buffer *without* consuming its newline, and that newline
becomes the whole of the second line — which then strips to empty, yielding an
empty reject set. A 100-character line pushes its 100th character into `s2`,
which usually makes the answer `0` instead of a large number. Covered by the
98 / 99 / 100 / 150 / 120-byte tests plus a randomized sweep over lengths
90..=110 on both lines.

### 4. `strcspn` boundaries at the NUL terminator on *both* sides

Two separate off-by-one traps: the scan of `s1` must stop at `s1`'s NUL, and
`s2`'s own NUL must **not** join the reject set (otherwise every input would
match at index 0). `c_strcspn` slices `s2` to `c_strlen(s2)` and bounds the loop
by `c_strlen(s1)`. Both directions are covered, and mutating either one is caught
(see the mutation table below).

### 5. Embedded NUL bytes

`fgets` happily stores a `\0` from stdin, but `strlen` then reports a shorter
string. Input `"a\0bcd\n"` gives `strlen == 1`, so the store empties the buffer
and the program prints `0`. `c_strlen` reproduces this by scanning for the first
zero byte. Covered by `embedded_nul_truncates_the_string_early`.

### 6. `s[strlen(s) - 1]` when `strlen(s) == 0` — the out-of-bounds store

This is the one genuine undefined-behaviour site in the C, and it is reachable:
on empty input (both `fgets` calls return `NULL`, buffers stay zeroed), on a
line consisting of just `"\n"` after the strip, and on any line beginning with a
NUL byte. `strlen` returns 0, so the C evaluates `s[-1]` and writes one byte
before the array.

`strip_last_byte` in the Rust deliberately **skips** the store in that case. That
is a divergence in the machine operation, so it was verified rather than assumed
to be harmless. Disassembling `c_src/build/driver`:

```
sub    $0xe0,%rsp                  ; 224-byte frame
lea    -0x70(%rbp),%rax            ; s1 at rbp-112, occupies rbp-112 .. rbp-13
lea    -0xe0(%rbp),%rax            ; s2 at rbp-224, occupies rbp-224 .. rbp-125
```

- `s1[-1]` is `rbp-113`, which falls in the 12 bytes of dead alignment padding
  between the end of `s2` (`rbp-125`) and the start of `s1`. Nothing reads it.
- `s2[-1]` is `rbp-225`, one byte below the allocated frame. It is written before
  `printf` is called and never read afterwards.

Also note that both stores write the value `0`, and the only bytes that could
plausibly be aliased (`s1[99]`, `s2[99]`) are always either zero from the
zero-initialisation or the NUL that `fgets` wrote, so even under a different
stack layout the store would be a no-op. Skipping it is therefore
behaviour-preserving here. Covered by
`out_of_bounds_store_with_a_completely_full_neighbour_buffer`, which pairs a
zero-length string with a neighbour buffer filled to all 99 usable bytes so that
any spill would change the printed number.

### 7. `printf("%zu\n", ...)` formatting and stream usage

Plain decimal, no padding or precision, exactly one trailing newline, on stdout
only. The C never writes to stderr and always returns 0, including on empty
input — there is no error path in this program, so "exit status matches" means
"both exit 0 on everything". Asserted explicitly in
`output_is_a_decimal_number_and_one_trailing_newline_on_stdout_only`.

### 8. Bytes that are not valid UTF-8

The C is byte-oriented throughout. The Rust operates on `[u8; 100]` and never
converts to `str`, so no lossy replacement or panic can occur. Covered by tests
over bytes `0xff 0xfe 0xfd`, Latin-1 input, and byte ranges 1..=90 and 150..=240.

## Coverage of the enumerated input classes

Every branch point in the C is one row here. All 37 tests assert stdout, stderr
and exit status together.

| input class | test | prints |
|---|---|---|
| empty input, both `fgets` return NULL | `empty_input_both_fgets_return_null` | `0` |
| `"\n"` only, second `fgets` returns NULL | `only_a_newline_second_fgets_returns_null` | `0` |
| two empty lines | `two_empty_lines` | `0` |
| one line, second `fgets` NULL | `single_line_with_newline_second_fgets_null` | `3` |
| no trailing newline (EOF stop) | `single_line_without_newline` | `2` |
| single character, no newline | `single_char_no_newline` | `0` |
| match in the middle | `happy_path_match_in_middle` | `4` |
| no match at all (returns full length) | `no_character_of_s2_occurs_in_s1` | `6` |
| match at index 0 | `match_at_first_character` | `0` |
| match at the last index | `match_at_last_character` | `3` |
| empty reject set | `second_line_empty_gives_empty_reject_set` | `6` |
| multi-character reject set, earliest wins | `multi_character_reject_set_takes_earliest` | `1` |
| single-character strings, match and no match | `single_char_strings` | `1`, `0` |
| a third line exists and is never read | `third_line_is_never_read` | `3` |
| neither line newline-terminated | `neither_line_has_a_trailing_newline` | `3` |
| CRLF input | `crlf_line_endings_leave_the_cr_in_place` | `3` |
| 98 chars + newline (exactly fits) | `line_of_98_chars_plus_newline_fits` | `98` |
| 99 chars (buffer full, newline left behind) | `line_of_99_chars_fills_the_buffer_...` | `98` |
| 100 chars (spills into `s2`) | `line_of_100_chars_spills_the_100th_into_s2` | `0` |
| 150 chars | `line_much_longer_than_the_buffer` | `0` |
| both lines 120 chars | `both_lines_overflow_the_buffer` | `0` |
| reject byte truncated off the end of `s2` | `reject_char_at_the_far_end_of_a_full_s2` | `51` |
| 1 MB input | `very_large_input_only_first_two_lines_matter` | — |
| embedded NUL in `s1` | `embedded_nul_truncates_the_string_early` | `0` |
| leading NUL → `s1[-1]` store | `leading_nul_makes_strlen_zero_...` | `0` |
| leading NUL on both lines | `leading_nul_on_both_lines` | `0` |
| leading NUL in `s2` → empty reject set | `leading_nul_in_s2_empties_the_reject_set` | `6` |
| `s[-1]` store beside a full 99-byte buffer | `out_of_bounds_store_with_a_completely_full_neighbour_buffer` | — |
| NUL-only lines | `nul_bytes_only_on_both_lines` | `0` |
| spaces and tabs | `whitespace_is_an_ordinary_character_...` | `1` |
| high bytes / invalid UTF-8 / Latin-1 | `high_bytes_and_invalid_utf8_...` | — |
| byte ranges 1..=90 and 150..=240 | `every_byte_value_1_through_255` | — |
| stdin closed immediately | `stdin_closed_immediately` | `0` |
| 1500 random inputs, length 0..260 | `randomized_differential_sweep` | — |
| 400 random inputs, lengths 90..=110 per line | `randomized_sweep_near_the_buffer_boundary` | — |

An additional ad-hoc sweep of 4000 random byte strings run outside the suite
also produced zero mismatches.

## Validating the test suite itself

A suite that finds no mismatch is worthless if it cannot detect one. Nine
deliberate bugs were injected into `src/main.rs` one at a time; **every one was
caught** by `cargo test --release`. `src/main.rs` was restored afterwards and
byte-compared against its backup.

| injected bug | detected |
|---|---|
| never delete the last byte | yes |
| `fgets` reads `buf.len()` instead of `buf.len() - 1` (off-by-one) | yes |
| `fgets` does not stop at the newline | yes |
| exit with status 1 instead of 0 | yes |
| drop the trailing newline from the output | yes |
| include `s2`'s NUL in the reject set | yes |
| scan `s1` past its NUL to the end of the buffer | yes |
| write a stray byte to stderr | yes |
| delete two trailing bytes instead of one | yes |

## Completion gate (Phase D)

- Both programs build with no errors — confirmed.
- Every enumerated input produces identical stdout, stderr and exit status —
  37/37 tests pass in both the debug and release profiles.
- `cargo test` passes in `translation/` — 37 passed, 0 failed, 0 ignored.
- No test is disabled, skipped or `#[ignore]`d; the file contains no `#[ignore]`
  and no early `return`s that bypass assertions.
- `c_src/` sources are unmodified; only the `c_src/build/` output directory was
  created, per the build instructions.
