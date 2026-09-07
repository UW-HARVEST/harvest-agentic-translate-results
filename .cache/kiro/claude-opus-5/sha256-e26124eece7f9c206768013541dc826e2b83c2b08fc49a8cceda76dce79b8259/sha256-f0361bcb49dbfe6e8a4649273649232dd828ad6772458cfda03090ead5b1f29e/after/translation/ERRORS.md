# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** error enum,
no `RETURN_ERROR`-style macro, no `assert`, and no explicit null/range
validation. Its entire rejection surface consists of:

* one sentinel return (`return -1` in `find_value_in_buffer`, line 39),
* the guard branches that *silently* skip work (`if (result != NULL)`,
  `if (b != 0)`, `if (pos >= 0)`, `if (direct_search != NULL)`, `for (i = 0; i < size; ...)`),
* implicit rejections that fall out of undefined/implementation-defined
  behaviour the code deliberately exercises (out-of-range `double`→`int`,
  signed overflow, `%` with negative operands).

Every row below is a distinct condition the C source actually distinguishes.
Each has a differential test in `tests/differential.rs` that calls **both**
`.so`s and asserts the *same* sentinel/value, not merely "both failed".

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|-----------------------------------------|-------------------|
| 1  | `find_value_in_buffer` | `memchr` returns `NULL` — byte absent from the first `size` bytes (line 36 false, line 39) | `-1` |
| 2  | `find_value_in_buffer` | `size == 0` (empty range; `memchr` must not deref, always `NULL`) | `-1` |
| 3  | `find_value_in_buffer` | `size == 0` **and** `buffer == NULL` (null pointer with zero length; C never dereferences) | `-1` |
| 4  | `find_value_in_buffer` | `search_val` outside `char` range on the high side, e.g. `0x141` — narrowed by `(char)search_val` to `0x41`, so it *matches* byte `0x41` instead of being rejected | index of byte `0x41`, else `-1` |
| 5  | `find_value_in_buffer` | negative `search_val`, e.g. `-1` — `(char)(-1) = -1`, promoted to `int` then compared as `(unsigned char)0xFF` | index of byte `0xFF`, else `-1` |
| 6  | `find_value_in_buffer` | `search_val` whose low byte is `0x80..0xFF` (sign-extension trap: `char` is signed on x86-64) | index of that byte, else `-1` |
| 7  | `find_value_in_buffer` | `search_val == 0` searching a buffer with no `NUL` byte | `-1` |
| 8  | `find_value_in_buffer` | `size` larger than the number of matching bytes but needle only present *past* `size` (oversized/undersized length boundary, one step past a match) | `-1` |
| 9  | `find_value_in_buffer` | `search_val = INT_MIN` / `INT_MAX` (extreme enum-style out-of-range int across FFI) | index of low byte `0x00` / `0xFF`, else `-1` |
| 10 | `create_numeric_buffer` | `size == 0` — loop body never runs, buffer untouched | no writes, `void` |
| 11 | `create_numeric_buffer` | `size < 0` (e.g. `-1`, `INT_MIN`) — `i < size` false immediately, **no** write and **no** crash | no writes, `void` |
| 12 | `create_numeric_buffer` | `size == 0` with `buffer == NULL` | no writes, no deref |
| 13 | `create_numeric_buffer` | `seed < 0` — `(seed + i*7) % 256` truncates toward zero, so stored bytes are **negative** | negative `char` values, not `seed mod 256` |
| 14 | `create_numeric_buffer` | `seed = INT_MAX` with `size` large enough that `seed + i*7` overflows `int` (signed-overflow UB; wraps in practice) | wrapped byte sequence |
| 15 | `create_numeric_buffer` | `seed = INT_MIN` (most negative, `%` on negative dividend) | negative byte sequence |
| 16 | `calculate_with_doubles` | `b == 0` — division guarded out (line 57 false), but `result *= pow(...)` still executes on `0.0` | `0.0 * pow(10, c%10)` = `0.0` (never NaN/inf) |
| 17 | `calculate_with_doubles` | `c < 0` — `c % 10` truncates toward zero giving a **negative** exponent | `a/b * 10^(negative)` |
| 18 | `calculate_with_doubles` | `c = INT_MIN` (`INT_MIN % 10 == -8`) | `a/b * 1e-8` |
| 19 | `calculate_with_doubles` | `a = INT_MIN, b = -1` — would overflow in integer math, but both are converted to `double` first | `2147483648.0 * 10^(c%10)` (no trap) |
| 20 | `calculate_with_doubles` | `b == 0` **and** `a == 0` and `c` making `pow` huge (`c%10 == 9`) | `0.0` |
| 21 | `convert_double_to_int` | value `> INT_MAX` (e.g. `2147483648.0`, `1e18`, `DBL_MAX`) — out-of-range cast UB, x86-64 `cvttsd2si` | `INT_MIN` (`-2147483648`) |
| 22 | `convert_double_to_int` | value `< INT_MIN` (e.g. `-2147483649.0`, `-1e18`, `-DBL_MAX`) | `INT_MIN` |
| 23 | `convert_double_to_int` | `+INFINITY` (line 136/140 exercises this) | `INT_MIN` |
| 24 | `convert_double_to_int` | `-INFINITY` | `INT_MIN` |
| 25 | `convert_double_to_int` | `NAN` (line 137/144 exercises this) | `INT_MIN` |
| 26 | `convert_double_to_int` | negative-signed NaN and non-canonical NaN payloads (bit patterns `0xFFF8..`, `0x7FF0..1`) | `INT_MIN` |
| 27 | `convert_double_to_int` | exactly `INT_MAX + 0.5` (`2147483647.5`) — truncates *into* range, must **not** be rejected | `2147483647` |
| 28 | `convert_double_to_int` | exactly `INT_MIN - 0.5` (`-2147483648.5`) — truncates *into* range | `-2147483648` |
| 29 | `convert_double_to_int` | one step past the valid range: `nextafter(2147483648.0, 0)` vs `2147483648.0` | in-range value vs `INT_MIN` |
| 30 | `convert_double_to_int` | `-0.0` and subnormals (`5e-324`) — truncate to zero, sign dropped | `0` |
| 31 | `process_negation` | `INT_MIN` / `INT_MAX` (no overflow path; `!!` is total) | `1` |
| 32 | `process_negation` | `0` — the only input mapping to `0` | `0` |
| 33 | `doubleneg` | `pos < 0` for a search value (line 112 false) → would print `"Value %d not found"` | **provably unreachable at `size = 256`** — see note below; the `-1` sentinel is covered by rows 1–2 |
| 34 | `doubleneg` | `direct_search == NULL` (line 121 false) — byte `100` absent from the generated buffer | **provably unreachable at `size = 256`** — see note below; the NULL path is covered by rows 1–2 |
| 35 | `doubleneg` | `param1 = INT_MIN`/`INT_MAX` → signed overflow inside `create_numeric_buffer` and `(param1 + i*param2) % 256` (line 129) | wrapped values, no trap |
| 36 | `doubleneg` | `param2 = 0` → `calculate_with_doubles` takes the `b == 0` branch **and** `search_byte` is constant `param1 % 256` for all 10 iterations | deterministic accumulation |
| 37 | `doubleneg` | `param2 = INT_MIN, param1 = INT_MAX` → `param1 + i*param2` overflows repeatedly (UB) | wrapped `search_byte`, same stdout |
| 38 | `doubleneg` | `param3` such that `calculate_with_doubles` returns a value `> INT_MAX` → `converted_int == INT_MIN`, then `INT_MIN % 1000 == -648` (negative remainder) | `result` decreases by 648 |
| 39 | `doubleneg` | all params `0` → every `!!` is `0`, `b == 0`, buffer is `0,7,14,...`, byte `100` absent | fully deterministic baseline |
| 40 | `doubleneg` | `result` accumulation overflowing `int` (signed-overflow UB via `+= (converted_int % 1000)` etc.) | wrapped `int`, printed with `%d` |

