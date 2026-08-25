# Configuration surface

## Build-time configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` declares a feature, option,
conditional source, or backend. There is exactly one valid build-time
configuration:

| # | Cargo invocation | CMake configuration | [x] |
|---|------------------|---------------------|-----|
| B01 | `--no-default-features` (empty feature set; omit `--features`) | default shared-library target | [x] |

## Runtime configurations

For `unfilter`, each positive-dimension row is exercised with `bpp` equal to
one and many bytes and with width zero, one, and many pixels. For `cp_inflate`,
random payloads cover empty, one-byte, and many-byte output shapes. This keeps
the table finite while fully crossing the branch axes that the C code actually
distinguishes.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C01 | exported data | all seven initialized exported tables have byte-identical initial contents; `cp_error_reason` is initially null | [x] |
| C02 | `cp_inflate` | final stored block; empty payload; exact output capacity | [x] |
| C03 | `cp_inflate` | final stored block; one-byte payload; exact and excess output capacity | [x] |
| C04 | `cp_inflate` | final stored block; many-byte random payload; exact and excess output capacity | [x] |
| C05 | `cp_inflate` | final fixed-Huffman block; empty payload (end symbol only) | [x] |
| C06 | `cp_inflate` | final fixed-Huffman block; one literal | [x] |
| C07 | `cp_inflate` | final fixed-Huffman block; many random literals | [x] |
| C08 | `cp_inflate` | fixed-Huffman match with backwards distance `1` (`memset` branch) | [x] |
| C09 | `cp_inflate` | fixed-Huffman match with backwards distance greater than `1` (copy loop branch) | [x] |
| C10 | `cp_inflate` | final dynamic-Huffman block; code-length symbols use ordinary lengths | [x] |
| C11 | `cp_inflate` | final dynamic-Huffman block; code-length repeat symbol `16` | [x] |
| C12 | `cp_inflate` | final dynamic-Huffman block; zero repeat symbol `17` | [x] |
| C13 | `cp_inflate` | final dynamic-Huffman block; long zero repeat symbol `18` | [x] |
| C14 | `cp_inflate` | dynamic block emits literals and end symbol | [x] |
| C15 | `cp_inflate` | dynamic block emits a length/distance match | [x] |
| C16 | `cp_inflate` | non-final fixed-Huffman block followed by a final fixed-Huffman block | [x] |
| C17 | `cp_inflate` | exact and excess output capacities for fixed and dynamic blocks | [x] |
| C18 | `cp_inflate` | input address aligned to 4 bytes (`first_bytes == 0`) | [x] |
| C19 | `cp_inflate` | input address modulo 4 is 1 (`first_bytes == 3`) | [x] |
| C20 | `cp_inflate` | input address modulo 4 is 2 (`first_bytes == 2`) | [x] |
| C21 | `cp_inflate` | input address modulo 4 is 3 (`first_bytes == 1`) | [x] |
| C22 | `cp_inflate` | bytes after the aligned prefix are divisible by 4 (`last_bytes == 0`) | [x] |
| C23 | `cp_inflate` | aligned-body tail has 1 byte (`last_bytes == 1`) | [x] |
| C24 | `cp_inflate` | aligned-body tail has 2 bytes (`last_bytes == 2`) | [x] |
| C25 | `cp_inflate` | aligned-body tail has 3 bytes (`last_bytes == 3`) | [x] |
| C26 | `unfilter` | `h <= 0`; no first-row switch and no later-row loop | [x] |
| C27 | `unfilter` | `h == 1`, first filter `0` | [x] |
| C28 | `unfilter` | `h == 1`, first filter `1` | [x] |
| C29 | `unfilter` | `h == 1`, first filter `2` | [x] |
| C30 | `unfilter` | `h == 1`, first filter `3` | [x] |
| C31 | `unfilter` | `h == 1`, first filter `4` | [x] |
| C32 | `unfilter` | `h > 1`, first/later filters `0/0` | [x] |
| C33 | `unfilter` | `h > 1`, first/later filters `0/1` | [x] |
| C34 | `unfilter` | `h > 1`, first/later filters `0/2` | [x] |
| C35 | `unfilter` | `h > 1`, first/later filters `0/3` | [x] |
| C36 | `unfilter` | `h > 1`, first/later filters `0/4` | [x] |
| C37 | `unfilter` | `h > 1`, first/later filters `1/0` | [x] |
| C38 | `unfilter` | `h > 1`, first/later filters `1/1` | [x] |
| C39 | `unfilter` | `h > 1`, first/later filters `1/2` | [x] |
| C40 | `unfilter` | `h > 1`, first/later filters `1/3` | [x] |
| C41 | `unfilter` | `h > 1`, first/later filters `1/4` | [x] |
| C42 | `unfilter` | `h > 1`, first/later filters `2/0` | [x] |
| C43 | `unfilter` | `h > 1`, first/later filters `2/1` | [x] |
| C44 | `unfilter` | `h > 1`, first/later filters `2/2` | [x] |
| C45 | `unfilter` | `h > 1`, first/later filters `2/3` | [x] |
| C46 | `unfilter` | `h > 1`, first/later filters `2/4` | [x] |
| C47 | `unfilter` | `h > 1`, first/later filters `3/0` | [x] |
| C48 | `unfilter` | `h > 1`, first/later filters `3/1` | [x] |
| C49 | `unfilter` | `h > 1`, first/later filters `3/2` | [x] |
| C50 | `unfilter` | `h > 1`, first/later filters `3/3` | [x] |
| C51 | `unfilter` | `h > 1`, first/later filters `3/4` | [x] |
| C52 | `unfilter` | `h > 1`, first/later filters `4/0` | [x] |
| C53 | `unfilter` | `h > 1`, first/later filters `4/1` | [x] |
| C54 | `unfilter` | `h > 1`, first/later filters `4/2` | [x] |
| C55 | `unfilter` | `h > 1`, first/later filters `4/3` | [x] |
| C56 | `unfilter` | `h > 1`, first/later filters `4/4` | [x] |
