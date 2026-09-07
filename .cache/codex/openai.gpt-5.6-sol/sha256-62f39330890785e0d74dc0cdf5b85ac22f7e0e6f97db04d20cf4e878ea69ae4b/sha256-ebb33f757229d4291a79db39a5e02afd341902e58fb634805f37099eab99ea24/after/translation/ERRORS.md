# Error surface

The complete C source was mechanically searched for error-return statements,
error macros/enums, assertions, null checks, range checks, and min/max
constants. It contains none. `driver` returns `void` and performs no input
validation before passing both pointers to libc `strcspn`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

There are therefore no C-defined rejection rows to check off. The generic FFI
boundary audit still covers null pointers in isolated subprocesses because C
does not reject them and dereferencing them has undefined behavior. Zero and
oversized explicit lengths, enums, and documented numeric ranges are not
applicable: this API has only two NUL-terminated string pointers.

Generic FFI boundary audit:

- [x] null `s1`: C and Rust terminate with the same process signal
- [x] null `s2`: C and Rust terminate with the same process signal
- [x] zero/oversized explicit lengths: not applicable (no length parameters)
- [x] out-of-range enums/numeric values: not applicable (no enum or numeric parameters)
