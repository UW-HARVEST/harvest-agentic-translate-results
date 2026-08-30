# Differential Mismatch Log

No mismatches were found.

The initial translation built successfully and matched the C executable for
normal input (`1.5`), a conversion failure (`bad`), and empty input. All 25
branch, parsing, boundary, and range cases in `tests/differential.rs` also
matched for stdout, stderr, and exit status. No Rust implementation fix was
required.
