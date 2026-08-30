# Configuration Surface

The public dynamic-symbol surface contains `printLine`, `bad`, `good`, and
`driver`. The only runtime branch controlled by a caller is C truthiness of
`driver`'s `useGood` argument. `printLine` separately branches on nullness;
its null case is tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `printLine` | Non-null NUL-terminated C string; randomized empty, one-byte, and multi-byte payloads | [x] |
| 2 | `bad` | No options or input; execute the exported bad path end to end | [x] |
| 3 | `good` | No options or input; execute the exported good path end to end | [x] |
| 4 | `driver` | `useGood == 0`; dispatch to `bad` | [x] |
| 5 | `driver` | `useGood != 0`; randomized positive and negative C `int` values, including `INT_MIN` and `INT_MAX`; dispatch to `good` | [x] |

Feature combinations derived from `Cargo.toml`:

| # | Cargo features | C preprocessor configuration | Verified |
|---|----------------|------------------------------|----------|
| 1 | Empty set (the manifest has no `[features]` table) | No configurable `#ifdef` branches | [x] |
