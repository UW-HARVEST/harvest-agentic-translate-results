# Differential Error Log

No C/Rust mismatches were found, so no Rust fixes were required.

## Verification scope

- The integration suite covers every reachable argument-count, conversion,
  bounds, and ordering branch in the C executable.
- It includes empty input, one argument, the maximum accepted argument count,
  every reachable error return, integer overflow/truncation, partial numeric
  conversions, and non-UTF-8 argument bytes.
- An additional deterministic matrix compared 4,924 invocations of the C and
  Rust release executables.

## Unreachable C branch

The `Third argument must be an integer!` branch is not reachable when the
program is launched as a subprocess. The C code passes `NULL` to the third
`strtol` call and then compares `argv[3]` with the stale end pointer from
parsing `argv[2]`. Invalid third arguments therefore convert to zero and reach
the later bounds or ordering checks. The Rust executable preserves that
behavior, and the integration suite covers it.
