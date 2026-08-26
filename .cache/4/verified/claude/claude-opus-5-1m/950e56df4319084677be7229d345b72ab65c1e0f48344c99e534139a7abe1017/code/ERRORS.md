# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical grep for every rejection construct

```
$ grep -nE "return|assert|NULL|errno|exit|abort|goto|RETURN_ERROR|if|switch|case|#if" \
      c_src/src/driver.c c_src/include/driver.h    # (comment lines excluded)
c_src/include/driver.h:24:#ifndef DRIVER_H_        <- include guard only
c_src/include/driver.h:29:#endif //DRIVER_H_       <- include guard only
c_src/src/driver.c:30:    for (int i = 0; i < len; i++) {   <- loop bound
```

Findings — the complete rejection inventory:

* `return` statements with a value: **0** (both functions return `void`).
* `assert` / `abort` / `exit` / `goto` / `errno` / error enums / `RETURN_ERROR`
  macros / sentinel returns (`-1`, `NULL`): **0 occurrences**.
* Null-pointer checks: **0**. `print_hex` dereferences `p` unconditionally.
* Explicit range / min / max checks or named limit constants: **0**.
* Conditional branches of any kind: **1** — the `i < len` loop-continuation test
  in `print_hex` (line 30). This is the only comparison in the library.
* `#ifdef` / configuration branches: **0** (only the header's include guard).

Consequently the public API **has no error surface**: `void driver(int x)`
accepts every one of the 2^32 `int` values, rejects none, returns nothing, and
sets no status anywhere. The rows below are therefore the *complete* set of
"rejection-like" conditions the C actually contains, plus the generic FFI
boundaries mandated for every C API. "Expected C result" is the ground truth the
Rust must reproduce byte-for-byte.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `print_hex` (via `driver`) | loop bound `i < len` with `len == 0` — the only guard in the library. Not reachable from the public API because `driver` always passes `sizeof(raw) == 4`; verified indirectly: no call ever emits fewer than 4 hex bytes. | no `%02x` iterations, still emits the trailing `"\n"` → output `"\n"` | `e1_loop_bound_never_degenerates` |
| E2 | `print_hex` (via `driver`) | negative `len` (`i < len` false immediately). Also unreachable from the public API (`sizeof` is unsigned, always 4). Same expected behaviour as E1. | no iterations, output `"\n"` | `e1_loop_bound_never_degenerates` |
| E3 | `driver` | `x` = most negative value `INT_MIN` (`0x80000000`) — most-significant byte `0x80` has the sign bit set. No check rejects it. | prints `"00000080\n"` (little-endian: `00 00 00 80`). Must NOT sign-extend to `ffffff80`. | `e3_int_min_no_sign_extension` |
| E4 | `driver` | `x == -1` (`0xffffffff`) — every byte is `0xff`, maximal per-byte value. No check rejects it. | prints `"ffffffff\n"` | `e4_minus_one_all_ff` |
| E5 | `driver` | `x == INT_MAX` (`0x7fffffff`), one step past which `int` overflows. No check rejects it. | prints `"ffffff7f\n"` | `e5_int_max` |
| E6 | `driver` | any individual object-representation byte in `0x80..=0xff` (high bit set) — the `unsigned char` → `int` promotion at line 31. | each such byte prints as two lowercase hex digits `80..ff`, never `ffffff80` | `e6_every_high_bit_byte_in_every_position` |
| E7 | `driver` | value whose bytes are all `< 0x10`, i.e. the `%02x` zero-padding path (`x == 0` gives four `0x00` bytes) | `"00000000\n"` — padded to 2 digits each, never `"0000\n"` | `e7_zero_padding` |
| E8 | `driver` (generic FFI boundary) | caller passes a value with garbage in the **upper 32 bits** of the argument register, i.e. an out-of-range value for the declared `int` parameter (the analogue of an out-of-range enum: the C ABI lets any 64-bit pattern arrive, and `int` reads only the low half) | both must ignore the upper 32 bits and print only the low 32 bits' object representation, identically | `e8_upper_argument_bits_ignored` |
| E9 | `driver` (generic FFI boundary) | 0-length / null-pointer inputs: **not applicable** — the public API takes no pointer and no length, so there is no null or length argument to abuse. `print_hex`'s pointer is always the address of a live 4-byte local and is `static`, hence unreachable across the FFI boundary. | n/a (documented, nothing to test) | — (n/a, justified) |
| E10 | `driver` (generic FFI boundary) | out-of-range **enum** value across FFI: **not applicable** — the library declares no enum type. Closest analogue is E8. | n/a (documented, nothing to test) | — (n/a, justified) |

## Checklist

- [x] E1 — loop bound, `len == 0`
- [x] E2 — loop bound, negative `len`
- [x] E3 — `INT_MIN`, no sign extension
- [x] E4 — `-1`, all `0xff`
- [x] E5 — `INT_MAX`
- [x] E6 — high-bit byte in every position
- [x] E7 — `%02x` zero padding
- [x] E8 — upper argument bits ignored
- [x] E9 — n/a (no pointer/length in the public API), justified above
- [x] E10 — n/a (no enum in the library), justified above
