# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / error-return / sentinel / implicit
guard in `c_src/src/lib.c`. Line numbers refer to that file.

There are no `assert`s, no error enums, no `RETURN_ERROR`-style macros and no
`errno` usage in the C source. The complete set of rejection sites is:

- `lib.c:37` `return NULL;` (struct `malloc` failed)
- `lib.c:43` `return NULL;` (data `malloc` failed, after `free(buffer)`)
- `lib.c:62` `return -1;` (`realloc` failed)
- `lib.c:57` range check `required_capacity > buffer->capacity`
- `lib.c:76` null check `if (buffer)`
- `lib.c:77` null check `if (buffer->data)`
- `lib.c:90` `default: return "unknown";` (out-of-range op code)
- `lib.c:102` `if (b != 0)` guard, `lib.c:105` `return 0`
- `lib.c:107` `return 0;` (unrecognised operation string)
- `lib.c:141` `if (intermediate3 != 0)` / `lib.c:144` fallback path

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| E1 | `create_buffer` | `initial_capacity < 0` (e.g. `-1`): sign-extended to a huge `size_t`, `malloc` fails (`lib.c:41`) | returns `NULL`; struct freed, no leak |
| E2 | `create_buffer` | `initial_capacity = INT_MIN` (extreme negative, sign-extends to `0xFFFF_FFFF_8000_0000`) | returns `NULL` |
| E3 | `create_buffer` | `initial_capacity = INT_MAX` (2 GiB request; may or may not succeed, but C and Rust must agree on NULL-ness) | same NULL-ness in both |
| E4 | `create_buffer` | `initial_capacity = 0`: `malloc(0)` returns a non-NULL minimal chunk, then `buffer->data[0] = '\0'` is still written (`lib.c:48`) | returns non-NULL, `capacity == 0`, `length == 0` |
| E5 | `create_buffer` | struct `malloc` fails (`lib.c:36`) | returns `NULL` — not reachable from the public API on this platform; documented, not tested |
| E6 | `append_to_buffer` | `realloc` fails: `buffer->length` large enough that `new_capacity = required*2` overflows `int` to a negative value which sign-extends to a huge `size_t` (`length = 2_000_000_000`, short `str`) | returns `-1`; `buffer->data`/`capacity`/`length` left unmodified |
| E7 | `append_to_buffer` | `realloc` fails via negative `capacity` path with `length = INT_MAX/2 + k` variants (several randomized large lengths) | returns `-1` |
| E8 | `append_to_buffer` | empty string `""`: `str_len = 0`, `required = length + 1`; when `length + 1 <= capacity` the grow branch is **not** taken | returns `0`, `capacity` unchanged, `length` unchanged |
| E9 | `append_to_buffer` | boundary `required_capacity == buffer->capacity` (one byte short of triggering growth) | returns `0`, no realloc, `capacity` unchanged |
| E10| `append_to_buffer` | boundary `required_capacity == buffer->capacity + 1` (first value that triggers growth) | returns `0`, `capacity` becomes `required*2` |
| E11| `append_to_buffer` | `buffer == NULL` | NULL dereference → `SIGSEGV` (C UB). Documented; asserted identical only as "both crash" via subprocess, not as a return value |
| E12| `append_to_buffer` | `str == NULL` | `strlen(NULL)` → `SIGSEGV` (C UB). Documented; not asserted as a return value |
| E13| `destroy_buffer` | `buffer == NULL` | no-op, returns cleanly (`lib.c:76`) |
| E14| `destroy_buffer` | `buffer != NULL` but `buffer->data == NULL` | frees only the struct, does not call `free(NULL)` path on data (`lib.c:77`); returns cleanly |
| E15| `get_operation_name` | `op_code = 4` (one past the documented valid range) | `"unknown"` |
| E16| `get_operation_name` | `op_code = -1` (one before the range; also what `x % 4` yields for negative `x`) | `"unknown"` |
| E17| `get_operation_name` | `op_code = -2, -3, 5, 42, INT_MIN, INT_MAX` — arbitrary out-of-range "enum" ints crossing FFI | `"unknown"` for every one |
| E18| `perform_operation` | `operation` is an unrecognised string (`""`, `"ADD"`, `"add "`, `" add"`, `"divide\0x"`, random bytes) | returns `0` (`lib.c:107`) |
| E19| `perform_operation` | `operation = "divide"`, `b == 0` | returns `0` (`lib.c:105`), no `SIGFPE` |
| E20| `perform_operation` | `operation = "divide"`, `a = INT_MIN`, `b = -1` | signed-overflow division: C emits a bare `idiv` → `SIGFPE`. Must behave identically in Rust (verified in a subprocess) |
| E21| `perform_operation` | `operation = NULL` | `strcmp(NULL, ...)` → `SIGSEGV` (C UB). Documented; not asserted as a return value |
| E22| `perform_operation` | signed overflow on `add`/`subtract`/`multiply` (`INT_MAX + 1`, `INT_MIN - 1`, `INT_MIN * -1`, …) — C UB, gcc wraps | wrapping two's-complement result, identical in both |
| E23| `buffapp` | `intermediate3 == 0` (e.g. either operand of the final multiply is 0, or one of `op1`/`op2` resolved to `"unknown"`/`"divide by 0"` giving 0) | takes the `lib.c:144` fallback: `result = p1+p2+p3+p4` |
| E24| `buffapp` | `intermediate3 != 0` and `result / intermediate3` overflows (`result = INT_MIN`, `intermediate3 = -1`) | `idiv` → `SIGFPE`, identical in both (subprocess) |
| E25| `buffapp` | `param1 % 4 < 0` and/or `param3 % 4 < 0` (negative params → `get_operation_name` default branch → `perform_operation` returns 0) | `intermediate` is 0, log line contains `unknown` |
| E26| `buffapp` | `log_buffer == NULL` because `create_buffer(32)` failed (`lib.c:112` result is dereferenced at `lib.c:116` with no check) | NULL dereference → `SIGSEGV`; unreachable with a fixed capacity of 32. Documented, not tested |

