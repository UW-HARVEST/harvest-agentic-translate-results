# ERRORS.md — error-surface table (Phase A / Phase C)

## 1. Mechanical derivation

Every rejection mechanism a C library can have was grepped for over the whole
C source (`c_src/src/long.c`, `c_src/include/long.h`):

```
$ grep -n "return" -r src include
src/long.c:66:    return;                 # bare `return;` from a void function — not an error path

$ grep -n -E "assert|NULL|errno|exit|abort|perror|fprintf|stderr|-1|if *\(|switch|#if" -r src include
include/long.h:24:#ifndef ECHO_H_        # include guard only

$ grep -n "#define" -r src include
src/long.c:29:#define ARRAY_SIZE (256 * 1024)
src/long.c:30:#define ITERATIONS 2000
include/long.h:25:#define ECHO_H_
```

Result: the library has **no error-return macros, no error enums, no
`assert`s, no range checks, no null checks, no `errno` use, no min/max
validation constants, and no failure return values at all**. Both public
functions return `void`, take no pointer arguments, and contain no `if`,
`switch`, or `#ifdef` branch. `ARRAY_SIZE` and `ITERATIONS` are compile-time
loop bounds, not validated limits.

Consequently the error-surface table below has no rows of the
"function rejects input X" kind. To make the table useful instead of vacuous,
it enumerates every *degenerate / boundary / hostile* input that can be
presented to this ABI across the FFI boundary, together with the behaviour the
C actually exhibits (which is what the Rust must reproduce). Every row has a
differential test in `tests/phase_c_errors.rs`.

## 2. Error / boundary surface

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|----|----------|---------------------------------------------|-------------------|------|----|
| E1 | *(whole library)* | any explicit error return / error code / sentinel | **does not exist** — no `return <error>`, no `NULL` return, no `assert`, no `errno`; both entry points are `void` and unconditionally succeed | `no_error_paths_exist_in_c_source` | [x] |
| E2 | `long_exec` | `seed = 0` (glibc `srand(0)` is documented to behave like `srand(1)`) | no rejection; PRNG seeded, array filled, single `"%d\n"` line printed | `boundary_seed_values_produce_identical_fill` | [x] |
| E3 | `long_exec` | `seed = 1` (glibc identity seed) | no rejection; identical fill to `seed = 0` | `boundary_seed_values_produce_identical_fill` | [x] |
| E4 | `long_exec` | `seed = 0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF` (`UINT_MAX`), i.e. one step past the signed range and the top of the `unsigned int` range | no rejection; `srand` takes the value modulo its own state arithmetic | `boundary_seed_values_produce_identical_fill` | [x] |
| E5 | `long_exec` | seed passed as a *negative* `int` from the caller (e.g. `-1`, `INT_MIN`) — legal across the FFI boundary because the ABI passes a 32-bit register | no rejection; reinterpreted as `unsigned` (`-1` ⇒ `0xFFFFFFFF`) | `negative_seed_is_reinterpreted_as_unsigned` | [x] |
| E6 | `perform_expensive_operations` | `array` element = `INT_MIN` (`-2147483648`): `x * 3`, `x << 1`, `x - (x << 1)` all overflow (signed overflow is UB in C; gcc emits two's-complement wrapping) and `x / 2`, `x % 7` must truncate toward zero | no trap, no abort: wrapping result; **must not panic in Rust** (`wrapping_*` used) | `extreme_element_values_do_not_trap` | [x] |
| E7 | `perform_expensive_operations` | `array` element = `INT_MAX`, `-1`, `0`, `1`, `-7`, `7`, `INT_MIN + 1` (division/modulo sign boundaries, `x % 7 == 0` boundary) | wrapping/truncating result, no trap | `extreme_element_values_do_not_trap` | [x] |
| E8 | `perform_expensive_operations` | called **before** anything writes `array` (i.e. on the zero-initialised `.bss` state) — the "zero-length/uninitialised input" analogue for a library whose only input is a global | no rejection; `0` is transformed like any other value; result deterministic | `zero_initialised_bss_state` | [x] |
| E9 | `perform_expensive_operations` | called repeatedly (0, 1, 2, … times) — state is carried in the global; there is no "reset" or "already initialised" error | no rejection; each call is the previous state transformed again | `repeated_calls_never_reject` | [x] |
| E10 | `perform_expensive_operations` | called with **extra arguments** across the FFI boundary. The C definition is `void perform_expensive_operations()` — an old-style declarator with an *unspecified* parameter list, so any argument list is accepted by the C ABI | arguments ignored; identical effect to the no-argument call | `extra_ffi_arguments_are_ignored` | [x] |
| E11 | `long_exec` / `perform_expensive_operations` | null-pointer arguments | **not applicable**: neither function takes a pointer. Row kept to record that the standard null-pointer boundary was examined and does not exist in this ABI | `no_pointer_parameters_exist` | [x] |
| E12 | *(whole library)* | out-of-range `enum` value across the FFI boundary | **not applicable**: the library declares no `enum` and no mode/flag parameter. Recorded here so the absence is explicit; the closest analogue (an arbitrary 32-bit value in the only scalar parameter) is covered by E2–E5 | `no_enum_parameters_exist` | [x] |
| E13 | `array` | writing the very first and very last element of the exported `array` object (`array[0]`, `array[262143]`) and checking the transformed value comes back — one step past that (`array[262144]`) is out of bounds of a 0x100000-byte object in *both* libraries, so it is not a valid input | first/last element handled; symbol size identical (`0x100000`) in both `.so`s so no oversized/undersized buffer divergence | `array_bounds_are_identical` | [x] |
| E14 | `long_exec` | stdout not writable / output redirected | `printf` return value ignored by the C, so no failure is reported | `printf_failure_is_ignored` | [x] |

## 3. Results

All 14 rows are covered by passing differential tests in
`tests/phase_c_errors.rs`, run against both Rust build profiles:

```
tests/phase_c_errors.rs   debug   11 passed   (37 s)
tests/phase_c_errors.rs   release 11 passed   (20 s)
```

(11 test functions cover the 14 rows; rows E2–E4 share
`boundary_seed_values_produce_identical_fill` and rows E6–E7 share
`extreme_element_values_do_not_trap`.)

Rows E2–E4 are additionally covered by the *real* `long_exec` entry point in
`tests/phase_e2e.rs`, which was run end to end (2000 iterations, ~500 s per
library per seed) for seeds `0`, `7`, `42` and `0xFFFFFFFF`: the captured
`printf` bytes, the XOR reduction and (seed 7) the full 1 MiB final `array` are
byte-identical between the C and the Rust library.
