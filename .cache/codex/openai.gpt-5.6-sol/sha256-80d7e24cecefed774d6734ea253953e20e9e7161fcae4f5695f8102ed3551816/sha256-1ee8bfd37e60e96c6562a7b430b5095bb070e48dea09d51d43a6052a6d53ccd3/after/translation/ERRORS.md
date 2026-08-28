# Error Surface

The complete `c_src/src/lib.c` and `c_src/include/lib.h` surface contains no
error-return macro or statement, error enum, assertion, explicit range check,
null check, or min/max constant. `hsv_to_rgb` returns `void` and assumes both
pointers address at least three `float` elements.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are therefore zero C rejection branches to enumerate or check off.
Null pointers are outside the C function's contract and cause process-level
undefined behavior rather than a reported rejection; differential subprocess
tests cover the observed behavior without crashing the test runner.

The API has no length or enum parameter, and its header documents no numeric
range. Zero/oversized lengths and out-of-range enum values are therefore not
applicable. The valid-path tests still cover adjacent floats around every
sector boundary and around the conventional HSV upper bound of `1.0`.
