# Error Surface

Mechanical source search:

```sh
rg -n -i \
  'RETURN_ERROR|return\s+(-1|NULL)|\bassert\s*\(|\bif\s*\(|\bswitch\s*\(|#\s*if(n?def)?|\bNULL\b|\benum\b|\b(MIN|MAX)\b' \
  ../c_src/src ../c_src/include
```

The search has no matches. The sole public function, `md5_digest`, returns
`void` and the C source performs no explicit rejection, null check, assertion,
range check, enum validation, or error return. Therefore there are no
source-derived rejection rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| — | — | No explicit C rejection paths exist. | — | N/A |

Generic FFI pointer-boundary behavior is tested separately in Phase C because
null pointers violate the C function's pointer contract and are not explicit
rejection branches in the source. Length and enum boundary cases are
inapplicable: this API accepts neither a length nor an enum.

Generic Phase C boundary coverage:

- [x] Null `m`: C and Rust terminate with the same signal in isolated probes.
- [x] Null `out`: C and Rust terminate with the same signal in isolated probes.
- [x] Zero/oversized lengths: not applicable; no length parameter exists.
- [x] Out-of-range enums: not applicable; no enum parameter exists.
