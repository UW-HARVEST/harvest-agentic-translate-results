# Differential verification: `c_src/src/main.c` vs `translation/src/main.rs`

## Result

No output mismatches were found. Across every enumerated input class plus a
randomized sweep, the Rust binary produced byte-identical stdout, byte-identical
stderr (always empty), and an identical exit status (always 0) to the C binary.

## How the two programs were run

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver          # reads stdin

# Rust
cd translation && cargo build --release
./translation/target/release/driver   # reads stdin
```

Both are driven as subprocesses by `translation/tests/differential.rs`, which
writes the same bytes to each program's stdin and compares stdout, stderr and
exit status. The Rust code is never loaded as a library.

## The program under test

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}

int main() {
    char s1[100] = "", s2[100] = "";
    fgets(s1, sizeof(s1), stdin);
    fgets(s2, sizeof(s1), stdin);

    s1[strlen(s1)-1] = '\0';
    s2[strlen(s2)-1] = '\0';

    driver(s1, s2);
    return 0;
}
```

## Quirks in the C that the Rust must reproduce (and does)

These are the places where a natural, idiomatic Rust rewrite would silently
diverge. Each is deliberately preserved, and each is pinned by a test.

### 1. The last byte is deleted unconditionally, not "the newline is stripped"

`s1[strlen(s1)-1] = '\0'` removes the final byte whatever it is. It only looks
like newline-stripping because the common case ends in a newline. When the line
has no trailing newline (EOF) or was truncated by `fgets`, this deletes a
**data** byte.

- Input `"abc"` (no newline) -> `s1` becomes `"ab"`.
- A 99-byte line fills the buffer with no newline, so the chop leaves 98 bytes.

Pinned by `single_line_no_trailing_newline`, `buffer_length_boundaries`.
A mutant that made the chop newline-aware (the "fixed" version) failed 9 tests.

### 2. `s[strlen(s)-1]` underflows to index -1 when the string is empty

If `fgets` hits EOF immediately it returns NULL and leaves the buffer as `""`,
so `strlen` is 0 and the index is `(size_t)-1`. This is an out-of-bounds store
and formally undefined behavior. It is emulated as a no-op, which was confirmed
against the compiled binary rather than assumed:

```
main:
  ...          # s1 at rsp+0x00 (100 bytes, 0x00..0x63)
  lea 0x70(%rsp),%rdi        # s2 at rsp+0x70 (0x70..0xd3)
  movb $0x0,-0x1(%rsp,%rax,1)   # s1[strlen(s1)-1], rax=0 -> rsp-0x01
  movb $0x0,0x6f(%rsp,%rax,1)   # s2[strlen(s2)-1], rax=0 -> rsp+0x6f
```

With `rax == 0` the stores land at `rsp-0x01` (below the frame) and `rsp+0x6f`
(the padding gap between the end of `s1` at `0x63` and the start of `s2` at
`0x70`). Neither byte is part of `s1` or `s2`, so neither can affect the printed
result. Even had the arrays been adjacent it would remain a no-op: `fgets` never
leaves a non-zero byte at index 99 of either buffer, since it writes at most 99
bytes plus a NUL terminator.

Reached by empty input, `"\n"`, `"\n\n"`, and any input with fewer than two
non-empty lines. Pinned by `empty_and_near_empty_input`. A mutant that performed
the chop unconditionally (panicking on the empty case) failed 8 tests.

### 3. `fgets` does not read across newlines, and a long line spills into `s2`

`fgets(buf, 100, stdin)` consumes at most 99 bytes and stops after a newline.
A first line longer than 99 bytes is truncated, and **the remainder of that same
line is what the second `fgets` reads** — so `s2` is filled from the tail of
`s1`'s own line, not from a separate line.

- 149 `a`/`b` bytes on one line: `s1` = 99 `a`s (chopped to 98), `s2` = the
  50 `b`s (chopped to 49).
- 100 `a`s + newline: `s1` = 98 `a`s after the chop, `s2` = `"a"`, so `strcspn`
  returns 0, not 100.

Pinned by `long_line_spills_into_second_fgets`, `buffer_length_boundaries`,
`reject_byte_at_every_index_near_boundary`, `reject_set_at_length_boundary`.
A mutant reading 100 bytes instead of 99 failed 6 tests.

### 4. NUL bytes in the input truncate the effective strings

`fgets` copies NUL bytes into the buffer verbatim, but `strlen` and `strcspn`
stop at the first one. So an embedded NUL shortens `s1`, and a leading NUL in
`s2` empties the reject set entirely (making `strcspn` return `strlen(s1)`).

- `"a\0b\ncd\n"`: `strlen(s1)` is 1, the chop clears index 0, `s1` is empty -> `0`.
- `"abc\na\0b\n"`: reject set is empty -> `3`.

Pinned by `embedded_nul_bytes`, `boundary_with_nul_and_crlf`. A mutant that
scanned the full 100-byte reject buffer instead of stopping at NUL failed 2
tests.

### 5. `char` signedness and high bytes

`strcspn` compares raw bytes, so values `0x80..0xff` must be handled as
unsigned. The Rust uses `u8` throughout. All 256 byte values were exercised on
each side of the comparison (`every_byte_value`).

### 6. Output format

`printf("%zu\n", ...)` emits an unsigned decimal with a trailing newline and
nothing else. stderr is never written; the exit status is always 0, including
for empty input. A mutant dropping the newline failed all 14 tests; a mutant
returning exit code 1 failed all 14.

## Coverage

`translation/tests/differential.rs`, 14 tests, none `#[ignore]`d, skipped or
disabled:

| Test | Input class |
|---|---|
| `both_binaries_run` | Phase A smoke check, both binaries produce the expected reference output |
| `empty_and_near_empty_input` | empty input, `"\n"`, `"\n\n"`, `"\n\n\n"`, bare NUL — the index `-1` paths |
| `single_line_no_trailing_newline` | one item; EOF with no newline; the chop eating a data byte |
| `strcspn_match_positions` | match at index 0 / middle / last index / no match / empty reject set / empty `s1` |
| `embedded_nul_bytes` | NUL in `s1`, in `s2`, leading, in both, all-NUL |
| `every_byte_value` | all 256 byte values as `s1`, as `s2`, and on both sides |
| `buffer_length_boundaries` | lengths 0,1,2,96–102,197–201,260 crossed with 6 tails |
| `long_line_spills_into_second_fgets` | the maximum the code handles, and lines exceeding it |
| `reject_byte_at_every_index_near_boundary` | reject byte at every index of 98/99/100/101-byte `s1` |
| `reject_set_at_length_boundary` | distinguishing byte at boundary positions of a 98–101-byte `s2` |
| `boundary_with_nul_and_crlf` | CRLF, lone CR, NUL at indices 98/99, all-NUL line |
| `trailing_input_is_ignored` | third and later lines, 10 KB trailing blob, binary garbage |
| `extra_arguments_are_ignored` | `int main()` takes no argv |
| `randomized_differential_fuzz` | 1200 deterministic random inputs, lengths biased to the 99/198 boundaries, alphabets rich in `\n`, `\0`, `\r`, high bytes |

Additional configurations checked manually, all matching: closed stdout,
stdout to a closed pipe, closed stdin, and stdin pointing at a directory.

## Harness validation

To confirm the suite is not vacuously green, six mutations were injected into
`translation/src/main.rs` one at a time; every one was caught (failure counts
above). `src/main.rs` was restored byte-for-byte afterwards and re-verified.

Nothing under `c_src/` was modified. The only addition there is the
out-of-tree `c_src/build/` directory produced by the prescribed CMake
invocation.