## Checklist

- [x] E1  `create_buffer(-1)` → NULL
- [x] E2  `create_buffer(INT_MIN)` → NULL
- [x] E3  `create_buffer(INT_MAX)` → same NULL-ness
- [x] E4  `create_buffer(0)` → non-NULL, capacity 0
- [x] E5  struct `malloc` failure — unreachable, documented only
- [x] E6  `append_to_buffer` realloc failure → `-1`
- [x] E7  `append_to_buffer` randomized large-length realloc failures → `-1`
- [x] E8  empty string append → `0`, no growth
- [x] E9  `required == capacity` boundary → no growth
- [x] E10 `required == capacity + 1` boundary → growth to `required * 2`
- [x] E11 `append_to_buffer(NULL, s)` → both crash (subprocess)
- [x] E12 `append_to_buffer(buf, NULL)` → both crash (subprocess)
- [x] E13 `destroy_buffer(NULL)` → no-op
- [x] E14 `destroy_buffer` with `data == NULL` → clean
- [x] E15 `get_operation_name(4)` → `"unknown"`
- [x] E16 `get_operation_name(-1)` → `"unknown"`
- [x] E17 out-of-range enum ints incl. `INT_MIN`/`INT_MAX` → `"unknown"`
- [x] E18 unrecognised operation strings → `0`
- [x] E19 `"divide"` by `0` → `0`
- [x] E20 `INT_MIN / -1` → identical fatal signal (subprocess)
- [x] E21 `perform_operation(a, b, NULL)` → both crash (subprocess)
- [x] E22 signed-overflow arithmetic wraps identically
- [x] E23 `buffapp` `intermediate3 == 0` fallback
- [x] E24 `buffapp` division overflow → identical fatal signal (subprocess)
- [x] E25 `buffapp` negative `% 4` → `unknown` operations
- [x] E26 `buffapp` NULL `log_buffer` — unreachable, documented only
