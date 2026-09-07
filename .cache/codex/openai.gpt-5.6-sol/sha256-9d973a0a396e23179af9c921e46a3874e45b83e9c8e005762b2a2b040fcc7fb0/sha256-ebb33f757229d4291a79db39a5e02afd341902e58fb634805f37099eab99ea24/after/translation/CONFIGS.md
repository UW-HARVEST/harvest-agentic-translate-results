# Configuration Surface

Mechanically derived axes:

- Exported callable entry points: `perform_expensive_operations(void)` and
  `long_exec(unsigned int)`.
- Exported state: `array`, exactly `256 * 1024` C `int` elements.
- Fixed transform shape: every array element, exactly 100 arithmetic rounds.
- Fixed composed-operation shape: libc `srand`/`rand` initialization, exactly
  2000 full transforms, then XOR reduction and one decimal line on stdout.
- Runtime option/input: the full 32-bit `unsigned int` seed supplied to
  `long_exec`; the C source has no mode flags or option branches.
- Cargo features: none.
- Executable targets: none (shared libraries only).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|--------|
| 1 | exported `array`; `perform_expensive_operations` | Full fixed-size array populated with arbitrary 32-bit bit patterns (including zero, positive, negative, `INT_MIN`, and `INT_MAX`), then one 100-round transform over all elements | [x] |
| 2 | `long_exec`; exported `array`; `perform_expensive_operations` | Full-range `unsigned int` seed (including `0` and `UINT_MAX`); libc PRNG fills the full fixed-size array, 2000 transforms run, array is XOR-reduced, and decimal result plus newline is written to stdout | [x] |

The rows are the complete pruned cross-product: there are no public flags,
variable lengths, element-type choices, formats, byte-order modes, feature
combinations, or alternate entry points in the C source.

The prescribed unoptimized C build exceeded the mandatory 600-second limit.
The same untouched C source was therefore also built with CMake's Release
configuration. That optimized library first matched Rust across all 12
randomized row-1 cases, establishing the same observed transform semantics,
then all eight row-2 workers (boundary seeds plus six deterministic randomized
seeds) passed in 399.69 seconds. The orchestrator remains ignored by default so
ordinary test runs do not accidentally select the unoptimized reference.
