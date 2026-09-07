# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** error enums,
no `assert`, no `RETURN_ERROR` macro, no `return NULL`, and no allocation. Its
entire rejection surface consists of:

* one sentinel return (`return -1` in `find_value_in_buffer`, line 39),
* the `if (result != NULL)` null-check that guards it (line 36),
* the `if (b != 0)` divide-by-zero guard in `calculate_with_doubles` (line 57),
* the loop guards `for (i = 0; i < size; i++)` (line 49) which silently reject
  non-positive `size`,
* the two `if`/`else` rejection branches inside `doubleneg` (lines 112–117 and
  121–125),
* the implementation-defined / undefined narrowing conversions
  (`(int)value` line 30, `(char)search_val` line 34, `(char)(...)` line 50)
  which "reject" out-of-range values by wrapping / producing the x86-64 integer
  indefinite value.

`grep -n 'return\|assert\|!=\|== 0\|NULL\|for (' c_src/src/lib.c` was used to
enumerate the rows below; every row has a differential test in
`tests/errors.rs`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | status |
|----|----------|----------------------------------------------|-------------------|--------|
| 1  | `find_value_in_buffer` | needle byte absent from `buffer[0..size]` → `memchr` returns `NULL` (line 36 false) | `-1` | [x] |
| 2  | `find_value_in_buffer` | `size == 0` (empty range; `memchr` never reads, returns `NULL`) | `-1` | [x] |
| 3  | `find_value_in_buffer` | `buffer == NULL` **and** `size == 0` (no dereference happens) | `-1` | [x] |
| 4  | `find_value_in_buffer` | `search_val` outside `unsigned char` range, e.g. `300` — `(char)300` truncates to `44` | index of byte `44`, else `-1` (never matches byte 300) | [x] |
| 5  | `find_value_in_buffer` | `search_val` negative, e.g. `-1` — `(char)-1 == -1`, `memchr` re-widens to `0xFF` | index of byte `255`, else `-1` | [x] |
| 6  | `find_value_in_buffer` | `search_val` one past the signed-`char` boundary (`128`) — narrows to `-128`, matches byte `0x80` | index of byte `128`, else `-1` | [x] |
| 7  | `find_value_in_buffer` | `search_val == INT_MIN` / `INT_MAX` (extreme int, low byte `0x00` / `0xFF`) | index of byte `0` / `255`, else `-1` | [x] |
| 8  | `find_value_in_buffer` | huge `size` beyond the real allocation is *not* checked by C — tested only with `size <= allocation` | (no check; not a rejection) | n/a |
| 9  | `create_numeric_buffer` | `size == 0` → loop body never runs, buffer untouched | no writes, `void` | [x] |
| 10 | `create_numeric_buffer` | `size < 0` (e.g. `-1`, `INT_MIN`) → `i < size` false immediately, buffer untouched | no writes, `void` | [x] |
| 11 | `create_numeric_buffer` | `buffer == NULL` with `size <= 0` (never dereferenced) | no writes, `void` | [x] |
| 12 | `create_numeric_buffer` | `seed` negative → `(seed + i*7) % 256` is **negative** (C `%` truncates toward zero) then narrowed by `(char)` | negative bytes, e.g. `seed=-1` → byte `0xFF` | [x] |
| 13 | `create_numeric_buffer` | `seed` near `INT_MAX` → `seed + i*7` signed overflow (UB, wraps on gcc) | wrapped bytes | [x] |
| 14 | `calculate_with_doubles` | `b == 0` → division skipped (line 57 false), `result` stays `0.0` | `0.0 * pow(10, c%10)` = `0.0` (never `inf`/`nan`) | [x] |
| 15 | `calculate_with_doubles` | `c` negative → `c % 10` negative → `pow(10, negative)` denormal-ish tiny factor | `a/b * 10^(c%10)` with negative exponent | [x] |
| 16 | `calculate_with_doubles` | `a == INT_MIN, b == -1` → done in `double`, no integer overflow trap | `2147483648.0 * 10^(c%10)` | [x] |
| 17 | `convert_double_to_int` | `value` is `NaN` → `(int)NaN` is UB; x86-64 `cvttsd2si` yields the integer indefinite value | `-2147483648` | [x] |
| 18 | `convert_double_to_int` | `value` is `+INFINITY` → UB | `-2147483648` | [x] |
| 19 | `convert_double_to_int` | `value` is `-INFINITY` → UB | `-2147483648` | [x] |
| 20 | `convert_double_to_int` | `value > INT_MAX` (one step past range: `2147483648.0`, also `1e300`) → UB | `-2147483648` | [x] |
| 21 | `convert_double_to_int` | `value < INT_MIN` (one step past range: `-2147483649.0`, also `-1e300`, `-2^40`) → UB | `-2147483648` | [x] |
| 22 | `convert_double_to_int` | in-range boundaries `2147483647.0`, `-2147483648.0`, and the fractional `2147483647.9`, `-2147483648.9` (truncate back into range) | `2147483647` / `-2147483648` | [x] |
| 23 | `convert_double_to_int` | negative fraction `-0.9` → C truncates toward zero, not floor | `0` | [x] |
| 24 | `convert_double_to_int` | `-0.0` | `0` | [x] |
| 25 | `process_negation` | any non-zero incl. `INT_MIN`, `INT_MAX`, `-1` → `!!` normalises | `1` | [x] |
| 26 | `process_negation` | `0` | `0` | [x] |
| 27 | `doubleneg` | `param2/3/4 % 256` search value not present in the 256-byte generated buffer → `pos < 0` branch (line 115) prints `"Value %d not found"`, `result` unchanged. **Unreachable by construction**: the buffer is `(param1 + 7i) & 0xFF` for `i ∈ 0..256` and `gcd(7,256)=1`, so it is a permutation of all 256 byte values for every `param1` (truncating `%` and signed overflow both preserve the low 8 bits). Asserted differentially: neither library may ever print `"not found"`. | same accumulated int + same stdout; branch never taken in either | [x] |
| 28 | `doubleneg` | `memchr(buffer, 100, 256) == NULL` (line 121 false) → no print, nothing added. **Unreachable** for the same reason as #27; asserted differentially: both libraries must always print the `"Direct memchr"` line. | same accumulated int + same stdout; branch always taken in both | [x] |
| 27a | `find_value_in_buffer` (the reachable form of #27) | the same "needle absent" rejection reached directly, with buffers where the byte genuinely is missing | `-1` (see row #1) | [x] |
| 29 | `doubleneg` | `param2 == 0` → `calculate_with_doubles` takes the `b == 0` path **and** the combined-feature loop's `search_byte` is constant `param1 % 256` | same accumulated int + same stdout | [x] |
| 30 | `doubleneg` | `param1 == INT_MIN`/`INT_MAX` with `param2` large → `param1 + i*param2` signed overflow (UB, wraps) in the combined-feature loop | same accumulated int + same stdout | [x] |
| 31 | `doubleneg` | all params `0` → every `!!` is `0`, `calculate_with_doubles(0,0,0) == 0.0`, `converted_int == 0` | same accumulated int + same stdout | [x] |
| 32 | out-of-range "enum" values across FFI | the API declares no enums; the widest int-typed inputs (`search_val`, `size`, `seed`, `param1..4`) are swept with `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` — i.e. one step past every documented range | identical results | [x] |
