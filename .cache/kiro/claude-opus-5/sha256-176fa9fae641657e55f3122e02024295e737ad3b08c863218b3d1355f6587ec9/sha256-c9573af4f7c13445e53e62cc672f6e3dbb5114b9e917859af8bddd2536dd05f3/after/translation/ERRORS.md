# ERRORS.md — error-surface table (Phase A / Phase C)

## How this table was derived

Mechanically, from the complete C source (`c_src/src/driver.c`, 50 lines, and
`c_src/include/driver.h`), by grepping for every rejection mechanism a C library
can use:

```
grep -nE 'return|assert|NULL|errno|ERROR|_MAX|_MIN|if *\(|switch|\bexit\b|abort' \
     src/driver.c include/driver.h
```

→ **no matches at all.**

Reading the source confirms it:

* `void driver(int floors)` returns `void` — there is no return value, no error
  code, no sentinel, and no `errno` write. It cannot signal failure.
* There is no `assert`, no `NULL` check, no explicit range check, no min/max
  constant, no `#ifdef`, no `if`, and no `switch` anywhere in the library.
* No pointer, array, length, enum, or struct is accepted from the caller — the
  single parameter is a by-value `int`, and **every** `int` bit pattern is a
  valid input that the C code accepts and processes identically (it is copied
  verbatim into `house.floors` and dumped).
* `print_hex` is `static`, never receives a caller-supplied pointer or length
  (it is always called with `&raw` / `sizeof(raw)`), so its loop bound cannot be
  driven out of range from outside.

So the library's error surface is genuinely **empty**. To keep the phase
meaningful rather than vacuous, the rows below record the generic C-API
boundaries the task calls out, expressed for the one parameter that exists, and
each row is still verified as a real differential test (same observable result
from both `.so`s, byte for byte, including the exit status / absence of a trap).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `driver` | `floors = 0` (zero "length"-analogue / all-zero bit pattern) | no error; prints `00000000` + `03000000` + `0000000000000040` + `\n`. `void`, nothing to reject. | [x] |
| 2 | `driver` | `floors = INT_MAX` (`2147483647`) — one step past it is not representable, so this is the top of the documented valid range | no error; prints the little-endian bytes `ffffff7f…`. Accepted, not rejected. | [x] |
| 3 | `driver` | `floors = INT_MIN` (`-2147483648`) — bottom of range | no error; prints `00000080…`. Accepted, not rejected. | [x] |
| 4 | `driver` | `floors = -1` (all-ones bit pattern / the classic error sentinel value passed *in*) | no error; prints `ffffffff…`. Not treated specially. | [x] |
| 5 | `driver` | `floors = INT_MAX + 1` computed as a *64-bit* value `0x80000000` and passed across the FFI boundary in a 64-bit register (i.e. one step past the valid `int` range, as an out-of-range "enum-like" integer would arrive) | no error; the callee only observes the low 32 bits per the SysV AMD64 ABI, so the result is identical to `floors = INT_MIN`. The upper garbage bits are ignored, not rejected. | [x] |
| 6 | `driver` | out-of-range *enum* value across FFI: the API declares no enum, so the nearest real case is an arbitrary integer with no "valid variant" (e.g. `0x7fffffff`, `0xdeadbeef` as `int`) | no error; every bit pattern is a valid variant. Processed, not rejected. | [x] |
| 7 | `driver` | null-pointer boundary | **not reachable**: `driver` takes no pointer parameter, and the library exports no other symbol. There is no null-pointer path to test. Recorded so the omission is explicit rather than an oversight. | [x] n/a |
| 8 | `driver` | oversized-length boundary | **not reachable**: no caller-supplied length exists. `print_hex`'s length is always `sizeof(house_t)` (16), fixed at compile time and unreachable from outside (`static`). | [x] n/a |

Rows 1–6 are exercised by `tests/differential.rs::error_surface_*`; rows 7–8
are structurally unreachable and are documented as such.
