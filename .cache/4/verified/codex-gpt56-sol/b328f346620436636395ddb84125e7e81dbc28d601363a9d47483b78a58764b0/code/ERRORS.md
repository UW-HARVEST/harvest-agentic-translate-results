# Error Surface

Mechanical inspection covered every `return`, assertion, comparison,
preprocessor branch, null check, range check, and min/max constant in
`c_src/include/lib.h` and `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

There are no rejection or error paths. `to_barycentric` takes four structures
by value, has no pointers, lengths, enums, options, assertions, or range
checks, and always returns an `lm_vec2`. Degenerate and non-finite inputs flow
through the same IEEE-754 arithmetic as ordinary inputs and are covered as
valid configurations in `CONFIGS.md`.

- [x] No applicable null-pointer boundary
- [x] No applicable zero/oversized-length boundary
- [x] No applicable enum boundary
- [x] Every C rejection branch is represented (there are zero)

