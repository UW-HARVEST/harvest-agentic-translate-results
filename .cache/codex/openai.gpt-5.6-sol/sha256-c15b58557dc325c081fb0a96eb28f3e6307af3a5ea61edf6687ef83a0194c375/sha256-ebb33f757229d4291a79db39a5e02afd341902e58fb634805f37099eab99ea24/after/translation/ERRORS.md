# Error Surface

The following source scan was used:

```text
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|return\s+[A-Z_]*ERROR|assert\s*\(|if\s*\(|switch\s*\(|#if|#ifdef|#ifndef|NULL|MIN|MAX|enum' ../c_src/include/lib.h ../c_src/src/lib.c
```

It found no rejection branch, error return, assertion, null check, explicit
range check, error enum, or min/max input constant. `crc16` always returns a
`uint16_t` CRC for inputs satisfying its pointer/length precondition.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are therefore **0 explicit C rejection rows**.

## Generic FFI boundaries

| Boundary | C behavior | Coverage |
|----------|------------|----------|
| `d == NULL`, `len == 0` | Defined: no dereference; returns the initial CRC unchanged | [x] |
| non-null `d`, `len == 0` | Defined: no dereference; returns the initial CRC unchanged | [x] |
| large valid length (16 MiB + 7 bytes) | Defined: exercises sustained slicing and a maximum-size tail | [x] |
| `d == NULL`, `len > 0` | Undefined behavior in C; there is no error/sentinel to compare | N/A |
| `len` exceeds the readable allocation | Undefined behavior in C; there is no error/sentinel to compare | N/A |
| out-of-range enum | No enum parameters exist | N/A |
| one past a documented numeric range | No narrower documented range exists; all `uint16_t` CRC and `uint32_t` length values are representable | N/A |
