# Error Surface

Mechanical scan:

```text
rg -n 'RETURN_ERROR|return\b|assert\s*\(|if\s*\(|switch\s*\(|case\b|default\s*:|NULL|[Mm][Ii][Nn]|[Mm][Aa][Xx]|enum|#if|#ifdef|#ifndef' ../c_src/include ../c_src/src
```

The source contains no error-return statements, assertions, null checks, range
checks, or min/max constants. The sole rejection behavior is the switch's
implicit handling of unsupported enum values.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `colourblind` | `Impairment < cbProtanopia` or `Impairment > cbTritanopia` (any integer other than 0, 1, or 2) | [x] Return `void` without reading or writing `R`, `G`, or `B`; null pointers are therefore also accepted in this condition |

For a valid impairment, any null pointer is dereferenced by the C source and
has undefined behavior rather than a defined rejection result. Length
boundaries do not apply because this API has no length argument.
