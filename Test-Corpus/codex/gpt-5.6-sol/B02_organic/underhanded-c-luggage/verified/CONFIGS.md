# Configuration Surface

## Build-Time Configurations

`Cargo.toml` declares no `[features]` table. `c_src/CMakeLists.txt` declares no
options or conditional sources. There is exactly one valid combination:

| # | Rust features | C configuration | checked |
|---|---------------|-----------------|---------|
| B01 | empty set (`--no-default-features`) | default source, PIC enabled | [x] |

## Runtime and Input Configurations

Rows are derived from the public entry points and every `if`, loop, scan width,
filter flag, and special input shape in `c_src/src/luggage.c`.

| # | entry point(s) | configuration (options set + input shape) | checked |
|---|----------------|--------------------------------------------|---------|
| C01 | `addRoutingDirectiveToList` | empty tail (`next_directive == NULL`) | [x] |
| C02 | `addRoutingDirectiveToList` | insert before a greater timestamp | [x] |
| C03 | `addRoutingDirectiveToList` | recurse then insert between timestamps | [x] |
| C04 | `addRoutingDirectiveToList` | recurse to end; equal timestamps remain stable | [x] |
| C05 | `supersedes` | empty list | [x] |
| C06 | `supersedes` | first luggage matches and departure matches | [x] |
| C07 | `supersedes` | mismatched luggage nodes precede a matching luggage/departure | [x] |
| C08 | `supersedes` | no luggage match through a many-node list | [x] |
| C09 | `supersedes` | first luggage match has a different departure | [x] |
| C10 | `supersedes` | first luggage match differs in departure, later node would match; early `0` wins | [x] |
| C11 | `superseded` | no later directive | [x] |
| C12 | `superseded` | later same luggage and same departure | [x] |
| C13 | `superseded` | later same luggage and different departure | [x] |
| C14 | `superseded` | unrelated nodes before the decisive later directive | [x] |
| C15 | `matches` | expected begins with `-` wildcard (including unused null actual) | [x] |
| C16 | `matches` | exact string equality | [x] |
| C17 | `matches` | non-wildcard mismatch | [x] |
| C18 | `printMatchingDirectives` | null/empty directive list | [x] |
| C19 | `printMatchingDirectives`, `main` | one unsuperseded directive; all four filters exact | [x] |
| C20 | `printMatchingDirectives`, `main` | filter mask `EEEE` (exact luggage/flight/departure/arrival) | [x] |
| C21 | `printMatchingDirectives`, `main` | filter mask `WEEE` | [x] |
| C22 | `printMatchingDirectives`, `main` | filter mask `EWEE` | [x] |
| C23 | `printMatchingDirectives`, `main` | filter mask `WWEE` | [x] |
| C24 | `printMatchingDirectives`, `main` | filter mask `EEWE` | [x] |
| C25 | `printMatchingDirectives`, `main` | filter mask `WEWE` | [x] |
| C26 | `printMatchingDirectives`, `main` | filter mask `EWWE` | [x] |
| C27 | `printMatchingDirectives`, `main` | filter mask `WWWE` | [x] |
| C28 | `printMatchingDirectives`, `main` | filter mask `EEEW` | [x] |
| C29 | `printMatchingDirectives`, `main` | filter mask `WEEW` | [x] |
| C30 | `printMatchingDirectives`, `main` | filter mask `EWEW` | [x] |
| C31 | `printMatchingDirectives`, `main` | filter mask `WWEW` | [x] |
| C32 | `printMatchingDirectives`, `main` | filter mask `EEWW` | [x] |
| C33 | `printMatchingDirectives`, `main` | filter mask `WEWW` | [x] |
| C34 | `printMatchingDirectives`, `main` | filter mask `EWWW` | [x] |
| C35 | `printMatchingDirectives`, `main` | filter mask `WWWW` | [x] |
| C36 | `printMatchingDirectives` | luggage filter is first nonmatching predicate | [x] |
| C37 | `printMatchingDirectives` | flight filter is first nonmatching predicate | [x] |
| C38 | `printMatchingDirectives` | departure filter is first nonmatching predicate | [x] |
| C39 | `printMatchingDirectives` | arrival filter is nonmatching predicate | [x] |
| C40 | `printMatchingDirectives`, `main` | same luggage/departure later: earlier directive suppressed | [x] |
| C41 | `printMatchingDirectives`, `main` | same luggage/different departure later: earlier directive retained | [x] |
| C42 | `printMatchingDirectives`, `main` | many unsorted inputs become timestamp-sorted output | [x] |
| C43 | `main` | empty stdin | [x] |
| C44 | `main` | one record with empty comment (newline immediately follows arrival) | [x] |
| C45 | `main` | one record with a one-byte comment | [x] |
| C46 | `main` | one record with an 80-byte maximum-width comment | [x] |
| C47 | `main` | one record with maximum-width luggage, flight, and airport fields | [x] |
| C48 | `main` | fields shorter than their scan widths | [x] |
| C49 | `main` | luggage is one byte past width; excess byte becomes the flight scan | [x] |
| C50 | `main` | duplicate timestamps preserve input order | [x] |
| C51 | `main` | timestamp `0` | [x] |
| C52 | `main` | positive signed-int timestamp boundary | [x] |
| C53 | `main` | negative timestamp is stored and printed through the unsigned object | [x] |
| C54 | `main` | C whitespace variants and whitespace spanning line boundaries | [x] |
| C55 | `main` | multiple records with optional and nonempty comments | [x] |
| C56 | `main` | flight is one byte past width; excess byte becomes the departure scan | [x] |
| C57 | `main` | departure is one byte past width; excess byte becomes the arrival scan | [x] |
| C58 | `main` | arrival is one byte past width; excess byte becomes the comment scan | [x] |
| C59 | `main` | comment is one byte past width; excess byte begins the next timestamp | [x] |
| C60 | `main` | timestamp is one past the signed-int maximum | [x] |
