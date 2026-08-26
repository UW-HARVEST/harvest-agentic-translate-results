# Error Surface

Mechanical scan inputs:

```text
rg -n '#if|#ifdef|#ifndef|\bif\s*\(|\bswitch\s*\(|\bcase\b|\breturn\b|assert|ERROR|NULL|MIN|MAX|enum|typedef|#define' c_src
```

The public C source contains no error-return macro, error enum, assertion,
range check, null check, or min/max constant. Therefore there are zero
source-level rejection rows.

## Mandatory FFI Boundaries

These rows cover the generic Phase C boundaries even though the C function
does not reject them. They deliberately record the C behavior rather than
assigning an invented error code. The two index cases are undefined behavior
in the C abstract machine; differential expectations refer to the built
ground-truth shared object used by the tests.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|---|
| 1 | `hdr_bitrate` | `h == NULL` | process terminates with `SIGSEGV` while evaluating `h[1]` | [x] |
| 2 | `hdr_bitrate` | layer bits `((h[1] >> 1) & 3) == 0`, one below the valid table-index range | reads the built C object's adjacent static storage and returns the resulting byte multiplied by 2 | [x] |
| 3 | `hdr_bitrate` | bitrate nibble `(h[2] >> 4) == 15`, one above the table-index maximum 14 | reads the built C object's next adjacent table/storage byte and returns it multiplied by 2 | [x] |

Length boundaries do not apply because the API accepts no length. Enum
boundaries do not apply because the API declares no enum parameter. The
function unconditionally reads exactly `h[1]` and `h[2]`.
