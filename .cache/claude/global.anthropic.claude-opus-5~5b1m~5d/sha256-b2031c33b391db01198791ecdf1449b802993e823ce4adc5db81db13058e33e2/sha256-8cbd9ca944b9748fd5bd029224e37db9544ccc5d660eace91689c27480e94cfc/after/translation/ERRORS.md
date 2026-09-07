# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping `c_src/src/lib.c` for every rejection path:
every `return -1`, every early `return 0`, every null check, every
`== '\0'` emptiness check, every `<= 0` / `== 0` / `< sizeof(...)` range check,
and every loop-bound guard. There are no `assert`s, no error enums and no
`RETURN_ERROR`-style macros in this source.

`memchra2` is the sole exported entry point and it takes four `int`s by value,
so most of these rejection paths are reached only *internally*. They are still
listed one row per distinct rejection, and each row records how the row is
exercised differentially through the public FFI boundary (or why it is
statically unreachable — in which case the test asserts the corresponding
observable contribution to the return value, which is the only way an external
caller can witness the branch).

Legend for "how tested": every row is checked by calling BOTH the C `.so` and
the Rust `.so` `memchra2` export with the described inputs and asserting the
returned `int` is bit-identical.

| # | function | trigger (exact invalid input/condition) | expected C result | how tested | status |
|---|----------|------------------------------------------|-------------------|-----------|--------|
| 1 | `process_buffer` | `buffer == NULL` | `return -1` | statically unreachable from `memchra2` (always passed the on-stack `buffer[64]`); the Rust `Option<&[u8]>` `None` arm mirrors it. Verified by the `buf_sum > 0` contribution never being suppressed for any input. | [x] |
| 2 | `process_buffer` | `buffer != NULL` but `*buffer == '\0'` (empty string) | `return -1` | unreachable: `snprintf` always writes at least `"test"`, so `buffer[0] == 't'`. Asserted indirectly — `result` always includes `buf_sum % 256`. | [x] |
| 3 | `process_buffer` | `len == 0` (loop body never runs, non-NULL non-empty buffer) | `return 0`, so caller's `if (buf_sum > 0)` is FALSE and nothing is added | unreachable via `memchra2` (`strlen(buffer) >= 4`); the guard `buf_sum > 0` is exercised for all inputs. | [x] |
| 4 | `process_buffer` | first NUL encountered before `buffer + len` | loop `break`s early, partial sum returned | exercised for every input: `len == strlen(buffer)` so the two loop conditions terminate together. | [x] |
| 5 | `process_strings` | `strings == NULL` | `return 0` (0 matches ⇒ `+0`) | unreachable (literal array passed); Rust `None` arm mirrors. | [x] |
| 6 | `process_strings` | `count <= 0` (i.e. `0` or negative) | `return 0` | unreachable (`count == 4` literal); Rust `count <= 0` arm mirrors. | [x] |
| 7 | `process_strings` | an element `*i == NULL` | `continue` — element skipped, not counted | unreachable (all 4 literals non-NULL); Rust `None` arm mirrors. | [x] |
| 8 | `process_strings` | an element `**i == '\0'` (empty string) | `continue` — element skipped | unreachable (all 4 literals non-empty); Rust arm mirrors. | [x] |
| 9 | `process_strings` | element does not match `target` prefix (`strncmp != 0`, e.g. `"other"` vs `"test"`) | not counted; `matches` stays 3 of 4 ⇒ `+15` | exercised on every call — the `+15` constant is validated by every differential input. | [x] |
| 10 | `safe_sum_array` | `arr == NULL` | `return 0` | unreachable (`values[4]` on stack); Rust `None` arm mirrors. | [x] |
| 11 | `safe_sum_array` | `size == 0` | `return 0` | unreachable (`size == 4`); Rust `size == 0` arm mirrors. | [x] |
| 12 | `interpret_as_int` | `bytes == NULL` | `return 0` (so `result ^= 0`) | unreachable (`bytes[4]` on stack); Rust `None` arm mirrors. | [x] |
| 13 | `interpret_as_int` | `len < sizeof(int)` (i.e. `len` in `0..=3`) | `return 0` | unreachable (`len == 4` literal); Rust `len < 4` arm mirrors. | [x] |
| 14 | `count_occurrences` | `text == NULL` | `return 0` (`dash_count == 0` ⇒ `+0`) | unreachable; Rust `None` arm mirrors. | [x] |
| 15 | `count_occurrences` | `text != NULL` but `*text == '\0'` | `return 0` | unreachable (`buffer` starts with `'t'`); Rust arm mirrors. | [x] |
| 16 | `complex_iteration` | `data == NULL` | `return -1`, which is then **added** to `result` | unreachable; Rust `None` arm mirrors. | [x] |
| 17 | `complex_iteration` | `count == 0` | `return -1`, **added** to `result` | unreachable (`count == 4`); Rust arm mirrors. | [x] |
| 18 | `memchra` | `n == 0` | loop never runs, `return 0` | reached when `strlen(buffer) == 0` — unreachable; the non-zero path is covered everywhere. | [x] |
| 19 | `int_to_float_bits` / `memchra2` | `f` is NOT in the open range `(0.0f, 1000.0f)` — the `if` REJECTS the value and adds nothing | no `+= (int)f` | **reachable and tested**: `a <= 0` (sign bit ⇒ negative float, and `a == 0` ⇒ `f == 0.0`), `a >= 0x447A0000` (`f >= 1000.0`), `a == 0x7F800000` (`+inf`), `a` a NaN pattern (all comparisons false), `a == 0x7FFFFFFF` (NaN), `a == INT_MIN` (`-0.0`). | [x] |
| 20 | `memchra2` | `buf_sum <= 0` — the `if (buf_sum > 0)` REJECTS and adds nothing | no `+= buf_sum % 256` | `buf_sum` is a sum of printable ASCII, always `> 0`; the guard's TRUE branch is validated by every input. | [x] |
| 21 | `memchra2` (`snprintf`) | formatted text would exceed `sizeof(buffer) - 1 == 63` bytes ⇒ `snprintf` TRUNCATES | truncated, NUL-terminated at 63 | maximum possible length is `4 + 4*11 + 3 = 51 < 63`, so truncation is unreachable; boundary probed with `INT_MIN` in all four slots (longest output = 51 bytes). | [x] |

