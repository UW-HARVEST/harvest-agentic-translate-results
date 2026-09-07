# Phase A.3 — Configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

* **Cargo features / `#ifdef`s:** none. `c_src/src/lib.c` contains zero
  `#if`/`#ifdef` conditionals, and `translation/Cargo.toml` declares no
  `[features]` table. There is therefore exactly **one** build configuration;
  `--no-default-features` and `--all-features` are the same build (verified in
  Phase D).
* **Runtime option/mode flag:** `charinbuf`'s `mode` parameter is the only
  option selector — a 6-way `switch` (`0,1,2,3,4,default`). `value`, `opt1`,
  `opt2` are data, and which of them is *read at all* depends on `mode`
  (mode 0 reads `value`; mode 1/2/4 read none; mode 3 reads `value`, `opt1`,
  `opt2`; default reads `mode`).
* **Hidden state axis:** the file-scope `static int counter`. It persists across
  calls, `charinbuf` unconditionally zeroes it on entry, and mode 3 leaves a
  non-zero residue. So *call order* is a configuration axis in its own right.
* **Input shapes:** pointer nullness; string emptiness; `char` signedness
  (`0x00`, ASCII, `0x80..0xFF`); buffer length (0 / 1 / many); match position
  (first / middle / last / absent); `size` vs. match offset (short / exact /
  long); integer boundary values (`INT_MIN`, `-1`, `0`, `1`, `65535`, `65536`,
  `INT_MAX`) and overflow-inducing operands for `+ - *`.
* **Entry-point levels:** the low-level leaves (`increment_counter`,
  `decrement_counter`, `multiply_counter`, `reset_counter`, `is_string_empty`,
  `find_char_in_buffer`, `create_buffer`, `validate_uint16_range`), the
  mid-level dispatcher (`apply_operation`, which takes a function pointer), and
  the one-shot wrapper (`charinbuf`). All three levels are driven directly.
* **Observable output:** return value **and** the bytes written to `stdout`
  (only `charinbuf` prints). Both are compared for every `charinbuf` row.

