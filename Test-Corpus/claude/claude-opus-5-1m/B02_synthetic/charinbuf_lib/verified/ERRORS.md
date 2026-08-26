# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically: every `return` statement, every `if (...)` guard, every
null check, every range check and every `default:` label in the single C
translation unit was enumerated (see the greps below), then one row was written
per *distinct* rejection branch. There are **no** `assert`s, no error enums and
no `RETURN_ERROR`-style macros in this library; rejection is expressed as
sentinel return values (`0`, `1`, `-1`, `NULL`) plus a diagnostic on `stdout`.

```
$ grep -n "return" c_src/src/lib.c                   -> 17 return sites
$ grep -nE "if \(|assert|default:|NULL|MAX|MIN" ...  -> 8 guards, 1 default:
```

Constants that bound the input domain: `UINT16_MAX` = 65535 (an `int`-typed
macro, so the `value > UINT16_MAX` comparison happens in `int`). `INT_MIN` /
`INT_MAX` bound every `int` parameter across the FFI boundary.

Every row is asserted on BOTH shared objects, through `dlopen`/`dlsym` only, and
compares the returned sentinel **and** the bytes written to `stdout`. Every row
is checked off only because its test passes against both libraries — in the
debug *and* release profile (`./verify.sh`).

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|----------------------------------------------|-------------------|------|-----|
| 1  | `is_string_empty` | `str == NULL` (`if (!str) return 1;`, line 55) | returns `1` | `err_01_is_string_empty_null` | [x] |
| 2  | `is_string_empty` | `str` points at `'\0'` (empty string; falls through `if (*str)` to line 59) | returns `1` | `err_02_is_string_empty_empty` | [x] |
| 3  | `is_string_empty` | `*str != 0` (non-empty; line 57) — the accept branch, included so the reject/accept pair is pinned for all 255 possible first bytes | returns `0` | `err_03_is_string_empty_nonempty_all_bytes` | [x] |
| 4  | `find_char_in_buffer` | `buffer == NULL` (`if (!buffer) return NULL;`, line 63) — checked with `size` 0, small, and `SIZE_MAX`, so the guard must short-circuit before `memchr` | returns `NULL` | `err_04_find_char_null_buffer` | [x] |
| 5  | `find_char_in_buffer` | `target` absent from the first `size` bytes → `memchr` returns `NULL` (line 64) | returns `NULL` | `err_05_find_char_absent` | [x] |
| 6  | `find_char_in_buffer` | `size == 0` → `memchr` inspects nothing, so *every* target is rejected, even one present at `buffer[0]` | returns `NULL` | `err_06_find_char_zero_size` | [x] |
| 7  | `find_char_in_buffer` | `size` shorter than the position of an otherwise-present `target` (truncated search window) | returns `NULL` | `err_07_find_char_size_truncates_match` | [x] |
| 8  | `create_buffer` | `initial == NULL` (`if (!initial) return NULL;`, line 68) | returns `NULL` | `err_08_create_buffer_null` | [x] |
| 9  | `create_buffer` | `malloc` returns `NULL` → `if (buffer)` false, the `strcpy` is skipped and the NULL is returned unchanged (lines 71–77) | returns `NULL`, no copy | `err_09_create_buffer_malloc_failure` | [x] |
| 10 | `validate_uint16_range` | `value < 0` (line 81); tested at `-1`, `INT_MIN`, and randoms | returns `0` | `err_10_validate_negative` | [x] |
| 11 | `validate_uint16_range` | `value > 65535` (line 82); tested at `65536`, `INT_MAX`, and randoms | returns `0` | `err_11_validate_above_max` | [x] |
| 12 | `validate_uint16_range` | `0 <= value <= 65535` (line 83) — accept branch, pins both boundary values and one step past each | returns `1` | `err_12_validate_boundaries` | [x] |
| 13 | `apply_operation` | `op == NULL` (`if (!op) return -1;`, line 87) | returns `-1` | `err_13_apply_operation_null` | [x] |
| 14 | `apply_operation` | `op` non-NULL but the callee itself returns `-1` — proves the `-1` sentinel is **not** distinguishable from a legitimate `-1`, and that Rust's `Option<fn>` niche does not turn a valid pointer into `None` | returns `-1` from the callee | `err_14_apply_operation_callee_returns_minus_one` | [x] |
| 15 | `charinbuf` | `mode` matches no `case` → `default:` (line 204): prints `Invalid mode: %d\n` | returns `-1` | `err_15_charinbuf_invalid_mode` | [x] |
| 16 | `charinbuf` | `mode == 0` with `value` outside `[0, 65535]`: prints `Value %d is out of range for uint16_t\n` then the `UINT16_MAX` line | returns `-1` | `err_16_charinbuf_mode0_out_of_range` | [x] |
| 17 | `charinbuf` | `mode == 2` with a failing `malloc`: prints `Failed to allocate buffer\n` | returns `-1` | `err_17_charinbuf_mode2_alloc_failure` | [x] |
| 18 | `charinbuf` | `mode == 4` with a failing `malloc`: the whole `if (buffer)` block is skipped, so **nothing** is printed after the mode banner and `result` keeps its initial `0` — note `0`, **not** `-1`, unlike mode 2 | returns `0` | `err_18_charinbuf_mode4_alloc_failure` | [x] |
| 19 | `charinbuf` | `mode == 1` dead branch: `is_string_empty("Hello, World!")` truthy → `Non-empty string check failed!\n` and `result` not incremented by 10 | unreachable (see note) | `err_19_charinbuf_mode1_dead_branch` | [x] |

