# Configuration Surface

Mechanical inputs:

- Public-header declaration search finds only `md5_digest`.
- Source searches find no `if`, `switch`, preprocessor feature branch, runtime
  option, mode, flag, enum, format selector, length, count, or element-type
  selector.
- The only input shape is one `tflac_md5` containing four `uint32_t` words and
  one writable output region of exactly 16 bytes.
- The C implementation always serializes `a`, `b`, `c`, and `d`, in that order,
  least-significant byte first.
- `Cargo.toml` declares no features and builds only a `cdylib`; neither project
  defines a binary executable.

Because the C code contains no control-flow branches, the mechanically pruned
cross-product has one meaningful valid configuration.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `md5_digest` | No options; four arbitrary `uint32_t` state words; writable 16-byte output; non-overlapping and overlapping state/output placements; 37 boundary/bit-pattern states plus 50,000 fixed-seed randomized states and 7,936 randomized overlap cases. | [x] |
