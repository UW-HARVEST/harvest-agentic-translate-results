# Configuration Surface

Mechanical review covered the complete public header and all branches in the C
implementation. The API has no runtime options, modes, flags, element types,
lengths, formats, byte-order choices, conditional-compilation branches, or
features.

| # | entry point(s) | configuration (options set + input shape) | Covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `next_double` | Direct low-level call with every possible shape of the complete two-`uint64_t` state; include boundary states and many fixed-seed randomized states, comparing the returned `double` bits and both mutated state words | [x] |

Covered by `configuration_1_boundary_and_randomized_states_match` under the
default and no-default-feature invocations.
