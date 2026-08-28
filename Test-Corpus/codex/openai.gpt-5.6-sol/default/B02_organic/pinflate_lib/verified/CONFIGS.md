# Configuration surface

Derived from the public header, dynamic symbols, and every input-dependent
branch in `../c_src/src/lib.c`. There are no Cargo features and no runtime
options or enum parameters. `pinflate` is the only public callable entry
point; all lower-level decoder functions are `static` and can only be driven
through it.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | Seven exported C objects | Initial bytes and sizes of `cp_error_reason`, `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, and `cp_dist_base`. | [x] |
| 2 | `pinflate` / stored decoder | Input pointer aligned to 4 bytes; final stored block; empty, one-byte, and many-byte payloads; exact output capacity. | [x] |
| 3 | `pinflate` / stored decoder | Input pointer offset 1 from 4-byte alignment; final stored block; empty, one-byte, and many-byte payloads; exact output capacity. | [x] |
| 4 | `pinflate` / stored decoder | Input pointer offset 2 from 4-byte alignment; final stored block; empty, one-byte, and many-byte payloads; exact output capacity. | [x] |
| 5 | `pinflate` / stored decoder | Input pointer offset 3 from 4-byte alignment; final stored block; empty, one-byte, and many-byte payloads; output capacity larger than the payload. | [x] |
| 6 | `pinflate` / fixed decoder | Input pointer aligned to 4 bytes; final fixed block; empty, one-literal, and many-literal payloads; exact output capacity. | [x] |
| 7 | `pinflate` / fixed decoder | Input pointer offset 1; final fixed block; randomized literal values and counts; output capacity larger than the payload. | [x] |
| 8 | `pinflate` / fixed decoder | Input pointer offset 2; final fixed block; randomized literal values and counts; stream ends in a partial final input word. | [x] |
| 9 | `pinflate` / fixed decoder | Input pointer offset 3; final fixed block; randomized literal values and counts; stream includes one or more full input words. | [x] |
| 10 | `pinflate` / fixed decoder | Fixed match with backwards distance 1 (`memset` branch), covering base length 3, length 258, and symbols with nonzero extra length bits. | [x] |
| 11 | `pinflate` / fixed decoder | Fixed match with backwards distance greater than 1 (byte-copy loop), covering base and extra-bit distance symbols and overlapping copies. | [x] |
| 12 | `pinflate` / fixed decoder | Two or more fixed blocks, with `BFINAL == 0` followed by `BFINAL == 1`; empty and nonempty component blocks. | [x] |
| 13 | `pinflate` / dynamic decoder | Input pointer aligned to 4 bytes; final dynamic block whose code-length stream uses direct lengths; randomized literals. | [x] |
| 14 | `pinflate` / dynamic decoder | Input pointer offset 1; final dynamic block whose code-length stream uses direct lengths; randomized literals. | [x] |
| 15 | `pinflate` / dynamic decoder | Input pointer offset 2; final dynamic block whose code-length stream uses direct lengths; randomized literals. | [x] |
| 16 | `pinflate` / dynamic decoder | Input pointer offset 3; final dynamic block whose code-length stream uses direct lengths; randomized literals. | [x] |
| 17 | `pinflate` / dynamic decoder | Input pointer aligned to 4 bytes; dynamic code-length symbols 16, 17, and 18 all used; randomized literals from the resulting tree. | [x] |
| 18 | `pinflate` / dynamic decoder | Input pointer offset 1; dynamic code-length symbols 16, 17, and 18 all used; exact output capacity. | [x] |
| 19 | `pinflate` / dynamic decoder | Input pointer offset 2; dynamic code-length symbols 16, 17, and 18 all used; larger output capacity. | [x] |
| 20 | `pinflate` / dynamic decoder | Input pointer offset 3; dynamic code-length symbols 16, 17, and 18 all used; both full-word and final-partial-word refill paths. | [x] |
| 21 | `pinflate` / dynamic decoder | Dynamic literal followed by distance-1 matches; base and nonzero-extra-bit length symbols. | [x] |
| 22 | `pinflate` / dynamic decoder | Dynamic matches with distance greater than 1; base and nonzero-extra-bit distance symbols with overlapping copies. | [x] |
| 23 | `pinflate` | Mixed nonfinal/final Huffman blocks (fixed then dynamic and dynamic then fixed), exercising state reuse across block types. | [x] |
| 24 | `pinflate` | Valid streams whose byte length and input alignment make `last_bytes` each of 0, 1, 2, and 3, covering full-word refill and `final_word_available`. | [x] |

The stored decoder's `s->bits_left / 8 <= LEN` check rejects a nonfinal
stored block when bytes for a following block remain. That C behavior belongs
to error row 2 and is not treated as a valid multi-stored configuration.
