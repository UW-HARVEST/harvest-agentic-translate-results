# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every error-shaped construct was grepped out of the C source:

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit\(|abort|if *\(|switch|#if' \
      c_src/src/driver.c c_src/include/driver.h
c_src/include/driver.h:24:#ifndef DRIVER_H_        # include guard only
```

Result: `driver.c` contains **zero** `return` statements, **zero** `assert`s,
**zero** `if`/`switch`/range checks, **zero** null checks, **zero** error
enums/macros, and **no** min/max constants. `driver` is `void`-returning and
takes a by-value `char`, so there is no pointer to be null and no status to
return. There is therefore **no explicit rejection path in this library**.

The real error/edge surface is consequently:

1. the implicit range handling glibc performs on the value it is handed
   (`__ctype_b_loc()[c]`, and the real `tolower`/`toupper` functions, which the
   C `.so` imports — see `nm -D --undefined-only`, `U tolower@GLIBC_2.2.5`);
2. what happens when a caller pushes a value that is not a valid `char` across
   the FFI boundary (C prototypes are promoted, so any `int` is accepted by an
   external caller — the analogue of an out-of-range enum value).

Both are enumerated below and every row is differentially tested. "Expected C
result" is whatever the C `.so` actually emits; the test asserts the Rust `.so`
emits the identical bytes, so no row encodes a guess.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | `c = 0` (NUL — the C-string terminator, degenerate "empty" char) | no rejection; 14 lines; classifiers report the `_IScntrl` bit (`control: 2`), `to lower`/`to upper` emit the NUL byte `0x00` |
| 2 | `driver` | `c = 127` (`0x7F`, DEL — top of the positive `char` range) | no rejection; `control: 2`; identity case conversion |
| 3 | `driver` | `c = -1` (`0xFF`) — bit-identical to `EOF`, the sentinel every ctype function special-cases | no rejection; all classifier bits 0; `tolower(-1)`/`toupper(-1)` return `-1`, and `printf("%c", -1)` emits byte `0xFF` |
| 4 | `driver` | `c = -128` (`0x80`) — most negative `char`, lowest legal ctype table index | no rejection; all classifier bits 0; case conversion is identity, `%c` emits byte `0x80` |
| 5 | `driver` | `c` in `-128..=-2` (`0x80..=0xFE`) — every remaining negative index, i.e. one step past the *documented* `unsigned char` domain of `<ctype.h>` | no rejection; all 12 classifier bits 0; both conversions identity |
| 6 | `driver` | out-of-range argument: caller passes an `int` outside `-128..=127` (e.g. `256`, `-129`, `0x1234`, `INT_MAX`, `INT_MIN`) across the FFI boundary — legal for a promoted C prototype, no valid `char` "variant" | callee truncates to the low 8 bits; result is identical to the corresponding in-range `char`, no trap/error |
| 7 | `driver` | `c = 32` (space) vs `c = 9` (tab) — the only two values where `isspace`/`isblank` disagree, the classic off-by-one classification boundary | `space: 8192` for both; `blank: 1` for both; but `printing: 16384` only for `32`, and `graphical` is 0 for both |
| 8 | `driver` | boundary values one step outside each classifier's valid range: `'0'-1`=`/`, `'9'+1`=`:`, `'A'-1`=`@`, `'Z'+1`=`[`, `'a'-1`=`` ` ``, `'z'+1`=`{`, `'f'+1`=`g`, `'F'+1`=`G` | no rejection; the adjacent value must report the *punct* bits, not the digit/alpha/xdigit bits — verifies no fencepost error in the classification tables |
| 9 | `driver` | repeated invocation (state leak / `setlocale(LC_ALL,"C")` re-entry): call `driver` many times in one process, interleaved between the C and Rust `.so` | no rejection; output for a given `c` is identical on every call and unaffected by the other library having run first |

Check-off status (all rows verified by `tests/differential.rs`):

- [x] 1  `error_row_01_nul`
- [x] 2  `error_row_02_del_127`
- [x] 3  `error_row_03_eof_minus_one`
- [x] 4  `error_row_04_most_negative`
- [x] 5  `error_row_05_all_negative_chars`
- [x] 6  `error_row_06_out_of_range_int_arg`
- [x] 7  `error_row_07_space_vs_blank`
- [x] 8  `error_row_08_one_past_each_class_boundary`
- [x] 9  `error_row_09_repeat_and_interleave`

## Divergence found and fixed (row 6)

Row 6 was the only failing row and it was a real translation bug, not a test
artifact:

* **Symptom.** Calling the export with `256` (a promoted `int`, low 8 bits `0`)
  gave, from the C `.so`, exactly the `c == 0` output (`control: 2`, everything
  else `0`). The Rust `.so` instead printed `alphabetic: 1024`, `digit: 2048`,
  `space: 8192`, `printing: 16384` and `to lower: 0x80` — values read from
  past the end of the ctype table.
* **Cause.** The export was declared `extern "C" fn driver(c: c_char)`. rustc
  tags an `i8` C-ABI parameter `signext`, so LLVM assumed the register already
  held a sign-extended `char`, kept the full `256`, proved `c` was in
  `-128..=127`, elided the bounds check on `CTYPE_B[c + 128]`, and read index
  `384` of a 384-element table. gcc's `char`-taking callee, by contrast, reads
  only the low 8 bits of the argument register.
* **Fix.** The export now takes `c_int` and truncates with `arg as c_char`,
  reproducing the C's truncation; `ctype_index` additionally masks with `0x1FF`
  so an out-of-bounds table read is impossible by construction. Both changes
  are in `src/lib.rs`; `c_src` was not touched.
* **ABI check.** `scripts/compare_binaries.sh` compiles a gcc consumer against
  the unmodified `c_src/include/driver.h` (declared prototype `void
  driver(char)`), links it against each `.so` in turn, and diffs stdout over
  all 256 `char` values plus values the C compiler must narrow itself — the
  `c_int` parameter is ABI-indistinguishable for real callers.

## Harness credibility (mutation checks)

The suite was confirmed capable of failing, not just of passing:

| mutation applied to `src/lib.rs` | result |
|----------------------------------|--------|
| `IS_CNTRL` mask `2` → `6` | 9 of 25 rows FAILED |
| one `CTYPE_TOLOWER` entry `97` → `98` | `compare_binaries.sh` FAILED (exit 1) |

Both mutations were reverted and the suite returned to 25/25 passing.
