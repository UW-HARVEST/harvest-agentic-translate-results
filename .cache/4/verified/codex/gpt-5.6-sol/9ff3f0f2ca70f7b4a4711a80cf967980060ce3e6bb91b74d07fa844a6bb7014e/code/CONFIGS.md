# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and Cargo metadata reports `"features":
{}`. `c_src/CMakeLists.txt` has no options or conditional sources. There is
exactly one valid feature combination:

| # | Cargo invocation feature set | CMake configuration | verified |
|---|------------------------------|---------------------|----------|
| B1 | `--no-default-features` (empty set) | default, unconditional shared library | [x] |

## Runtime configurations

The only public entry point is the lowest-level and complete API:
`float ldexp_q2(float y, int exp_q2)`.

The rows below come from the C ternary at `c_src/src/lib.c:8`, table index and
shift at line 9, and loop condition at line 10. Every row is exercised across
many randomized `float` bit patterns, including signed zero, subnormal, normal,
infinity, and NaN values. The C source does not branch on the class or sign of
`y`, so those are randomized data values rather than separate configuration
rows.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| C1 | `ldexp_q2` | `exp_q2 < 0`, table index `exp_q2 & 3 == 0`; built-reference behavior for the C negative shift | [x] |
| C2 | `ldexp_q2` | `exp_q2 < 0`, table index `exp_q2 & 3 == 1`; built-reference behavior for the C negative shift | [x] |
| C3 | `ldexp_q2` | `exp_q2 < 0`, table index `exp_q2 & 3 == 2`; built-reference behavior for the C negative shift | [x] |
| C4 | `ldexp_q2` | `exp_q2 < 0`, table index `exp_q2 & 3 == 3`; built-reference behavior for the C negative shift | [x] |
| C5 | `ldexp_q2` | `exp_q2 == 0`; do-while executes once, selects table index 0 and shift 0 | [x] |
| C6 | `ldexp_q2` | `0 < exp_q2 < 120`, terminal table index 0; one loop iteration and shift quotients 1 through 29 | [x] |
| C7 | `ldexp_q2` | `0 < exp_q2 < 120`, terminal table index 1; one loop iteration and shift quotients 0 through 29 | [x] |
| C8 | `ldexp_q2` | `0 < exp_q2 < 120`, terminal table index 2; one loop iteration and shift quotients 0 through 29 | [x] |
| C9 | `ldexp_q2` | `0 < exp_q2 < 120`, terminal table index 3; one loop iteration and shift quotients 0 through 29 | [x] |
| C10 | `ldexp_q2` | `exp_q2 == 120`; ternary selects the 120 cap and loop terminates | [x] |
| C11 | `ldexp_q2` | `exp_q2 > 120`, terminal remainder 0; one or many full 120-unit chunks | [x] |
| C12 | `ldexp_q2` | `exp_q2 > 120`, terminal remainder with table index 1; repeated loop | [x] |
| C13 | `ldexp_q2` | `exp_q2 > 120`, terminal remainder with table index 2; repeated loop | [x] |
| C14 | `ldexp_q2` | `exp_q2 > 120`, terminal remainder with table index 3; repeated loop | [x] |
| C15 | `ldexp_q2` | `exp_q2 > 120`, nonzero terminal remainder with table index 0; repeated loop | [x] |
| C16 | `ldexp_q2` | large positive `exp_q2`, including `INT_MAX`; many repeated 120-unit chunks and integer boundary | [x] |

## Completion

- [x] Every runtime row passes byte-for-byte differential tests.
- [x] Every build-time configuration passes Phases B and C.
