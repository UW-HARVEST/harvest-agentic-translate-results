# Configuration Surface

The only public entry point is `jumpnode`. Modes `0001`, `0002`, and `0004`
are valid selectors but necessarily take rejection paths because the private
node store cannot be initialized through the public API; they are covered in
`ERRORS.md`. Unknown selectors are also error configurations.

Mode `0003` is the complete reachable valid surface. Its two `%d` inputs each
have three formatting shapes (negative, zero, positive). The flags input is
partitioned by every materially distinct interaction with `flags & 0177`:
zero; nonzero low-seven bits only; nonzero high bits only (masked result zero);
positive mixed high/low bits; and negative mixed high/low bits. Every row uses
many values and decimal widths within its stated shape, including `INT_MIN`
and `INT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jumpnode` | mode `0003`; node ID negative; depth negative; flags zero | [x] |
| 2 | `jumpnode` | mode `0003`; node ID negative; depth negative; flags low-only nonzero | [x] |
| 3 | `jumpnode` | mode `0003`; node ID negative; depth negative; flags high-only | [x] |
| 4 | `jumpnode` | mode `0003`; node ID negative; depth negative; flags positive mixed | [x] |
| 5 | `jumpnode` | mode `0003`; node ID negative; depth negative; flags negative mixed | [x] |
| 6 | `jumpnode` | mode `0003`; node ID negative; depth zero; flags zero | [x] |
| 7 | `jumpnode` | mode `0003`; node ID negative; depth zero; flags low-only nonzero | [x] |
| 8 | `jumpnode` | mode `0003`; node ID negative; depth zero; flags high-only | [x] |
| 9 | `jumpnode` | mode `0003`; node ID negative; depth zero; flags positive mixed | [x] |
| 10 | `jumpnode` | mode `0003`; node ID negative; depth zero; flags negative mixed | [x] |
| 11 | `jumpnode` | mode `0003`; node ID negative; depth positive; flags zero | [x] |
| 12 | `jumpnode` | mode `0003`; node ID negative; depth positive; flags low-only nonzero | [x] |
| 13 | `jumpnode` | mode `0003`; node ID negative; depth positive; flags high-only | [x] |
| 14 | `jumpnode` | mode `0003`; node ID negative; depth positive; flags positive mixed | [x] |
| 15 | `jumpnode` | mode `0003`; node ID negative; depth positive; flags negative mixed | [x] |
| 16 | `jumpnode` | mode `0003`; node ID zero; depth negative; flags zero | [x] |
| 17 | `jumpnode` | mode `0003`; node ID zero; depth negative; flags low-only nonzero | [x] |
| 18 | `jumpnode` | mode `0003`; node ID zero; depth negative; flags high-only | [x] |
| 19 | `jumpnode` | mode `0003`; node ID zero; depth negative; flags positive mixed | [x] |
| 20 | `jumpnode` | mode `0003`; node ID zero; depth negative; flags negative mixed | [x] |
| 21 | `jumpnode` | mode `0003`; node ID zero; depth zero; flags zero | [x] |
| 22 | `jumpnode` | mode `0003`; node ID zero; depth zero; flags low-only nonzero | [x] |
| 23 | `jumpnode` | mode `0003`; node ID zero; depth zero; flags high-only | [x] |
| 24 | `jumpnode` | mode `0003`; node ID zero; depth zero; flags positive mixed | [x] |
| 25 | `jumpnode` | mode `0003`; node ID zero; depth zero; flags negative mixed | [x] |
| 26 | `jumpnode` | mode `0003`; node ID zero; depth positive; flags zero | [x] |
| 27 | `jumpnode` | mode `0003`; node ID zero; depth positive; flags low-only nonzero | [x] |
| 28 | `jumpnode` | mode `0003`; node ID zero; depth positive; flags high-only | [x] |
| 29 | `jumpnode` | mode `0003`; node ID zero; depth positive; flags positive mixed | [x] |
| 30 | `jumpnode` | mode `0003`; node ID zero; depth positive; flags negative mixed | [x] |
| 31 | `jumpnode` | mode `0003`; node ID positive; depth negative; flags zero | [x] |
| 32 | `jumpnode` | mode `0003`; node ID positive; depth negative; flags low-only nonzero | [x] |
| 33 | `jumpnode` | mode `0003`; node ID positive; depth negative; flags high-only | [x] |
| 34 | `jumpnode` | mode `0003`; node ID positive; depth negative; flags positive mixed | [x] |
| 35 | `jumpnode` | mode `0003`; node ID positive; depth negative; flags negative mixed | [x] |
| 36 | `jumpnode` | mode `0003`; node ID positive; depth zero; flags zero | [x] |
| 37 | `jumpnode` | mode `0003`; node ID positive; depth zero; flags low-only nonzero | [x] |
| 38 | `jumpnode` | mode `0003`; node ID positive; depth zero; flags high-only | [x] |
| 39 | `jumpnode` | mode `0003`; node ID positive; depth zero; flags positive mixed | [x] |
| 40 | `jumpnode` | mode `0003`; node ID positive; depth zero; flags negative mixed | [x] |
| 41 | `jumpnode` | mode `0003`; node ID positive; depth positive; flags zero | [x] |
| 42 | `jumpnode` | mode `0003`; node ID positive; depth positive; flags low-only nonzero | [x] |
| 43 | `jumpnode` | mode `0003`; node ID positive; depth positive; flags high-only | [x] |
| 44 | `jumpnode` | mode `0003`; node ID positive; depth positive; flags positive mixed | [x] |
| 45 | `jumpnode` | mode `0003`; node ID positive; depth positive; flags negative mixed | [x] |
