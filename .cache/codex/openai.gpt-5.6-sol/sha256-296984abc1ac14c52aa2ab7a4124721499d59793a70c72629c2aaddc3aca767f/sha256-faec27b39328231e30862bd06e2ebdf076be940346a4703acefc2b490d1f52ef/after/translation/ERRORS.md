# Differential Verification Errors

No mismatches were found in Phases B or C. The original Rust translation
matched the C executable for every enumerated parsing error, operation branch,
numeric conversion boundary, maximum-length case, and combined-flag case, so
no Rust implementation correction was required.

The source audit also identified helper branches that cannot be reached through
the executable's input interface:

- `process_buffer` cannot receive a null buffer from `main`.
- `rotate_buffer` cannot receive a length at most one with a nonzero normalized
  offset, and cannot receive a zero offset.
- `compact_runs` keeps `read` and `write` equal on its non-compacting path, so
  its `write != read` move is unreachable.
- `interleave_halves` cannot receive a length below two, and `half > 256` is
  impossible because `main` rejects lengths above 256.
- `reverse_segments` cannot receive `len < seg_size` because its caller checks
  `seg_size <= new_len`.
