# Configuration Surface

The public header exposes `long_exec(unsigned int seed)`. Dynamic-symbol
inventory additionally exposes the low-level
`perform_expensive_operations(void)` function and the fixed-size global
`int array[256 * 1024]`.

The C source has no runtime option, mode, flag, element-type, format,
byte-order, variable-size, or feature branch. Its only two operational axes
are:

- initial values in all 262,144 signed-int array elements for the low-level
  operation;
- the complete 32-bit unsigned seed domain for end-to-end execution.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `perform_expensive_operations`, `array` | Fixed 262,144-element `int` array; randomized full-domain `int` values plus `INT_MIN`, negative, zero, positive, and `INT_MAX` boundaries; one invocation performs 100 arithmetic rounds per element. | [x] |
| 2 | `long_exec`, `array` | End-to-end fixed-size initialization and 2,000 transformation passes; randomized `unsigned int` seeds plus `0` and `UINT_MAX`; compare observable stdout and all final array bytes. | [x] |

No Cargo features are declared in `Cargo.toml`, so the sole feature
configuration is the default/no-feature build.
