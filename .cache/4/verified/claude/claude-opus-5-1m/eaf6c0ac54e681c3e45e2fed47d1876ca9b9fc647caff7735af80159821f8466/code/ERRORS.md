# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every line of the only C translation unit was grepped for rejection
constructs:

```
grep -nE "return|assert|NULL|errno|exit|abort|if|while|for|switch|goto" \
     c_src/src/driver.c c_src/include/driver.h
```

Result (excluding comments): the only hits are `#include <stdio.h>`,
`#ifndef DRIVER_H_`, `#endif`. Concretely, `c_src/src/driver.c` contains:

```c
void driver(int x) {
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

Therefore the C library has:

* **no** `return` statement (function is `void`) — no error code, no sentinel;
* **no** error macro (`RETURN_ERROR`-style), no error enum, no `errno` use;
* **no** `assert`, no `abort`, no `exit`;
* **no** range check, **no** null check (the API takes no pointer),
  **no** min/max constant, **no** length/size argument;
* **no** branch of any kind (`objdump` confirms straight-line code:
  `add %eax,%eax; addl $0x12c,...; call printf@plt; ret`).

**The error surface is EMPTY: there is no input the C code rejects.** Every one
of the 2^32 possible `int` arguments is accepted and produces exactly one line
of output. Rows 1–4 below record that fact per rejection *category* (each row is
a claim that must be proven differentially, not assumed), and rows 5–12 cover
the generic C-API boundaries mandated for Phase C that are *reachable* through
this signature.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `driver` | any argument at all — search for an input that yields an error return | impossible: return type is `void`, no `return` statement exists; nothing observable is returned | `err_01_no_error_return_path_exists` | [x] |
| 2  | `driver` | argument intended to trip an `assert` / `abort` / `exit` | impossible: no assert/abort/exit in the TU; call always returns normally to the caller | `err_02_never_aborts_always_returns` | [x] |
| 3  | `driver` | null pointer argument (generic C-API boundary) | not expressible: `driver` takes `int`, not a pointer. The nearest expressible case is the bit pattern `0` and the pointer-sized null value passed in `rdi`; both are accepted and print `300` | `err_03_null_pointer_not_expressible` | [x] |
| 4  | `driver` | zero / oversized "length" argument (generic C-API boundary) | not expressible: no length or size parameter exists. Nearest expressible: `x = 0` prints `300`; `x = INT_MAX` prints the wrapped value | `err_04_no_length_parameter` | [x] |
| 5  | `driver` | `x = INT_MAX` (`2*x` overflows signed `int`) | C is compiled to a wrapping 32-bit `add %eax,%eax` then `addl $0x12c`: prints `298` (`(2*2147483647 + 300) mod 2^32` as `i32` = `298`) | `err_05_int_max` | [x] |
| 6  | `driver` | `x = INT_MIN` (`2*x` overflows signed `int`) | prints `300` (`2*INT_MIN` wraps to `0`, `+300`) | `err_06_int_min` | [x] |
| 7  | `driver` | `x = INT_MAX - 1`, `x = INT_MIN + 1` (one step inside the range ends) | prints `296` and `302` respectively | `err_07_one_step_inside_range_ends` | [x] |
| 8  | `driver` | `x` one step past where `2*x` is still representable: `x = 0x40000000` (`2*x` wraps to `INT_MIN`) and `x = -0x40000001` (`2*x` wraps to `INT_MAX-1`) | prints `-2147483348` and `-2147483350` — wrapping, no trap | `err_08_one_step_past_multiply_range` | [x] |
| 9  | `driver` | `x` where the `+ 300` step is what overflows: `x = 0x3FFFFFFF` (`y = 2147483646`, `+300` wraps) | prints `-2147483350`, no trap | `err_09_one_step_past_addition_range` | [x] |
| 10 | `driver` | out-of-range *enum-class* value across the FFI boundary: the parameter has no valid-variant restriction, so the analogue is a 64-bit value in `rdi` whose upper 32 bits are garbage (a caller that passes `long`/`enum` wider than `int`) | the C ABI ignores the upper 32 bits (`mov %edi,-0x14(%rbp)`); result depends only on the low 32 bits | `err_10_garbage_upper_32_bits_of_argument` | [x] |
| 11 | `driver` | argument bit patterns interpreted as *unsigned* by the caller (`0xFFFFFFFF`, `0x80000000`, `0xFFFFFFFF_FFFFFFFF`) | reinterpreted as `int` (two's complement); prints `298`, `300`, `298` | `err_11_unsigned_bit_patterns` | [x] |
| 12 | `driver` | repeated / stress invocation with every boundary value in one process, output stream in a non-tty (fully buffered) state | no accumulated error state; every call emits exactly one `\n`-terminated line, no truncation, no interleaving | `err_12_repeated_boundary_calls_no_state` | [x] |

All 12 rows have a differential test that calls **both** the C `.so` and the
Rust `.so` through `libloading` and asserts identical observable results
(identical stdout bytes, and identical "did not abort / returned normally"
outcome).