## Generic FFI boundary cases (covered even though not in the table)

The public ABI is `int memchra2(int, int, int, int)` — four by-value `int`s and
an `int` return. There are no pointer, length, or enum parameters, so:

| case | applicability | covered by |
|---|---|---|
| null pointer arguments | N/A — no pointer parameters | — |
| zero length / oversized length arguments | N/A — no length parameters | — |
| out-of-range enum value across FFI | N/A — no enum parameters | — |
| one step past a valid range | every `int` bit pattern is a valid input; tested at `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` in all four positions | `errors_boundary_ints` |
| values chosen to overflow internal signed arithmetic (`safe_sum_array`, `result +=`) | `int` overflow in C is UB but the compiled behaviour wraps; the Rust uses `wrapping_add` | `errors_overflow_wrap` |
| bit patterns that make the `int`→`float` pun produce subnormals / inf / NaN | reachable | `errors_float_pun_classes` |

---

## Results

All 21 rows plus the 3 generic-boundary tests PASS (`tests/phase_c_errors.rs`,
12 test functions) under all 3 feature invocations × both profiles.

Each row is asserted against a REFERENCE MODEL of the C (`fn expected`) as well
as against the C `.so`, so a row is only green when the C, the Rust and the
independently-derived model all produce the same `int` — this pins the *specific*
sentinel each guard contributes (`-1` vs `0` vs a real sum), rather than merely
"both failed somehow".

## Negative control

`mutation_check.sh` injects 10 targeted bugs into `src/lib.rs`, builds a `.so`
from each and re-runs Phases B+C against it. Result: **8 KILLED, 2 SURVIVED**.

| mutant | outcome |
|---|---|
| `endianness` (`from_le_bytes` → `from_be_bytes`) | KILLED |
| `float_lt_to_le` (`f < 1000.0` → `f <= 1000.0`) | KILLED |
| `dash_multiplier` (`*10` → `*11`) | KILLED |
| `complex_mask` (`& 0xFF` → `& 0xFFFF`) | KILLED |
| `strings_prefix` (`s.len() >= n` → `s.len() == n`) | KILLED |
| `sum_to_xor` (`sum +=` → `sum ^=`) | KILLED |
| `buf_mod` (`% 256` → `% 255`) | KILLED |
| `matches_mul` (`*5` → `*6`) | KILLED |
| `char_unsigned` (`byte as i8 as c_int` → `byte as c_int`) | SURVIVED — provably equivalent |
| `fzero_gt_to_ge` (`f > 0.0` → `f >= 0.0`) | SURVIVED — provably equivalent |

Both survivors are semantically equivalent rewrites, NOT test blind spots:

* `char_unsigned`: the `snprintf` buffer only ever contains `"test"`, ASCII
  digits and `'-'` — every byte is `<= 0x7F`, so signed and unsigned `char`
  widening coincide. (`as i8` remains the faithful spelling because C's `char`
  is signed on x86-64.) `row20_buf_sum_guard` asserts
  `s.bytes().all(|x| x.is_ascii_graphic())` over 20 000 random inputs.
* `fzero_gt_to_ge`: `f >= 0.0` additionally admits `f == ±0.0` (only when
  `a == 0` or `a == INT_MIN`), but then `result += (int)0.0` adds 0, leaving the
  return value unchanged. Those two `a` values are covered by
  `row10_float_pun_zeros`.
