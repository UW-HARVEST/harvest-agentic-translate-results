# ERRORS.md — error / rejection surface (Phase C)

Mechanically derived from `c_src/src/main.c`.  The grep for rejection
constructs comes back almost empty:

```sh
$ grep -nE 'return|if|assert|NULL|-1|else|switch|while|for|#if|errno' main.c
51:    return 0;          # main's success return, nothing else
```

There is **no** `RETURN_ERROR`, no error enum, no `assert`, no range check, no
null check and no `#define`d min/max in the C source.  Its entire error surface
is therefore *implicit*, and lives in

1. the four `scanf` calls whose return value is **ignored** (`main`), so every
   input failure / matching failure silently leaves the destination variable at
   its initialiser (`x = y = 0`, `b = z = 0`), and
2. the silent truncations performed by the C language itself: `strtol`/`strtoul`
   saturation inside `scanf`, narrowing `long`→`int`/`unsigned int`, and
   storing into the `:2`, `:3`, `:1` bit-fields, and
3. `print_foo`'s unchecked pointer dereference.

Every distinct rejection/degenerate-input path below has a differential test.
`E*` rows are exercised from `main` (stdin) in `tests/phase_c_errors.rs`; `F*`
rows go across the FFI boundary — F1..F4 and F6..F8 in `tests/ffi_inproc.rs`
(in-process `dlsym` calls), F5/F9/F10 in `tests/phase_c_errors.rs` (they need a
fresh process to observe a fatal signal / a pristine `stdin`).

Each `E*` test asserts **both** that the C implementation produces the exact
result documented in the "expected C result" column (so the test provably hits
the intended path) **and** that the Rust implementation produces identical
stdout, exit status and signal.

