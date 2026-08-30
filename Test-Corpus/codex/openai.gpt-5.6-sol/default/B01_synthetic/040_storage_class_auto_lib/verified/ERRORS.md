# Error Surface

Mechanical scans covered all C headers and sources for error returns, null and
range checks, assertions, switches, and conditional branches. The public API
has no rejection paths: `driver(int)` returns `void`, accepts the full C `int`
domain, and has no pointer, length, enum, or option parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

Generic FFI boundaries for null pointers, lengths, and out-of-range enums are
not applicable to this scalar-only API.