### Generic FFI-boundary boundaries (also covered, beyond the table)

| #  | condition | test | [x] |
|----|-----------|------|-----|
| 20 | out-of-range "enum" values for `mode`: `-1`, `5`, `6`, `INT_MIN`, `INT_MAX`, and randoms outside `0..=4`. `mode` is a plain `int`, so C accepts any of the 2^32 values and takes `default:` | `err_15`, `err_20_charinbuf_mode_exhaustive_boundaries` | [x] |
| 21 | one step past each documented range edge: `value` ∈ {`-1`,`0`,`1`,`65534`,`65535`,`65536`} and `mode` ∈ {`-1`,`4`,`5`} | `err_20`, `err_12` | [x] |
| 22 | `INT_MIN`/`INT_MAX` in every `int` parameter of `charinbuf` (`value`, `opt1`, `opt2`) — full 7×7×7 cross-product per mode, exercising signed wraparound in mode 3 | `err_21_charinbuf_extreme_int_params` | [x] |
| 23 | oversized `size` for `find_char_in_buffer` (`SIZE_MAX`, `usize::MAX/2`) with a NULL buffer | `err_04_find_char_null_buffer` | [x] |
| 24 | `target == '\0'`: the NUL is findable when it lies inside `size`, so this is an *accept*, not a reject — pinned to catch a `strchr`-style mistranslation | `err_22_find_char_nul_target` | [x] |
| 25 | `target` with the high bit set (`0x80..=0xFF`, negative `char` on x86-64). `memchr` converts its `int` to `unsigned char`, so C's sign-extension and Rust's `target as u8` zero-extension must agree — all 128 values, plus a check that the low-7-bit twin does **not** match | `err_23_find_char_high_bit_target` | [x] |
| 26 | `create_buffer("")` — a 1-byte `malloc` holding just the terminator; the smallest non-rejected input | `err_24_create_buffer_empty_string` | [x] |

## How rows 9, 17 and 18 are actually reached

These three branches require `malloc` to fail for a 1–38-byte request. That
cannot be provoked in the test process itself, and because the `.so`s are
`dlopen`ed into an already-running harness, `LD_PRELOAD` interposition is not
available either. Rather than document them as untestable, `tests/phase_c_oom.rs`
reaches them for real:

* the parent spawns the test binary again, once per library, via
  `CHARINBUF_OOM_LIB` / `CHARINBUF_OOM_CALL`;
* the child `dlopen`s exactly one library and forces its stdout buffer to be
  allocated **first**, while allocation still works;
* it then caps the address space with `RLIMIT_AS` at the current mapping size
  plus 256 KiB and drains every remaining free chunk (1 MiB down to 16 B) until
  `malloc` returns `NULL`;
* it makes the one call under test, reports the return value with `write(2)`
  (never `format!`, which would allocate) and `_exit`s;
* the parent compares the two children's post-`#BEGIN` stdout byte-for-byte and
  asserts the concrete sentinel.

Observed, identically from both libraries:

```
create_buffer -> NULL                                        (row 9)
charinbuf(2)  -> "Mode 2: ...\nFailed to allocate buffer\n", -1   (row 17)
charinbuf(4)  -> "Mode 4: Using memchr to find character\n",   0   (row 18)
```

Row 18's `0` versus row 17's `-1` is exactly the kind of asymmetry a
happy-path suite misses, and both implementations reproduce it.

The drain loop uses `black_box` and a `write_volatile` into each block on
purpose: LLVM treats `malloc` as a removable allocation function, so an
optimized build deletes an allocation whose result is only null-checked. Without
those, the release build drained nothing, the library's `malloc` succeeded, and
all three rows silently stopped testing the branch they exist for. This was
caught by running the suite under `--release` and is now guarded by asserting the
failure actually occurred (`assert_eq!(result, 0 / -1 / 0)`).

## Note on row 19

Row 19 is genuinely unreachable: `non_empty_string` is the fixed literal
`"Hello, World!"`, so `is_string_empty` on it is always `0`. The test pins the
reachable side exactly (`result == 10` and the three-line stdout) and asserts
that **neither** implementation ever prints the dead branch's message, which is
what a mistranslated condition would produce.
