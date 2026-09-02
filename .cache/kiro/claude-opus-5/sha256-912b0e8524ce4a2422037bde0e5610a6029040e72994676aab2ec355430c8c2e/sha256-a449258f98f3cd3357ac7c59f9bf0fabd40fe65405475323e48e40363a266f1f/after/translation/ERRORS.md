# Differential verification log — `c_src/src/main.c` vs `translation/`

## What runs what

| Program | Build | Run |
|---|---|---|
| C (reference) | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver < INPUT` |
| Rust (translation) | `cd translation && cargo build --release` | `translation/target/release/driver < INPUT` |

Both build with no errors and no warnings. Tests: `cd translation && cargo test`
(15 tests, all pass, none `#[ignore]`d). Nothing in `c_src/` was modified; the
only addition there is the generated `c_src/build/` directory.

## Input classes enumerated from the C source

`main` is `char c = getchar(); driver(c);` — there is not a single `if` or early
`return` in the program. All branching lives inside the glibc `<ctype.h>`
macros, so the input classes are the table regions those macros distinguish,
plus the EOF path in `getchar`:

* EOF — empty stdin (also: `/dev/null`, closed fd, zero-length pipe)
* every byte `0x00..=0xFF` as the first byte of stdin (exhaustive)
* region boundaries: `0x00`, `0x08/0x09` (blank), `0x0A..0x0D` (space, not
  blank), `0x1F/0x20` (printing), `0x20/0x21` (graphical), `/0-9:`,
  `@A-Z[`, `` `a-z{ ``, `F/G`, `f/g` (hex cut-offs), all punctuation, `0x7E/0x7F`
* `0x80..=0xFF`, which sign-extend to negative `int` arguments
* inputs longer than one byte, since only the first byte is ever read
  (`Zebra\n`, leading space/tab/newline, multi-line, UTF-8 lead byte, 256 KiB)

## Mismatches found

**None.** The exhaustive sweep — all 256 first-byte values plus EOF, comparing
stdout, stderr and exit status byte for byte — produced zero differences before
any change was made to the Rust source. No fix to `translation/` was required,
so this section has nothing to list. The Rust source is unchanged from the state
it was received in; the work added was `translation/tests/differential.rs` and
this file.

## Hazards checked, and why each one does not diverge

These are the places a translation of this program is expected to break. Each
was confirmed non-divergent by running both binaries, and — where the fault is
observable at all — confirmed to be *caught* by the test suite via mutation
testing (fault injected, `cargo test` failed, fault reverted).

1. **Classification returns raw mask bits, not `0`/`1`.** glibc implements
   `isalpha(c)` as `(*__ctype_b_loc())[c] & _ISalpha`, and `printf("%d", ...)`
   prints that masked value. For `a` the C program prints `alphabetic: 1024`,
   `lowercase: 512`, `alphanumeric: 8`, `hexadecimal: 4096`,
   `graphical: 32768`, `printing: 16384` — not `1`. `translation/src/ctype.rs`
   reproduces the `_ISbit` constants and returns the masked bits.
   *Mutation:* normalising to `0`/`1` fails 10 of 15 tests.

2. **`printf("%c", ...)` writes a raw byte, not a UTF-8 character.** The value is
   converted to `unsigned char`, so byte `0xC8` prints back as the single byte
   `0xC8` and EOF prints as `0xFF`. Building the line as a Rust `String` would
   emit two bytes for those. The translation assembles output as `Vec<u8>`.
   *Mutation:* UTF-8-encoding the `%c` value fails 4 tests.

3. **`char` is signed on this target, so bytes `0x80..=0xFF` reach the ctype
   functions as `-128..=-1`.** glibc's tables are addressed from `-128`, and in
   the `"C"` locale that whole region is zero with identity case conversion, so
   these bytes classify as nothing and echo themselves back. Indexing the
   positive region instead would report them as ASCII.
   *Mutation:* masking the index with `0x7F` fails 4 tests.

4. **EOF is `-1`, and `char c = getchar()` truncates it to `-1`.** Byte `0xFF`
   truncates to `-1` as well, so `printf 'a'`-free empty input and `\xFF` input
   must produce *identical* output — they do, in both programs.
   *Mutation:* mapping EOF to `0` fails 2 tests.

5. **`tolower`/`toupper` are table lookups, not case-mapping of a `char`.**
   *Mutation:* swapping the two tables fails 6 tests.

6. **Only the first byte of stdin is consumed.** `getchar` uses buffered stdio
   and may read ahead, but the program exits without reading again, so trailing
   input — including newlines and 256 KiB of data — changes nothing. Both
   programs agree, and neither leaves anything on stdout or stderr about the
   unread remainder.

7. **Exit status is `0` for every input.** `main` has no `return`, which in C99
   implies `return 0`. There is no error path in this program: no input makes it
   exit non-zero, and stderr is empty for every input tested. A test asserting
   only stdout would still have been sufficient here, but all three are asserted
   as required.

## One mutation that is *not* observable

Changing the translation to treat the byte as **unsigned** (`c as u8 as i32`,
indexing `128..=255` instead of `-128..=-1`) leaves all 15 tests passing, and
that is correct rather than a gap in the tests: in the `"C"` locale glibc's
class table is zero and its case tables are the identity in *both* the
`-128..=-1` and `128..=255` regions, and `printf("%c")` reduces both `-56` and
`200` to the same byte `0xC8`. The two readings are observationally identical
for this program on this locale. The signed reading is kept because it is what
the C actually does; the exhaustive `0x00..=0xFF` + EOF sweep is what
establishes the equivalence, rather than an assumption about the tables.
