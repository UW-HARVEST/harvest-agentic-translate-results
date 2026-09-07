# ERRORS.md — Phase A error-surface table

Mechanically derived by grepping `c_src/` for every rejection mechanism:

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|exit|abort|if *\(|< *0|>|<' c_src/src/driver.c c_src/include/driver.h
```

Result: the C source contains **zero** explicit rejections — no `return`
statement (the function is `void`), no `assert`, no error enum, no null check,
no range check, and no min/max constant. The entire body is:

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}
```

Consequently the error surface is **not** a set of returned error codes; it is
the set of *fault / sentinel behaviours* the C binary actually exhibits when
handed invalid input. Those are enumerated below and each is asserted
differentially by comparing the **termination status (signal number or exit
code) and the stdout bytes** of a forked child that calls the C `.so` against
one that calls the Rust `.so`. "Both failed somehow" is not accepted — the
tests compare the exact signal number.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `driver` | `s1 == NULL`, `s2 == NULL` | SIGSEGV (11), no stdout | `err_01_both_null` | [x] |
| 2 | `driver` | `s1 == ""` (valid, empty), `s2 == NULL` | SIGSEGV (11), no stdout — glibc `strcspn` dereferences `s2` to build its reject set *before* testing `s1`, so an empty `s1` does **not** short-circuit | `err_02_empty_s1_null_s2` | [x] |
| 3 | `driver` | `s1 == NULL`, `s2 == ""` (valid, empty) | SIGSEGV (11), no stdout | `err_03_null_s1_empty_s2` | [x] |
| 4 | `driver` | `s1 == "abc"` (valid), `s2 == NULL` | SIGSEGV (11), no stdout | `err_04_valid_s1_null_s2` | [x] |
| 5 | `driver` | `s1 == NULL`, `s2 == "abc"` (valid) | SIGSEGV (11), no stdout | `err_05_null_s1_valid_s2` | [x] |
| 6 | `driver` | `s1` = non-NULL but unmapped address (`0x1`), `s2` valid | SIGSEGV (11), no stdout | `err_06_unmapped_s1` | [x] |
| 7 | `driver` | `s1` valid, `s2` = non-NULL but unmapped address (`0x1`) | SIGSEGV (11), no stdout | `err_07_unmapped_s2` | [x] |
| 8 | `driver` | both pointers unmapped (`0x1`, `0x2`) | SIGSEGV (11), no stdout | `err_08_both_unmapped` | [x] |
| 9 | `driver` | `s1` = unterminated buffer whose bytes run off the end of the last mapped page (no NUL before the guard page); `s2` valid and non-matching | SIGSEGV (11), no stdout — `strcspn` scans past the mapping | `err_09_unterminated_s1_runs_off_page` | [x] |
| 10 | `driver` | `s2` = unterminated buffer running off the end of the last mapped page; `s1` valid, `s1[0]` not in `s2`'s mapped bytes | SIGSEGV (11), no stdout | `err_10_unterminated_s2_runs_off_page` | [x] |
| 11 | `driver` | `s1` = misaligned pointer into the middle of a mapping, unterminated up to the guard page | SIGSEGV (11), no stdout | `err_11_unterminated_s1_misaligned` | [x] |

## Generic boundaries additionally covered (not rows above)

These are the "every C API has them" boundaries required by Phase C. None of
them is a *rejection* in this library (the C accepts them all and prints a
number), so they are asserted as output equality rather than as errors.

| condition | C behaviour | test |
|-----------|-------------|------|
| zero length: `s1 == ""`, `s2 == ""` | prints `0\n` | `bnd_empty_empty` |
| zero length `s1`, non-empty `s2` | prints `0\n` | `bnd_empty_s1` |
| zero length `s2`, non-empty `s1` | prints `strlen(s1)\n` (empty reject set) | `bnd_empty_s2` |
| oversized length: `s1` of 1 MiB with no match | prints `1048576\n` | `bnd_oversized_no_match` |
| oversized length: `s2` containing all 255 non-NUL bytes | prints `0\n` for any non-empty `s1` | `bnd_s2_all_255_bytes` |
| one byte past the ASCII range: bytes `0x80..=0xFF` in `s1`/`s2` (sign of `char` is implementation-defined; a hand-rolled comparison on `i8` vs `u8` is the classic divergence) | compares raw bytes | `bnd_high_bytes` |
| embedded NUL in the middle of the Rust-side buffer | both stop at the first NUL | `bnd_embedded_nul` |
| `s1` aliases `s2` (same pointer) | prints `0\n` unless `s1` is empty | `bnd_aliased_pointers` |
| `s1` is a suffix pointer into `s2`'s buffer | overlapping-buffer read, well defined | `bnd_overlapping_buffers` |
| result magnitude spans printf digit widths: 0, 1, 9, 10, 99, 100, 999, 1000, 65535, 65536 | `%zu` prints with no padding, no `+`, no thousands separator | `bnd_printf_digit_widths` |

## Out-of-range enum values across the FFI boundary

`driver` takes **no enum, no flag, no mode, and no integer parameter** — its
only two parameters are `const char *`. There is therefore no enum whose
domain can be exceeded, and no `switch` in the C source to fall through.
This row of the Phase C checklist is **vacuous by construction**, and
`tests/differential.rs::phase_c_no_enum_parameters_exist` documents that with
a compile-time check of the FFI signature
(`unsafe extern "C" fn(*const c_char, *const c_char)`) so the claim is
re-verified rather than merely asserted in prose.
