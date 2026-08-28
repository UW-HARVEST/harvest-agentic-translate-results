# Configuration Surface

Mechanically inspected the public header and all branches in
`../c_src/src/hello.c`. The API has no runtime options, flags, modes, input
data, conditional compilation, or alternate entry points.
`Cargo.toml` declares no features, so the empty feature set is the only
effective feature combination. It passes both the default and explicit
`--no-default-features` test invocations.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|-------------------------------------------|--------|
| 1 | `helloworld` | No options and no input; emits `Hello World!\n` and returns `0` | [x] |