| #   | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|-----|----------|---------------------------------------------|-------------------|------|-----|
| E1  | `scanf("%u",&x)` #1 | empty stdin (immediate EOF) | input failure, `EOF` returned & ignored; `x` stays `0`; the 3 following `scanf`s also fail ⇒ output `0 0 0 0` | `err_e01_empty_stdin` | [x] |
| E2  | `scanf("%u",&x)` #1 | stdin contains only white space (`" \t\n\v\f\r"`) | white space skipped, then EOF ⇒ input failure, `x` stays `0` ⇒ `0 0 0 0` | `err_e02_whitespace_only` | [x] |
| E3  | `scanf("%u",&x)` #1 | first non-space byte is not `[0-9+-]` (e.g. `"abc"`) | matching failure; offending byte pushed back with `ungetc`; `x` stays `0`; every later `scanf` re-reads the same byte and fails identically ⇒ `0 0 0 0` | `err_e03_matching_failure_cascade` | [x] |
| E4  | `scanf("%u",&x)` | lone `-` / `+` then EOF | sign consumed, no digit ⇒ matching failure, `x` stays `0` | `err_e04_lone_sign_eof` | [x] |
| E5  | `scanf("%u",&x)` | sign followed by a non-digit (`"-a"`, `"+ 1"`) | sign consumed, non-digit pushed back ⇒ matching failure; the *next* conversion starts at the pushed-back byte | `err_e05_sign_then_nondigit` | [x] |
| E6  | `scanf("%u",&x)` | magnitude > `ULONG_MAX` (e.g. 20+ nines) | `strtoul` saturates to `ULONG_MAX`, narrowed to `unsigned int` ⇒ `0xFFFFFFFF`, then `x&3 == 3` | `err_e06_u_overflow_ulong` | [x] |
| E7  | `scanf("%u",&x)` | negative value (`"-1"`, `"-4294967295"`) | `strtoul` negates modulo 2⁶⁴, narrowed ⇒ `(u32)(0-v)` | `err_e07_u_negative` | [x] |
| E8  | `scanf("%u",&x)` | `UINT_MAX < value <= ULONG_MAX` (`"4294967296"`) | silent narrowing mod 2³² ⇒ `0` | `err_e08_u_narrowing` | [x] |
| E9  | `scanf("%u",&x)` | negative *and* overflowing (`"-99999999999999999999999"`) | `strtoul` reports `ERANGE` and returns `ULONG_MAX` **regardless of the sign** ⇒ `0xFFFFFFFF` | `err_e09_u_negative_overflow` | [x] |
| E10 | `scanf("%d",&b)` / `(&z)` | immediate EOF (fewer than 4 tokens on stdin) | input failure, destination keeps `0` | `err_e10_too_few_tokens` | [x] |
| E11 | `scanf("%d",&z)` | matching failure (non-numeric token) | destination keeps `0`, byte pushed back | `err_e11_d_matching_failure` | [x] |
| E12 | `scanf("%d",&z)` | value > `LONG_MAX` | `strtol` saturates to `LONG_MAX` ⇒ `(int)0x7FFFFFFFFFFFFFFF == -1` | `err_e12_d_overflow_long_max` | [x] |
| E13 | `scanf("%d",&z)` | value < `LONG_MIN` | `strtol` saturates to `LONG_MIN` ⇒ `(int)0x8000000000000000 == 0` | `err_e13_d_overflow_long_min` | [x] |
| E14 | `scanf("%d",&z)` | `INT_MAX < value <= LONG_MAX` (`"2147483648"`) | silent narrowing mod 2³² ⇒ `-2147483648` | `err_e14_d_narrowing` | [x] |
| E15 | `scanf("%d",&z)` | exactly `LONG_MIN` / `LONG_MAX` (boundary, not an overflow) | no saturation; narrowed to `0` / `-1` | `err_e15_d_long_boundaries` | [x] |
| E16 | `scanf("%u",&x)` | `"0x1f"` — base is **10**, not 0 | leading `0` consumed as a digit, `x` (…) `= 0`; `x` stays valid, `'x'` pushed back so all later conversions fail ⇒ `0 0 0 0` | `err_e16_hex_prefix_rejected` | [x] |
| E17 | `scanf(...)` | embedded NUL byte (`"1\0002 3 4"`) | NUL is an ordinary non-digit: terminates conversion #1 and makes #2..#4 fail | `err_e17_nul_byte` | [x] |
| E18 | `scanf(...)` | bytes ≥ `0x80` (`"\x80\x81 …"`) | not `isdigit`/`isspace` in the "C" locale ⇒ matching failure | `err_e18_high_bytes` | [x] |
| E19 | `scanf(...)` | float / exponent syntax (`"1.5"`, `".5"`, `"1e3"`) | integer conversion stops at `.`/`e`; remaining bytes make later conversions fail | `err_e19_float_syntax` | [x] |
| E20 | `scanf(...)` | double sign (`"--1"`, `"+-1"`) | first sign consumed, second sign is a non-digit ⇒ matching failure, and it is pushed back so the next conversion consumes it and fails too | `err_e20_double_sign` | [x] |
| E21 | `scanf` on a stream in error/EOF state | stdin is `/dev/null`, a closed fd (`0<&-`, `EBADF`) or a **directory** (`EISDIR` read error) | read error is reported as input failure ⇒ `0 0 0 0`, exit status `0` | `err_e21_unreadable_stdin` (+ shell matrix) | [x] |
| E22 | `scanf(...)` | thousands of digits (no width in the format ⇒ unbounded token) | whole token consumed, `ERANGE` saturation applies | `err_e22_very_long_digit_run` | [x] |
| E23 | `scanf(...)` | token straddling the stdio/reader buffer boundary (offset 4096/8192) | value is unaffected by buffering | `err_e23_buffer_boundary` | [x] |
| F1  | `driver` | `x > 3` (out of range for `unsigned int x : 2`) | silently truncated: `x & 3` | `err_f01_driver_x_out_of_range` | [x] |
| F2  | `driver` | `y > 7` (out of range for `unsigned int y : 3`) | silently truncated: `y & 7` | `err_f02_driver_y_out_of_range` | [x] |
| F3  | `driver` | `b` = a `_Bool` byte outside `{0,1}` (`2..255`) — an out-of-range "enum-like" value that C accepts across the FFI boundary | gcc stores `b & 1` into the 1-bit field ⇒ prints bit 0 of the byte (`2`→`0`, `3`→`1`, `255`→`1`) | `err_f03_driver_bool_out_of_range` | [x] |
| F4  | `driver` | `z = INT_MIN` / `INT_MAX` (extremes of the value range) | printed verbatim with `%d` | `err_f04_driver_z_extremes` | [x] |
| F5  | `print_foo` | `foo == NULL` | unchecked dereference ⇒ `SIGSEGV` (signal 11), no output | `err_f05_print_foo_null` (out-of-process) | [x] |
| F6  | `print_foo` | padding bits 6..7 of byte 0 set (`bits = 0xC0` …) | masked off by `& 3`, `>>2 & 7`, `>>5 & 1` ⇒ ignored | `err_f06_print_foo_padding_bits` | [x] |
| F7  | `print_foo` | padding bytes 1..3 of the bit-field allocation unit set to garbage | never read ⇒ ignored | `err_f07_print_foo_padding_bytes` | [x] |
| F8  | `print_foo` | `z` = every byte pattern incl. `INT_MIN`, `-1`, `INT_MAX` | printed with `%d` (signed) | `err_f08_print_foo_z_patterns` | [x] |
| F9  | `main` (FFI) | called with stdin already at EOF | returns `0` and prints `0 0 0 0` | `err_f09_so_main_eof` | [x] |
| F10 | `main` (FFI/exe) | any input at all (300 random byte strings) | always `return 0`, never a signal — there is no failure exit path | `err_f10_exit_code_always_zero` | [x] |

Additional generic FFI-boundary tests (not tied to a single row above, but
required by the Phase C checklist):

| test | what it covers |
|------|----------------|
| `err_gen_oversized_input` | 1 MiB of digits / white space / newlines on stdin |
| `err_gen_driver_out_of_process` | `driver` through `dlopen` in a fresh process, incl. `b = 254/255`, `x = y = UINT_MAX`, `z = INT_MIN/INT_MAX` |
| `err_gen_print_foo_out_of_process` | `print_foo` through `dlopen`, incl. all-`0xFF` byte image and a `foo_t` whose padding is garbage |

Notes on rows deliberately *not* in the table:

* There is no format-string, allocation, or resource path in the C code, so no
  `NULL`-return / `errno` handling to mirror.
* A *misaligned* `const foo_t *` is undefined behaviour in C too, but gcc
  happens to cope on x86-64, so the Rust `print_foo` reads its two fields with
  `read`/`read_unaligned` on raw pointers rather than through a `&foo_t`; that
  keeps the two implementations identical instead of tripping Rust's debug
  alignment assertion.
* `driver`/`print_foo` have no other parameters, so `x`/`y`/`z` have no invalid
  values at all — every 32-bit pattern is accepted (rows F1/F2/F4 cover the
  ones that are *truncated*, which is the closest thing to a rejection).
