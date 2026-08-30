// Test-case modules, grouped by verification phase.
//
// These live under `tests/parts/` (not directly in `tests/`) so cargo does not
// treat them as separate test targets; `autotests = false` in Cargo.toml also
// enforces that.

pub mod first_call;
pub mod phase_b;
pub mod phase_c;
pub mod phase_d;
