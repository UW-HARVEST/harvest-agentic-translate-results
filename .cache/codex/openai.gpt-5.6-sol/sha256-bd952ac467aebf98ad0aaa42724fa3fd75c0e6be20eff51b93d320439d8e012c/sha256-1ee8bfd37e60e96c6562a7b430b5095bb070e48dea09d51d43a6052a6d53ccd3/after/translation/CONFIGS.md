# Configuration Surface

The public headers expose one entry point and no runtime options:

```c
int memchra2(int a, int b, int c, int d);
```

The C source creates fixed four-element arrays, fixed string inputs, a 64-byte
native `char` buffer, and a four-byte native-endian integer representation.
On this build target, `int` is 32-bit and the ELF data encoding is little
endian. The public input always has exactly four elements; empty, one-element,
pointer, length, enum, format, and byte-order modes are not public
configurations.

The data-dependent branches form two axes:

1. The bits of `a`, reinterpreted as `float`, select six distinguishable paths:
   negative-sign finite/infinity/NaN (first comparison false), positive zero
   (first comparison false), positive `(0,1)` (gate true, cast adds zero),
   `[1,1000)` (gate true, cast adds a positive integer), `[1000,+infinity]`
   (second comparison false), and positive NaN (comparison false).
2. `snprintf("test%d-%d-%d-%d", ...)` adds one extra `'-'` for each negative
   decimal argument. `a`'s sign is fixed by axis 1; all eight sign masks for
   `b/c/d` are distinct input shapes. Random values in every row also exercise
   digit counts, native low bytes (including high-bit bytes), wrapping integer
   arithmetic, XOR, and the fixed true `buf_sum > 0` path.

Each row is run with fixed boundary cases and 128 seeded randomized cases.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `+++` | [x] |
| 2 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `-++` | [x] |
| 3 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `+-+` | [x] |
| 4 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `++-` | [x] |
| 5 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `--+` | [x] |
| 6 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `-+-` | [x] |
| 7 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `+--` | [x] |
| 8 | `memchra2` | `a`: negative-sign float bits; `b/c/d`: `---` | [x] |
| 9 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `+++` | [x] |
| 10 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `-++` | [x] |
| 11 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `+-+` | [x] |
| 12 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `++-` | [x] |
| 13 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `--+` | [x] |
| 14 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `-+-` | [x] |
| 15 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `+--` | [x] |
| 16 | `memchra2` | `a`: positive zero float bits; `b/c/d`: `---` | [x] |
| 17 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `+++` | [x] |
| 18 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `-++` | [x] |
| 19 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `+-+` | [x] |
| 20 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `++-` | [x] |
| 21 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `--+` | [x] |
| 22 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `-+-` | [x] |
| 23 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `+--` | [x] |
| 24 | `memchra2` | `a`: float in `(0,1)`; `b/c/d`: `---` | [x] |
| 25 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `+++` | [x] |
| 26 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `-++` | [x] |
| 27 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `+-+` | [x] |
| 28 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `++-` | [x] |
| 29 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `--+` | [x] |
| 30 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `-+-` | [x] |
| 31 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `+--` | [x] |
| 32 | `memchra2` | `a`: float in `[1,1000)`; `b/c/d`: `---` | [x] |
| 33 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `+++` | [x] |
| 34 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `-++` | [x] |
| 35 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `+-+` | [x] |
| 36 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `++-` | [x] |
| 37 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `--+` | [x] |
| 38 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `-+-` | [x] |
| 39 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `+--` | [x] |
| 40 | `memchra2` | `a`: float in `[1000,+infinity]`; `b/c/d`: `---` | [x] |
| 41 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+++` | [x] |
| 42 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `-++` | [x] |
| 43 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+-+` | [x] |
| 44 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `++-` | [x] |
| 45 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `--+` | [x] |
| 46 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `-+-` | [x] |
| 47 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `+--` | [x] |
| 48 | `memchra2` | `a`: positive NaN bits; `b/c/d`: `---` | [x] |
