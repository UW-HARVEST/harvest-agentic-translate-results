# Configuration Surface

The sole public and lowest-level entry point is `searchAndReplace`. It has no
runtime modes, flags, element types, formats, byte-order options, or Cargo
features. The matrix below is the cross-product of the source branches:

- first match at byte zero vs. after a nonempty prefix;
- no later match vs. adjacent later match vs. later match after a nonempty gap;
- no suffix vs. a nonempty suffix after the last match;
- empty vs. nonempty replacement.

No-match inputs bypass all replacement branches. Empty `orig` is listed
separately as the zero-size boundary. Empty `search` is also separate because
the C loop never advances and does not terminate.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `searchAndReplace` | empty `orig`, nonempty `search`, no match, empty or nonempty `value` | [x] |
| 2 | `searchAndReplace` | nonempty `orig`, nonempty `search`, no match, empty or nonempty `value` | [x] |
| 3 | `searchAndReplace` | first match at byte zero; no later match; no suffix; empty `value` | [x] |
| 4 | `searchAndReplace` | first match at byte zero; no later match; no suffix; nonempty `value` | [x] |
| 5 | `searchAndReplace` | first match at byte zero; no later match; nonempty suffix; empty `value` | [x] |
| 6 | `searchAndReplace` | first match at byte zero; no later match; nonempty suffix; nonempty `value` | [x] |
| 7 | `searchAndReplace` | first match at byte zero; adjacent later match; no suffix; empty `value` | [x] |
| 8 | `searchAndReplace` | first match at byte zero; adjacent later match; no suffix; nonempty `value` | [x] |
| 9 | `searchAndReplace` | first match at byte zero; adjacent later match; nonempty suffix; empty `value` | [x] |
| 10 | `searchAndReplace` | first match at byte zero; adjacent later match; nonempty suffix; nonempty `value` | [x] |
| 11 | `searchAndReplace` | first match at byte zero; later match after a nonempty gap; no suffix; empty `value` | [x] |
| 12 | `searchAndReplace` | first match at byte zero; later match after a nonempty gap; no suffix; nonempty `value` | [x] |
| 13 | `searchAndReplace` | first match at byte zero; later match after a nonempty gap; nonempty suffix; empty `value` | [x] |
| 14 | `searchAndReplace` | first match at byte zero; later match after a nonempty gap; nonempty suffix; nonempty `value` | [x] |
| 15 | `searchAndReplace` | first match after a nonempty prefix; no later match; no suffix; empty `value` | [x] |
| 16 | `searchAndReplace` | first match after a nonempty prefix; no later match; no suffix; nonempty `value` | [x] |
| 17 | `searchAndReplace` | first match after a nonempty prefix; no later match; nonempty suffix; empty `value` | [x] |
| 18 | `searchAndReplace` | first match after a nonempty prefix; no later match; nonempty suffix; nonempty `value` | [x] |
| 19 | `searchAndReplace` | first match after a nonempty prefix; adjacent later match; no suffix; empty `value` | [x] |
| 20 | `searchAndReplace` | first match after a nonempty prefix; adjacent later match; no suffix; nonempty `value` | [x] |
| 21 | `searchAndReplace` | first match after a nonempty prefix; adjacent later match; nonempty suffix; empty `value` | [x] |
| 22 | `searchAndReplace` | first match after a nonempty prefix; adjacent later match; nonempty suffix; nonempty `value` | [x] |
| 23 | `searchAndReplace` | first match after a nonempty prefix; later match after a nonempty gap; no suffix; empty `value` | [x] |
| 24 | `searchAndReplace` | first match after a nonempty prefix; later match after a nonempty gap; no suffix; nonempty `value` | [x] |
| 25 | `searchAndReplace` | first match after a nonempty prefix; later match after a nonempty gap; nonempty suffix; empty `value` | [x] |
| 26 | `searchAndReplace` | first match after a nonempty prefix; later match after a nonempty gap; nonempty suffix; nonempty `value` | [x] |
| 27 | `searchAndReplace` | empty `search`; the C implementation does not terminate | [x] |
