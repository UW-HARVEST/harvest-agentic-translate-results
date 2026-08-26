# Error Surface

Mechanical searches covered error-return statements/macros, assertions, null
checks, range checks, enums, and min/max constants in `c_src/include` and
`c_src/src`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

There are no rows because `hsv_to_rgb` has no rejection or error path. It
accepts three readable source floats and three writable destination floats and
returns `void`. Invalid pointers violate the C function's pointer preconditions
and invoke undefined behavior; the API has no length or enum arguments.