## Configuration rows

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `reset_counter` | direct call, randomized `value` over full `i32` incl. `INT_MIN`/`INT_MAX`/0 | [x] |
| 2  | `increment_counter` | direct call from a known state; randomized addends incl. pairs that overflow `INT_MAX` | [x] |
| 3  | `decrement_counter` | direct call from a known state; randomized subtrahends incl. pairs that underflow `INT_MIN` | [x] |
| 4  | `multiply_counter` | direct call from a known state; randomized multiplicands incl. 0, -1, `INT_MIN`, overflowing products | [x] |
| 5  | all four counter fns | long randomized *interleaved* sequence (state accumulates across ~2000 ops), asserting the return of every single step — catches state-divergence, not just per-call arithmetic | [x] |
| 6  | counter fns after `charinbuf(3,..)` | state-coupling: mode 3 leaves a residue, then counter fns are called directly and must see the same residue in both libs | [x] |
| 7  | `validate_uint16_range` | boundary sweep `-2..2`, `65533..65537`, `INT_MIN`, `INT_MAX` + randomized full-range `i32` | [x] |
| 8  | `is_string_empty` | non-NULL, `*str != 0`: ASCII first byte | [x] |
| 9  | `is_string_empty` | non-NULL, first byte in `0x80..=0xFF` (signed-`char` sign-extension shape) | [x] |
| 10 | `is_string_empty` | non-NULL empty string `""` | [x] |
| 11 | `is_string_empty` | randomized strings, len 0..32, arbitrary bytes incl. high bytes | [x] |
| 12 | `create_buffer` | `""` (len 0 -> `malloc(1)`), result must be a NUL-terminated, `free`-able heap pointer | [x] |
| 13 | `create_buffer` | len 1, short, and long (~4 KiB) strings; contents compared byte-for-byte incl. terminator | [x] |
| 14 | `create_buffer` | randomized non-NUL byte content, len 0..512, incl. bytes `0x80..0xFF` | [x] |
| 15 | `find_char_in_buffer` | target present, match at offset 0 | [x] |
| 16 | `find_char_in_buffer` | target present, match in the middle | [x] |
| 17 | `find_char_in_buffer` | target present, match at the last byte within `size` | [x] |
| 18 | `find_char_in_buffer` | target present in the buffer but *beyond* `size` (short `size`) | [x] |
| 19 | `find_char_in_buffer` | `size == 0` on a non-NULL buffer | [x] |
| 20 | `find_char_in_buffer` | `target == '\0'` with `size` including / excluding the terminator | [x] |
| 21 | `find_char_in_buffer` | `target` negative (`0x80..=0xFF` as signed `char`), buffer containing high bytes | [x] |
| 22 | `find_char_in_buffer` | randomized haystack (len 0..256, arbitrary bytes) x randomized target x randomized `size <= len`; result compared as an *offset* so the two libs' different base addresses are handled | [x] |
| 23 | `apply_operation` | `op` = each of the four counter fns *of the same library*, randomized `value`, from randomized prior state | [x] |
| 24 | `apply_operation` | `op` = a callback defined in the *test* binary (pointer crossing FFI inward), incl. one returning `-1` | [x] |
| 25 | `charinbuf` mode 0 | `value` valid in-range (0, 1, 65535, randomized `0..=65535`) — return + stdout | [x] |
| 26 | `charinbuf` mode 0 | `value` out of range (`-1`, `65536`, `INT_MIN`, `INT_MAX`, randomized outside) — return + stdout | [x] |
| 27 | `charinbuf` mode 1 | no data inputs read; randomized `value`/`opt1`/`opt2` must not change output — return + stdout | [x] |
| 28 | `charinbuf` mode 2 | malloc/strlen/free path; randomized irrelevant args — return + stdout | [x] |
| 29 | `charinbuf` mode 3 | function-pointer pipeline, `value`/`opt1`/`opt2` small non-overflowing — return + stdout | [x] |
| 30 | `charinbuf` mode 3 | `opt2 == 0` (multiply collapses the counter to 0) and `opt1 == 0` — return + stdout | [x] |
| 31 | `charinbuf` mode 3 | overflow shapes: `value`/`opt1`/`opt2` at `INT_MIN`/`INT_MAX`/`-1`, and randomized full-range triples (signed wrap-around must match GCC's) — return + stdout | [x] |
| 32 | `charinbuf` mode 4 | memchr path, `'X'` found at its fixed offset; randomized irrelevant args — return + stdout | [x] |
| 33 | `charinbuf` default | `mode` = `-1`, `5`, `6`, `INT_MIN`, `INT_MAX`, randomized outside `0..=4` (out-of-range enum-style ints) — return + stdout | [x] |
| 34 | `charinbuf` | mode-sequence axis: randomized sequences of modes back-to-back in one process, comparing return + stdout at every step (the `counter = 0` reset on entry must scrub mode 3's residue) | [x] |
| 35 | binary driver | none — `c_src/CMakeLists.txt` builds only `add_library(... SHARED)`; there is no executable target and `translation/Cargo.toml` declares only `crate-type = ["cdylib"]`. No stdout-of-binary comparison applies. | [x] |
| 36 | `reset_counter` -> `charinbuf` (modes 0/1/2/4/default) -> `increment_counter` | the `counter = 0` on entry to `charinbuf` (lib.c:101) is executed for *every* mode, but modes 0/1/2/4/default never read the counter and mode 3 immediately overwrites it — so the reset is only observable by seeding the counter, calling a non-counter mode, and reading it back through a low-level entry point. Randomized seeds. | [x] |

Rows 1-34 and 36 map one-to-one onto the identically numbered tests in
`tests/phase_b_valid.rs` (`row01_…` … `row36b_…`). Each `charinbuf` row compares
the return value **and** the exact stdout bytes; each row using randomized
inputs uses a per-row fixed seed (`Rng::new(0xNNNN)`, SplitMix64) so failures
reproduce.

## Why row 36 exists

Row 36 was added after mutation testing, not from reading the source: commenting
out `counter = 0;` at the top of `charinbuf` (lib.c:101) left rows 1-35 all
green. Modes 0/1/2/4/default never read the counter, and mode 3 overwrites it
with `reset_counter` before reading, so the entry reset is only observable by
seeding the counter through `reset_counter`, calling a *non-counter* mode, and
reading the value back through a low-level entry point afterwards. This is the
interaction-between-entry-points blind spot the configuration table is meant to
close.

## Non-vacuity evidence

Byte comparison of an empty capture would pass trivially, so
`d0_capture_harness_is_not_vacuous` (`tests/phase_d_parity.rs`) asserts the full
expected stdout of all five modes as literal strings, and `diff_charinbuf`
refuses an empty or newline-less capture.

Beyond that, the suite was mutation-tested: 27 deliberate divergences were
injected into `translation/src` one at a time, each rebuilt and run against the
full suite. 24 were killed (including two that manifest as SIGSEGV inside the
specific Phase C test that exercises the dropped NULL guard). The three
survivors are semantics-preserving transformations, not gaps:

| surviving mutation | why it cannot diverge |
|--------------------|-----------------------|
| `target as c_int` -> `(target as u8) as c_int` in `find_char_in_buffer` | `memchr` converts its `c` argument to `unsigned char`, so sign-extension and zero-extension of the same byte are indistinguishable |
| `malloc(len + 1)` -> `malloc(len + 2)` in `create_buffer` | over-allocating by one byte changes no observable output (an *under*-allocation would, and is not equivalent) |
| `Buffer length: %zu` -> `%d` in mode 2 | the length is always 23; on x86-64 little-endian `%d` reads the low 32 bits of the same argument slot and prints the same digits |

The `src/` tree was restored and byte-compared against a pre-mutation backup
after every mutant (`diff -r` clean).
