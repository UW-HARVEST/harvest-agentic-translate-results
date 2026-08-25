# Error Surface

Mechanical searches covered `return`, `assert`, null checks, range checks,
error names, and min/max constants in `c_src/src/` and `c_src/include/`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

There are no rejection paths. The only public API accepts one scalar `float`;
it has no pointer, length, enum, option, or documented range boundary.

- [x] All error-surface rows are covered (zero rows).
- [x] Generic pointer, length, range, and enum boundaries are not applicable.
