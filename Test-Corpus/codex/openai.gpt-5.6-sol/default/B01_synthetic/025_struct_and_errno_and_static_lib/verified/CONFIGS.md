# Configuration-surface table

The library has no compile-time Cargo features and no C preprocessor feature
branches. Its runtime axes are:

- entry point: low-level `run(int)` or composed `driver(const char *)`;
- mutable house state: one call versus repeated calls;
- `run` argument class: negative, zero, positive, and C `int` boundaries;
- `driver` input shape as consumed by `strtol(..., 10)`: sign, leading
  whitespace, trailing bytes, embedded NUL, and C `int` boundaries.

Each row is exercised with many deterministic randomized values where the
configuration has a value range.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `run` | one isolated call; negative `extra_bedrooms` values away from overflow | [x] |
| 2 | `run` | one isolated call; `extra_bedrooms == 0` | [x] |
| 3 | `run` | one isolated call; positive `extra_bedrooms` values away from overflow | [x] |
| 4 | `run` | one isolated call; `extra_bedrooms == INT_MIN` | [x] |
| 5 | `run` | one isolated call; `extra_bedrooms == INT_MAX` | [x] |
| 6 | `run` | multiple sequential calls in one loaded process, exercising accumulated floors, bathrooms, and bedrooms | [x] |
| 7 | `driver` | canonical decimal C strings spanning negative, zero, and positive `int` values | [x] |
| 8 | `driver` | accepted decimal with leading C whitespace | [x] |
| 9 | `driver` | accepted decimal with an explicit `+` or `-` sign | [x] |
| 10 | `driver` | accepted decimal prefix followed by nonnumeric suffix bytes | [x] |
| 11 | `driver` | accepted value followed by an embedded NUL; bytes after the NUL are ignored | [x] |
| 12 | `driver` | exact `INT_MIN` decimal boundary | [x] |
| 13 | `driver` | exact `INT_MAX` decimal boundary | [x] |
| 14 | `driver` → `run` | composed path calls `run` twice and preserves all intermediate mutable-state output | [x] |

## Feature combinations

`Cargo.toml` declares no `[features]` table. The only build configuration is
the empty/default feature set, also exercised explicitly with
`--no-default-features`.

- [x] Default feature invocation
- [x] Explicit `--no-default-features` invocation
