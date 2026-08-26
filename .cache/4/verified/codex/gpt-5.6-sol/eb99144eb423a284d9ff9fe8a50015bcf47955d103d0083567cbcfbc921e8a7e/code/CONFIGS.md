# Configuration Surface

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or conditional sources. Therefore the only valid build-time feature
combination is the empty feature set:

```text
--no-default-features --features ""
```

The public header declares one entry point. Its implementation has no runtime
option, mode, flag, conditional, switch, or input-shape branch.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(int x)` | No options; scalar `int` across the complete C `int` domain, including zero, positive and negative values, and both boundaries | [x] |
