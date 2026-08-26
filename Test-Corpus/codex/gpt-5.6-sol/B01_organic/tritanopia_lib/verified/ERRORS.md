# Error Surface

Mechanical scan:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert\s*\(|NULL|enum\b|min|max' \
  c_src/src c_src/include
```

The scan has no matches. The sole API takes and returns a fixed-size,
three-byte struct by value. It has no pointers, lengths, enum values, status
codes, sentinels, assertions, explicit range checks, or rejection branches.
Every possible input bit pattern is valid, so there are no error rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

Generic FFI boundaries:

| boundary | applicability |
|----------|---------------|
| Null pointers | Not applicable: no pointer arguments |
| Zero/oversized lengths | Not applicable: no length arguments |
| One past a documented range | Not applicable: all three fields span the complete `unsigned char` range |
| Out-of-range enum values | Not applicable: no enum arguments |

Completion:

- [x] Every explicit C rejection branch is represented (there are none).
- [x] Every generic FFI boundary is covered or mechanically shown inapplicable.
