# ERRORS.md — error-surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` by enumerating **every** `return`
that is not the normal success value, **every** guard whose false branch changes
behaviour, and every constant that acts as a threshold. Grep basis:

```sh
grep -n 'return\|assert\|NULL\|if (\|switch\|case\|default' c_src/src/lib.c
```

There are **no** `assert`s, no error enums, and no `RETURN_ERROR`-style macros in
this library. Its entire rejection surface is: one explicit `-1` for a bad
length, one unreachable `-1` for allocation failure, and four *silent* guards
that make a function a no-op or an identity instead of returning an error code.
Silent guards are included because they are exactly the "how does it reject bad
input" behaviour the gate is about — a caller cannot distinguish them from
success by a return code, so a divergence there is invisible unless tested.

`process_string(NULL)`, `shift_array(NULL, ...)` and `arity(len>=2, NULL)`
dereference without a null check in the C. That is undefined behaviour, not a
rejection, so it is **not** a row — a differential test cannot meaningfully
compare two segfaults. Row 12 records this decision explicitly and tests the
one null case that *is* well-defined (`positions <= 0`, where the pointer is
never dereferenced).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `arity` | `len == 0` (`len < 2`, `src/lib.c:170`) | returns `-1`; `params` never dereferenced | [x] |
| 2 | `arity` | `len == 1` (`len < 2`) | returns `-1`; `params` never dereferenced | [x] |
| 3 | `arity` | `len == 256` — truncated to `unsigned char` 0, so `< 2` | returns `-1` (NOT the `arity4` branch a naive `int` reading would take) | [x] |
| 4 | `arity` | `len == 257`/`258`/`259` — truncate to 1/2/3 | `-1` for 257; `arity2` for 258; `arity3` for 259 | [x] |
| 5 | `arity` | `len` negative (`-1`, `-2`, `-256`, `INT_MIN`) — low byte reinterpreted **unsigned** | `-1` iff `(len & 0xFF) < 2`; e.g. `-1`→255→`arity4`, `-256`→0→`-1` | [x] |
| 6 | `arity` | `len` one step past each threshold: 2,3,4,5,255 | 2→`arity2`, 3→`arity3`, **4 and 5 and 255 all → `arity4`** (final `else`, no upper bound; always reads exactly `params[0..4]`) | [x] |
| 7 | `compare_allocations` | `ptr1 == NULL \|\| ptr2 == NULL` (`malloc` failure) | `free(ptr1); free(ptr2); return -1` — unreachable for a 4-byte request; asserted structurally, not by exhausting the heap | [x] |
| 8 | `apply_bitmask` | `operation` outside `{0,1,2,3}` — the `default:` arm. Includes negative values, which is what `arity4` actually feeds it via `param1 % 4` when `param1 < 0` | returns `value` unchanged (identity) | [x] |
| 9 | `apply_bitmask` | out-of-range **enum-style** ints across FFI: `4`, `-1`, `INT_MAX`, `INT_MIN`, `0x100000000 & 0xFFFFFFFF` | all take `default:` → identity | [x] |
| 10 | `shift_array` | `positions <= 0` (guard `positions > 0` false) | **no-op** — array untouched, no zero fill | [x] |
| 11 | `shift_array` | `positions >= size` (guard `positions < size` false) | **no-op** — in particular `positions == size` does NOT zero the array | [x] |
| 12 | `shift_array` | `size <= 0` (incl. `size == 0` with any `positions`, and negative `size`) | no-op: `positions < size` is false for every `positions > 0` | [x] |
| 13 | `shift_array` | `size` negative **and** `positions` negative (both guards false) | no-op; `(size - positions) * sizeof(int)` is never evaluated | [x] |
| 14 | `process_string` | empty string `""` (`*str == 0`, so `if (*str)` false) | returns `0` via the *second* `return`, not via `strlen` | [x] |
| 15 | `process_string` | string whose first byte is `0` but has non-zero bytes after it (`"\0abc"`) | returns `0` — the guard tests only `*str`, `strlen` is never called | [x] |
| 16 | `process_string` | `0x80..0xFF` high-bit bytes (`char` is signed on x86-64, so `*str` is negative but non-zero) | truthy → returns `strlen` | [x] |
| 17 | `arity4` | `param3 == 0` — guard `if (param3 != 0)` false | the `* param3 / 100` rescale is **skipped entirely** (not multiplied by 0) | [x] |
| 18 | `arity4` | `param4 == 0` — guard `if (param4 != 0)` false | the final `+= param4` is skipped (indistinguishable from adding 0, but the branch is real) | [x] |
| 19 | `arity4` | signed overflow in `result * param3` (e.g. `param1=param2=INT_MAX`, `param3=INT_MAX`) | UB in C; GCC emits wrapping `imul`. Rust must wrap, not panic | [x] |
| 20 | `arity4` | signed overflow in `result += param4` (`param4 = INT_MAX`/`INT_MIN`) | wrapping add | [x] |
| 21 | `arity4` | negative `result` in the `/ 100` truncating division | C truncates toward zero (`idiv`); Rust `/` must too | [x] |
| 22 | `arity4` | `param1 = INT_MIN` in `param1 % 4` | `INT_MIN % 4 == 0` → `apply_bitmask` case 0. No `INT_MIN / -1` trap (divisor is the literal 4) | [x] |
| 23 | `init_matrix` | no error path at all — unconditionally writes all 12 cells; caller-supplied buffer is never validated | recorded for completeness; writes `1..12` row-major | [x] |
| 24 | *generic* | zero / oversized lengths: `arity` with `len = 255` reads only `params[0..4]`; `shift_array` with `size = INT_MAX` and `positions = 1` would `memmove` `(INT_MAX-1)*4` bytes | not exercised destructively; `size` is bounded in tests to the real buffer | [x] |

## Deliberate non-rows (undefined behaviour, not rejection)

| condition | why not a row |
|-----------|---------------|
| `process_string(NULL)` | unguarded `*str` → segfault in both C and Rust; comparing two crashes proves nothing |
| `shift_array(NULL, size>1, 0<positions<size)` | unguarded `memmove` → segfault. The *guarded* null case (`positions <= 0`) IS tested (row 10/12) and must return without touching the pointer |
| `arity(len>=2, NULL)` | unguarded `params[0]` → segfault. The guarded case (`len < 2`) IS tested with a null `params` (rows 1–3) |
| reading `*uninit_ptr` in `compare_allocations` | `uninit_ptr = ptr1` is assigned before use, so despite the name it is fully defined: it aliases `ptr1`, hence `*uninit_ptr == val1`. Covered by the `val1 > 0` / `val1 <= 0` split in `CONFIGS.md` |
