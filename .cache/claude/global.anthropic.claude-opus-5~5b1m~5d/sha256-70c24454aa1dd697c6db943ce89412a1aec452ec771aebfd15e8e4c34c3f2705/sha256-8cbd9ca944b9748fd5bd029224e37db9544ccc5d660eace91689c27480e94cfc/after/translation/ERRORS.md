# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/staticalias.c` + `c_src/include/staticalias.h`
by grepping for every `return`, `assert`, `if`, null check, range check, error
macro/enum and min/max constant:

```
$ grep -nE 'return|assert|NULL|-1|if|error|ERROR|exit|INT_MAX|INT_MIN|enum|switch|#if' \
      src/staticalias.c include/staticalias.h
src/staticalias.c:30:  if(*outer >= inner) {
src/staticalias.c:32:    return &inner;
src/staticalias.c:35:    return outer;
src/staticalias.c:49:  return;
include/staticalias.h:24:#ifndef STATICALIAS_H_   (header guard)
```

**Mechanical finding:** the C library contains **no** `RETURN_ERROR` macro, **no**
`assert`, **no** error enum, **no** `return -1` / `return NULL` sentinel, **no**
null check, **no** explicit range check, and **no** `INT_MIN`/`INT_MAX` clamp.
Both functions are total on their declared parameter types: `static_alias`
*always* returns a valid non-null `int*` (either `&inner` or its own `outer`
argument) and `driver` returns `void`. There is no enum in the public API, so the
"out-of-range enum variant" class degenerates to "arbitrary 32-bit `int`", which
is covered below (rows 5–9).

Consequently the rejection surface is entirely **implicit**: it consists of the
inputs that make the C undefined/trap, plus the degenerate-count and
extreme-value boundaries. Every row below is a real input an external caller can
pass across the FFI boundary, and each is asserted to produce the **same**
observable outcome (same fatal signal, or same returned sentinel/pointer
identity + same bytes) from C and Rust.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| 1 | `static_alias` | `outer == NULL` — the unguarded `*outer` read at line 30 dereferences null (no null check exists) | fatal `SIGSEGV` (signal 11); no return, no state change | `err_01_static_alias_null_ptr_same_signal` | [x] |
| 2 | `driver` | `iterations == 0` — loop guard `i < iterations` false on entry | returns normally, prints **nothing** (0 bytes), leaves `inner` untouched | `err_02_driver_zero_iterations` | [x] |
| 3 | `driver` | `iterations < 0` (incl. `INT_MIN`) — one step past the valid `>= 0` count range | returns normally, prints nothing, leaves `inner` untouched (no clamp, no error) | `err_03_driver_negative_iterations` | [x] |
| 4 | `static_alias` | `outer` aliases the returned `&inner` (caller feeds the static's own address back in) — `*outer >= inner` is then trivially true, so `inner += inner` | returns `&inner`, `inner` doubled; pointer identity == previous return | `err_04_static_alias_self_alias` | [x] |
| 5 | `static_alias` | signed overflow of `inner += *outer` (line 31): `inner = INT_MAX`, `*outer >= inner` | C signed-overflow UB; as compiled here it wraps two's-complement | `err_05_static_alias_overflow_inner` | [x] |
| 6 | `static_alias` | signed overflow of `*outer += inner` (line 34): `*outer = INT_MIN`, `inner > *outer` | C signed-overflow UB; as compiled here it wraps two's-complement | `err_06_static_alias_overflow_outer` | [x] |
| 7 | `static_alias` | `*outer == INT_MIN` (extreme low end of the `int` domain) — takes the `else` branch | returns `outer` (input pointer), `*outer = INT_MIN + inner` (wrapping) | `err_07_static_alias_int_min` | [x] |
| 8 | `static_alias` | `*outer == INT_MAX` (extreme high end) — takes the `then` branch | returns `&inner`, `inner = inner + INT_MAX` (wrapping) | `err_08_static_alias_int_max` | [x] |
| 9 | `driver` | `initial_value` = `INT_MIN` / `INT_MAX` combined with a large `iterations`, driving repeated overflow through the alias chain | full stdout byte stream, whatever the wrapped arithmetic yields | `err_09_driver_extreme_values` | [x] |
| 10 | `static_alias` | boundary `*outer == inner` exactly — the `>=` (not `>`) picks the *then* branch; an off-by-one to `>` would flip it | returns `&inner`, `inner` doubled | `err_10_static_alias_equal_boundary` | [x] |
| 11 | `driver` | `iterations == 1` — minimum non-degenerate count (one step past the empty case of row 2) | exactly one line of output | `err_12_driver_one_iteration` | [x] |
| 12 | `static_alias` | non-null but **invalid/unmapped** `outer` pointer | UB in C; traps identically in Rust — *documented, not asserted* (see note) | n/a | [x] |

**Note on row 12:** an arbitrary wild pointer is undefined in the C and in the
Rust `unsafe` block alike; the observable outcome depends on the process address
space rather than on the translation, so asserting equality would test the OS,
not the port. Row 1 (`NULL`, the one wild-pointer value that is deterministic
and portable) *is* asserted, in a forked child, via its exit signal.

Rows 5, 6, 9 concern C signed-overflow UB. The C is the ground truth: whatever
`libStaticAlias.so` as built by `c_src/CMakeLists.txt` actually does is what the
Rust must match, and the tests compare the two shared objects rather than
appealing to the standard. The Rust uses `wrapping_add` so it cannot panic in
release *or* debug and matches the compiled C's two's-complement wrap.
