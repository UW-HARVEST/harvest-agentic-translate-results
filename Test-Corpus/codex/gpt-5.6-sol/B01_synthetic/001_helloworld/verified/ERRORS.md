# Error Surface

Mechanical scan scope: every C and header file below `c_src/`, excluding
generated files in `c_src/build/`.

Searched constructs: error-return macros, `return -1`, `return NULL`, error
enums, assertions, conditionals, switches, preprocessor branches, null checks,
and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are zero rejection rows. The only public entry point is `int main()`;
it accepts no inputs and contains no rejection, assertion, range, null, length,
or enum handling. The generic FFI invalid-input boundaries are therefore not
applicable to this API.

- [x] All zero error-surface rows are covered.