## Generic FFI boundary cases (covered even though not table rows)

| case | covered by |
|------|-----------|
| null pointer + zero length | rows 3, 12 |
| zero length | rows 2, 10 |
| oversized / undersized length | row 8 |
| one step past a documented valid range | rows 27, 28, 29 |
| out-of-range "enum" values passed across FFI (C `int` accepts any value; `search_val` is the only such parameter and has no valid-variant set) | rows 4, 5, 6, 9 |
| `INT_MIN`/`INT_MAX` for every `int` parameter | rows 9, 11, 14, 15, 18, 19, 31, 35, 37 |

## Note on rows 33 and 34 — two rejection branches are dead inside `doubleneg`

`create_numeric_buffer(buffer, 256, param1)` stores `(char)((param1 + 7i) % 256)`
for `i` in `0..256`. Reading a byte back as `unsigned char` yields
`(param1 + 7i) mod 256` for every `param1` (C's truncating `%` and the
`int`→`char` narrowing cancel out modulo 256). Since `gcd(7, 256) == 1`, the 256
iterations enumerate **all** 256 byte values. Therefore, inside `doubleneg`:

* `memchr(buffer, 100, 256)` can never be `NULL`, so `if (direct_search != NULL)`
  is always taken (row 34), and
* every `search_values[i]` — each of the form `x % 256`, hence in `[-255, 255]`,
  hence narrowed to some byte the buffer contains — is always found, so
  `if (pos >= 0)` is always taken (row 33).

This was **verified against the compiled C library**, not assumed: a sweep over
seeds `-100000..=100000` plus `INT_MIN`/`INT_MAX` found 0 misses out of 800,004
searches and 0 seeds where byte `100` was absent. The tests
`err_row_33_*` / `err_row_34_*` assert that property against the C build and then
exercise the `-1`/NULL sentinel where it *is* reachable — directly on the
`find_value_in_buffer` export.

## Row status

All 40 rows have a passing differential test:

* rows 1–32 → `tests/phase_c_errors.rs` (libtest harness)
* rows 33–40 → `tests/phase_d_pipeline.rs` (`harness = false`, sequential,
  because comparing `doubleneg`'s stdout needs exclusive use of fd 1)
* generic boundary sweeps → `err_generic_*` in `tests/phase_c_errors.rs`
* `int`-truncation / overflow paths that need multi-gigabyte buffers →
  `tests/phase_e_large.rs`
