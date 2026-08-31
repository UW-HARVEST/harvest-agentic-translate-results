| | `png_get_uint_31` | 4-byte big-endian value with the top bit set, i.e. `uval > PNG_UINT_31_MAX` (pngrutil.c:45) | `png_error(png_ptr, "PNG unsigned integer out of range")` — longjmp, read aborted |
| | `png_get_int_32` | two's-complement value `0x80000000` (negation overflows: `(uval & 0x80000000) != 0` after negate, pngrutil.c:87-93) | silently returns `0` (data known invalid) |
| | `png_read_sig` | first 8 bytes are not the PNG signature and the mismatch is in the first 4 bytes (`num_checked < 4 && png_sig_cmp(...) != 0`, pngrutil.c:137-139) | `png_error(png_ptr, "Not a PNG file")` |
| | `png_read_sig` | signature mismatch confined to the CR/LF/^Z/LF trailer bytes (pngrutil.c:140-141) | `png_error(png_ptr, "PNG file corrupted by ASCII conversion")` |
| | `check_chunk_name` | any of the 4 chunk-type bytes is not in `A-Z`/`a-z` (bit-whack test `(t & 0xe0e0e0e0U) == 0U` fails, pngrutil.c:152-177) | returns `0` (invalid name) — caller errors |
| | `png_read_chunk_header` | chunk length field with high bit set in the first byte: `buf[0] >= 0x80U` (pngrutil.c:210-211) | `png_chunk_error(png_ptr, "bad header (invalid length)")` |
| | `png_read_chunk_header` | chunk type containing non-alphabetic bytes: `!check_chunk_name(chunk_name)` (pngrutil.c:214-215) | `png_chunk_error(png_ptr, "bad header (invalid type)")` |
| | `png_crc_read` | `png_ptr == NULL` (pngrutil.c:228-229) | returns immediately, no read performed |
| | `png_crc_error` | stored chunk CRC differs from the computed CRC: `crc != png_ptr->crc` (pngrutil.c:293-294) | returns non-zero (CRC error) to `png_crc_finish_critical` |
| | `png_crc_error` | ancillary chunk with `PNG_FLAG_CRC_ANCILLARY_USE\|NOWARN`, or critical chunk with `PNG_FLAG_CRC_CRITICAL_IGNORE` (pngrutil.c:271-282, 297-298) | CRC not computed; returns `0` — corrupt data accepted by app request |
| | `png_crc_finish_critical` | CRC error on an ancillary chunk (or `handle_as_ancillary`) without `PNG_FLAG_CRC_ANCILLARY_NOWARN` (pngrutil.c:342-348) | `png_chunk_warning(png_ptr, "CRC error")`, returns `1` (chunk discarded) |
| | `png_crc_finish_critical` | CRC error on a critical chunk with default flags (`PNG_FLAG_CRC_CRITICAL_USE` not set) (pngrutil.c:350-351) | `png_chunk_error(png_ptr, "CRC error")` |
| | `png_read_buffer` | requested chunk buffer bigger than the configured limit: `new_size > png_chunk_max(png_ptr)` (pngrutil.c:380) | returns `NULL`; callers emit `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_read_buffer` | `png_malloc_base` returns `NULL` (out of memory) (pngrutil.c:392-404) | returns `NULL` |
| | `png_inflate_claim` | zstream already owned by another chunk: `png_ptr->zowner != 0`, release build (pngrutil.c:416-428) | `png_chunk_warning(png_ptr, "<cHNK> using zstream")`, ownership stolen |
| | `png_inflate_claim` | `png_ptr->zowner != 0`, non-release build (pngrutil.c:429-431) | `png_chunk_error(png_ptr, "<cHNK> using zstream")` |
| | `png_inflate_claim` | `inflateInit2`/`inflateReset2` fails (e.g. `Z_MEM_ERROR`) (pngrutil.c:476-509) | `png_zstream_error`, returns the zlib error code (not `Z_OK`) |
| | `png_zlib_inflate` | first deflate header byte encodes a window size > 32K: `(*next_in >> 4) > 7` (pngrutil.c:527-534) | sets `zstream.msg = "invalid window size (libpng)"`, returns `Z_DATA_ERROR` |
| | `png_inflate` | called while the stream is owned by a different chunk: `png_ptr->zowner != owner` (pngrutil.c:560, 662-670) | `zstream.msg = "zstream unclaimed"`, returns `Z_STREAM_ERROR` |
| | `png_inflate` | `inflate()` returns any error (corrupt/truncated LZ data) (pngrutil.c:636-638, 658-659) | `png_zstream_error`, returns the zlib code (caller rejects the chunk) |
| | `png_decompress_chunk` | chunk prefix alone already exceeds the memory limit: `limit < prefix_size + (terminate != 0)` (pngrutil.c:695, 821-826) | `png_zstream_error(png_ptr, Z_MEM_ERROR)`, returns `Z_MEM_ERROR` |
| | `png_decompress_chunk` | `png_inflate_claim` fails (`ret != Z_OK`) (pngrutil.c:705-707, 815-818) | returns the zlib error code; `Z_STREAM_END` is mapped to `PNG_UNEXPECTED_ZLIB_RETURN` |
| | `png_decompress_chunk` | first (sizing) `png_inflate` returns `Z_OK` instead of `Z_STREAM_END` (truncated LZ stream) (pngrutil.c:808-809) | `ret = PNG_UNEXPECTED_ZLIB_RETURN` |
| | `png_decompress_chunk` | `inflateReset` fails after the sizing pass (pngrutil.c:724, 800-805) | `png_zstream_error`, `ret = PNG_UNEXPECTED_ZLIB_RETURN` |
| | `png_decompress_chunk` | `png_malloc_base(buffer_size)` for the decompressed text fails (pngrutil.c:737, 792-797) | `ret = Z_MEM_ERROR`, `png_zstream_error(png_ptr, Z_MEM_ERROR)` |
| | `png_decompress_chunk` | second inflate pass produces a different length: `new_size != *newlength` (pngrutil.c:747, 764-773) | `ret = PNG_UNEXPECTED_ZLIB_RETURN` ("unexpected end of LZ stream") |
| | `png_decompress_chunk` | second inflate pass returns `Z_OK` (output buffer not consumed) (pngrutil.c:776-777) | `ret = PNG_UNEXPECTED_ZLIB_RETURN` |
| | `png_decompress_chunk` | compressed bytes left over after `Z_STREAM_END`: `chunklength - prefix_size != lzsize` (pngrutil.c:787-789) | `png_chunk_benign_error(png_ptr, "extra compressed data")` |
| | `png_inflate_read` | stream not owned by the current chunk: `png_ptr->zowner != png_ptr->chunk_name` (pngrutil.c:840, 890-894) | `zstream.msg = "zstream unclaimed"`, returns `Z_STREAM_ERROR` |
| | `png_handle_IHDR` | width or height field with the top bit set (via `png_get_uint_31`, pngrutil.c:917-918) | `png_error(png_ptr, "PNG unsigned integer out of range")` |
| | `png_handle_IHDR` | invalid bit depth / colour type / compression / filter / interlace combination — delegated to `png_set_IHDR` (pngrutil.c:965-969; the `default:` colour-type case at pngrutil.c:939 just guesses 1 channel) | `png_error` raised inside `png_set_IHDR` |
| | `png_handle_PLTE` | a second PLTE: `(png_ptr->mode & PNG_HAVE_PLTE) != 0` (pngrutil.c:992-993) | `errmsg = "duplicate"` → error path below |
| | `png_handle_PLTE` | PLTE after IDAT: `(png_ptr->mode & PNG_HAVE_IDAT) != 0` (pngrutil.c:995-996) | `errmsg = "out of place"` |
| | `png_handle_PLTE` | PLTE in a greyscale image: `(png_ptr->color_type & PNG_COLOR_MASK_COLOR) == 0` (pngrutil.c:998-999) | `errmsg = "ignored in grayscale PNG"` |
| | `png_handle_PLTE` | `length > 3*PNG_MAX_PALETTE_LENGTH` or `(length % 3) != 0` (pngrutil.c:1001-1002) | `errmsg = "invalid"` |
| | `png_handle_PLTE` | PLTE in a truecolour image after tRNS or bKGD was seen (pngrutil.c:1015-1017) | `errmsg = "out of place"` (PLTE dropped in favour of tRNS/bKGD) |
| | `png_handle_PLTE` | any of the above with `color_type == PNG_COLOR_TYPE_PALETTE` (PLTE critical) (pngrutil.c:1061-1064) | `png_crc_finish` then `png_chunk_error(png_ptr, errmsg)` |
| | `png_handle_PLTE` | any of the above for a non-colour-mapped image (pngrutil.c:1067-1076) | `png_chunk_benign_error(png_ptr, errmsg)`, returns `handled_error` |
| | `png_handle_PLTE` | palette larger than the bit depth allows but `<= 256` entries: `length > 3U*max_palette_length` (pngrutil.c:1026-1034) | no error; extra entries silently truncated to `1U << bit_depth` |
| | `png_handle_IEND` | IEND with data: `length != 0` (pngrutil.c:1091-1092) | `png_chunk_benign_error(png_ptr, "invalid")` (still returns `handled_ok`) |
| | `png_handle_gAMA` | CRC failure on gAMA: `png_crc_finish(png_ptr, 0) != 0` (pngrutil.c:1111-1112) | returns `handled_error`, chunk discarded |
| | `png_handle_gAMA` | gamma value with top bit set: `ugamma > PNG_UINT_31_MAX` (pngrutil.c:1116-1120) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` |
| | `png_handle_sBIT` | `length != truelen` (3 for palette, else `png_ptr->channels`) (pngrutil.c:1161-1166) | `png_crc_finish(length)` + `png_chunk_benign_error(png_ptr, "bad length")` |
| | `png_handle_sBIT` | CRC failure (pngrutil.c:1171-1172) | returns `handled_error` |
| | `png_handle_sBIT` | any significant-bit byte zero or too large: `buf[i] == 0 \|\| buf[i] > sample_depth` (pngrutil.c:1174-1181) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` |
| | `png_get_int_32_checked` | cHRM value `0x80000000` (two's-complement negation overflows) (pngrutil.c:1216-1223) | sets `*error = 1` and returns `0` |
| | `png_handle_cHRM` | CRC failure (pngrutil.c:1237-1238) | returns `handled_error` |
| | `png_handle_cHRM` | any of the 8 chromaticity values is the un-negatable `0x80000000`: `error != 0` (pngrutil.c:1249-1253) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` |
| | `png_handle_sRGB` | CRC failure (pngrutil.c:1290-1291) | returns `handled_error` |
| | `png_handle_sRGB` | rendering intent outside the PNGv3 range: `intent > 3` (pngrutil.c:1298-1302) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` |
| | `png_handle_iCCP` | after reading up to 81 keyword bytes, too little data left for a zlib stream: `length < LZ77Min` (= 11) (pngrutil.c:1353-1358) | `png_crc_finish` + `png_chunk_benign_error(png_ptr, "too short")` |
| | `png_handle_iCCP` | keyword empty or 80+ bytes: `!(keyword_length >= 1 && keyword_length <= 79)` (pngrutil.c:1366, 1532-1533) | `errmsg = "bad keyword"` → `png_chunk_benign_error` |
| | `png_handle_iCCP` | compression-method byte missing or not 0: `keyword[keyword_length+1] != PNG_COMPRESSION_TYPE_BASE` (pngrutil.c:1371-1372, 1528-1529) | `errmsg = "bad compression method"` |
| | `png_handle_iCCP` | `png_inflate_claim(png_iCCP)` fails (pngrutil.c:1376, 1524-1525) | `errmsg = png_ptr->zstream.msg` → benign error |
| | `png_handle_iCCP` | compressed stream too short to yield the 132-byte ICC header: `size != 0` after the first `png_inflate_read` (pngrutil.c:1388, 1517-1518) | `errmsg = png_ptr->zstream.msg` ("profile truncated") |
| | `png_handle_iCCP` | `png_icc_check_length` rejects `profile_length` (pngrutil.c:1394-1395, 1514) | chunk rejected; error message already emitted by the ICC checker |
| | `png_handle_iCCP` | `png_icc_check_header` rejects the 132-byte header (pngrutil.c:1400-1401, 1511) | chunk rejected; error already emitted |
| | `png_handle_iCCP` | `png_read_buffer(profile_length)` returns `NULL` (over limit / OOM) (pngrutil.c:1410-1413, 1507-1508) | `errmsg = "out of memory"` |
| | `png_handle_iCCP` | tag table truncated: `size != 0` after inflating `12 * tag_count` bytes (pngrutil.c:1427, 1503-1504) | `errmsg = png_ptr->zstream.msg` |
| | `png_handle_iCCP` | `png_icc_check_tag_table` rejects the tag table (pngrutil.c:1429-1430, 1501) | chunk rejected; error already emitted |
| | `png_handle_iCCP` | uncompressed chunk data left over and benign errors are errors: `length > 0 && !(flags & PNG_FLAG_BENIGN_ERRORS_WARN)` (pngrutil.c:1443-1445) | `errmsg = "extra compressed data"` → `png_chunk_benign_error` |
| | `png_handle_iCCP` | leftover data but benign errors warn: `length > 0` at pngrutil.c:1450-1456 | `png_chunk_warning(png_ptr, "extra compressed data")`, profile still accepted |
| | `png_handle_iCCP` | profile body shorter than `profile_length`: `size != 0` after the final `png_inflate_read` (pngrutil.c:1448, 1498-1499) | `errmsg = png_ptr->zstream.msg` |
| | `png_handle_iCCP` | `png_malloc_base` for `info_ptr->iccp_name` fails (pngrutil.c:1468-1484) | `errmsg = "out of memory"`, `handled_error` |
| | `png_handle_sPLT` | chunk cache exhausted: `png_ptr->user_chunk_cache_max == 1` (pngrutil.c:1569-1573) | chunk skipped silently, returns `handled_error` |
| | `png_handle_sPLT` | last cache slot consumed: `--png_ptr->user_chunk_cache_max == 1` (pngrutil.c:1575-1580) | `png_warning(png_ptr, "No space in chunk cache for sPLT")`, `handled_error` |
| | `png_handle_sPLT` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:1584-1590) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_sPLT` | CRC failure (pngrutil.c:1599-1600) | returns `handled_error` |
| | `png_handle_sPLT` | no sample depth after the name: `length < 2U \|\| entry_start > buffer + (length - 2U)` (pngrutil.c:1610-1614) | `png_warning(png_ptr, "malformed sPLT chunk")`, `handled_error` |
| | `png_handle_sPLT` | entry data not a whole number of entries: `(data_length % entry_size) != 0` (entry_size 6 for depth 8, else 10) (pngrutil.c:1624-1628) | `png_warning(png_ptr, "sPLT chunk has bad length")`, `handled_error` |
| | `png_handle_sPLT` | entry count overflows the allocation: `dl > PNG_SIZE_MAX / sizeof(png_sPLT_entry)` (pngrutil.c:1630-1637) | `png_warning(png_ptr, "sPLT chunk too long")`, `handled_error` |
| | `png_handle_sPLT` | `png_malloc_warn` for the entries fails (pngrutil.c:1641-1648) | `png_warning(png_ptr, "sPLT chunk requires too much memory")`, `handled_error` |
| | `png_handle_tRNS` | greyscale image and `length != 2` (pngrutil.c:1697-1702) | `png_crc_finish(length)` + `png_chunk_benign_error(png_ptr, "invalid")` |
| | `png_handle_tRNS` | truecolour image and `length != 6` (pngrutil.c:1713-1718) | `png_chunk_benign_error(png_ptr, "invalid")` |
| | `png_handle_tRNS` | palette image with no preceding PLTE: `(png_ptr->mode & PNG_HAVE_PLTE) == 0` (pngrutil.c:1729-1734) | `png_chunk_benign_error(png_ptr, "out of place")` |
| | `png_handle_tRNS` | palette image with `length > num_palette`, `length > PNG_MAX_PALETTE_LENGTH`, or `length == 0` (pngrutil.c:1736-1743) | `png_chunk_benign_error(png_ptr, "invalid")` |
| | `png_handle_tRNS` | colour type already has an alpha channel (GA / RGBA) (pngrutil.c:1749-1754) | `png_chunk_benign_error(png_ptr, "invalid with alpha channel")` |
| | `png_handle_tRNS` | CRC failure (pngrutil.c:1756-1760) | `png_ptr->num_trans = 0`, returns `handled_error` |
| | `png_handle_bKGD` | palette image with no preceding PLTE (pngrutil.c:1782-1787) | `png_chunk_benign_error(png_ptr, "out of place")` |
| | `png_handle_bKGD` | `length != truelen` (1 palette / 6 colour / 2 grey) (pngrutil.c:1798-1803) | `png_chunk_benign_error(png_ptr, "invalid")` |
| | `png_handle_bKGD` | CRC failure (pngrutil.c:1807-1808) | returns `handled_error` |
| | `png_handle_bKGD` | palette index out of range: `buf[0] >= info_ptr->num_palette` (pngrutil.c:1819-1825) | `png_chunk_benign_error(png_ptr, "invalid index")` |
| | `png_handle_bKGD` | greyscale, `bit_depth <= 8`, and `buf[0] != 0 \|\| buf[1] >= (1 << bit_depth)` (pngrutil.c:1840-1846) | `png_chunk_benign_error(png_ptr, "invalid gray level")` |
| | `png_handle_bKGD` | colour, `bit_depth <= 8`, and any high byte non-zero: `buf[0] != 0 \|\| buf[2] != 0 \|\| buf[4] != 0` (pngrutil.c:1858-1864) | `png_chunk_benign_error(png_ptr, "invalid color")` |
| | `png_handle_cICP` | CRC failure (pngrutil.c:1891-1892) | returns `handled_error` |
| | `png_handle_cLLI` | CRC failure (pngrutil.c:1930-1931) | returns `handled_error` |
| | `png_handle_cLLI` | out-of-range maxCLL/maxFALL — checking delegated to `png_set_cLLI_fixed` (pngrutil.c:1934-1935) | error/warning raised inside `png_set_cLLI_fixed` |
| | `png_handle_mDCV` | CRC failure (pngrutil.c:1954-1955) | returns `handled_error` |
| | `png_handle_mDCV` | out-of-range chromaticities/luminances — delegated to `png_set_mDCV_fixed` (pngrutil.c:1977-1983) | error/warning raised inside `png_set_mDCV_fixed` |
| | `png_handle_eXIf` | `png_read_buffer(length)` returns `NULL` (pngrutil.c:2005-2012) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_eXIf` | CRC failure (pngrutil.c:2016-2017) | returns `handled_error` |
| | `png_handle_eXIf` | first 4 bytes are neither `0x49492A00` (II) nor `0x4D4D002A` (MM) (pngrutil.c:2024-2031) | `png_chunk_benign_error(png_ptr, "invalid")`, `handled_error` |
| | `png_handle_hIST` | `length != num * 2`, `num != png_ptr->num_palette`, or `num > PNG_MAX_PALETTE_LENGTH` (pngrutil.c:2056-2065) | `png_crc_finish(length)` + `png_chunk_benign_error(png_ptr, "invalid")` |
| | `png_handle_hIST` | CRC failure (pngrutil.c:2075-2076) | returns `handled_error` |
| | `png_handle_pHYs` | CRC failure (pngrutil.c:2097-2098) | returns `handled_error` |
| | `png_handle_oFFs` | CRC failure (pngrutil.c:2123-2124) | returns `handled_error` |
| | `png_handle_pCAL` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2156-2163) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_pCAL` | CRC failure (pngrutil.c:2167-2168) | returns `handled_error` |
| | `png_handle_pCAL` | fewer than 13 bytes after the purpose string: `endptr - buf <= 12` (pngrutil.c:2181-2185) | `png_chunk_benign_error(png_ptr, "invalid")` |
| | `png_handle_pCAL` | parameter count wrong for the equation type (`LINEAR!=2`, `BASE_E!=3`, `ARBITRARY!=3`, `HYPERBOLIC!=4`) (pngrutil.c:2198-2205) | `png_chunk_benign_error(png_ptr, "invalid parameter count")` |
| | `png_handle_pCAL` | `type >= PNG_EQUATION_LAST` (pngrutil.c:2207-2210) | `png_chunk_benign_error(png_ptr, "unrecognized equation type")` (processing continues) |
| | `png_handle_pCAL` | `png_malloc_warn` for the `nparams` pointer array fails (pngrutil.c:2217-2224) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_pCAL` | a parameter string runs past the end of the chunk: `buf > endptr` (pngrutil.c:2233-2242) | `png_free(params)` + `png_chunk_benign_error(png_ptr, "invalid data")` |
| | `png_handle_sCAL` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2274-2281) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_sCAL` | CRC failure (pngrutil.c:2286-2287) | returns `handled_error` |
| | `png_handle_sCAL` | unit byte neither 1 nor 2: `buffer[0] != 1 && buffer[0] != 2` (pngrutil.c:2290-2294) | `png_chunk_benign_error(png_ptr, "invalid unit")` |
| | `png_handle_sCAL` | width is not a valid ASCII float or is not NUL-terminated inside the chunk: `png_check_fp_number(...) == 0 \|\| i >= length \|\| buffer[i++] != 0` (pngrutil.c:2302-2304) | `png_chunk_benign_error(png_ptr, "bad width format")`, `handled_error` |
| | `png_handle_sCAL` | width parses but is zero/negative: `PNG_FP_IS_POSITIVE(state) == 0` (pngrutil.c:2306-2307) | `png_chunk_benign_error(png_ptr, "non-positive width")` |
| | `png_handle_sCAL` | height is not a valid ASCII float or does not end exactly at the chunk end: `png_check_fp_number(...) == 0 \|\| i != length` (pngrutil.c:2314-2316) | `png_chunk_benign_error(png_ptr, "bad height format")` |
| | `png_handle_sCAL` | height parses but is zero/negative (pngrutil.c:2318-2319) | `png_chunk_benign_error(png_ptr, "non-positive height")` |
| | `png_handle_tIME` | CRC failure (pngrutil.c:2354-2355) | returns `handled_error` |
| | `png_handle_tIME` | out-of-range month/day/hour/minute/second — checking delegated to `png_set_tIME` (pngrutil.c:2364) | warning/error raised inside `png_set_tIME` |
| | `png_handle_tEXt` | chunk cache exhausted: `user_chunk_cache_max == 1` (pngrutil.c:2388-2392) | chunk skipped silently, `handled_error` |
| | `png_handle_tEXt` | last cache slot consumed: `--user_chunk_cache_max == 1` (pngrutil.c:2394-2399) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")` |
| | `png_handle_tEXt` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2403-2410) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_tEXt` | CRC failure (pngrutil.c:2414-2415) | returns `handled_error` |
| | `png_handle_tEXt` | `png_set_text_2` fails (allocation failure storing the text) (pngrutil.c:2434-2438) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_zTXt` | chunk cache exhausted: `user_chunk_cache_max == 1` (pngrutil.c:2458-2462) | chunk skipped silently, `handled_error` |
| | `png_handle_zTXt` | last cache slot consumed (pngrutil.c:2464-2469) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")` |
| | `png_handle_zTXt` | `png_read_buffer(length)` returns `NULL` (pngrutil.c:2477-2484) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_zTXt` | CRC failure (pngrutil.c:2488-2489) | returns `handled_error` |
| | `png_handle_zTXt` | keyword empty or too long: `keyword_length > 79 \|\| keyword_length < 1` (pngrutil.c:2497-2498) | `errmsg = "bad keyword"` → `png_chunk_benign_error` |
| | `png_handle_zTXt` | no room for separator + method + LZ data: `keyword_length + 3 > length` (pngrutil.c:2504-2505) | `errmsg = "truncated"` |
| | `png_handle_zTXt` | `buffer[keyword_length+1] != PNG_COMPRESSION_TYPE_BASE` (pngrutil.c:2507-2508) | `errmsg = "unknown compression type"` |
| | `png_handle_zTXt` | `png_decompress_chunk` does not return `Z_STREAM_END` (pngrutil.c:2518-2519, 2549-2550) | `errmsg = png_ptr->zstream.msg` → `png_chunk_benign_error` |
| | `png_handle_zTXt` | `png_ptr->read_buffer == NULL` after a "successful" decompress (pngrutil.c:2523-2524) | `errmsg = "Read failure in png_handle_zTXt"` |
| | `png_handle_zTXt` | `png_set_text_2` fails (pngrutil.c:2542-2545) | `errmsg = "out of memory"`, `handled_error` |
| | `png_handle_iTXt` | chunk cache exhausted: `user_chunk_cache_max == 1` (pngrutil.c:2574-2578) | chunk skipped silently, `handled_error` |
| | `png_handle_iTXt` | last cache slot consumed (pngrutil.c:2580-2585) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")` |
| | `png_handle_iTXt` | `png_read_buffer(length+1)` returns `NULL` (pngrutil.c:2589-2596) | `png_chunk_benign_error(png_ptr, "out of memory")` |
| | `png_handle_iTXt` | CRC failure (pngrutil.c:2600-2601) | returns `handled_error` |
| | `png_handle_iTXt` | keyword empty or too long: `prefix_length > 79 \|\| prefix_length < 1` (pngrutil.c:2610-2611) | `errmsg = "bad keyword"` |
| | `png_handle_iTXt` | too short for keyword + flag + method + 2 NULs: `prefix_length + 5 > length` (pngrutil.c:2617-2618) | `errmsg = "truncated"` |
| | `png_handle_iTXt` | compression flag not 0, or flag 1 with method byte != 0 (pngrutil.c:2620-2622, 2698-2699) | `errmsg = "bad compression info"` → `png_chunk_benign_error` |
| | `png_handle_iTXt` | compressed iTXt whose prefix consumes the whole chunk: `compressed != 0 && prefix_length >= length` (pngrutil.c:2650-2670) | `errmsg = "truncated"` |
| | `png_handle_iTXt` | `png_decompress_chunk` does not return `Z_STREAM_END` (pngrutil.c:2661-2666) | `errmsg = png_ptr->zstream.msg` |
| | `png_handle_iTXt` | `png_set_text_2` fails (pngrutil.c:2691-2694) | `errmsg = "out of memory"`, `handled_error` |
| | `png_cache_unknown_chunk` | unknown chunk over the memory limit (`length > png_chunk_max`) or `png_malloc_warn` fails (pngrutil.c:2722, 2741-2747) | `png_crc_finish` + `png_chunk_benign_error(png_ptr, "unknown chunk exceeds memory limits")`, returns `0` |
| | `png_handle_unknown` | user read callback returns a negative value (pngrutil.c:2811-2812) | `png_chunk_error(png_ptr, "error in user chunk")` |
| | `png_handle_unknown` | callback returns 0 and neither per-chunk nor default keep is `>= PNG_HANDLE_CHUNK_IF_SAFE` (pngrutil.c:2827-2839) | `png_chunk_warning(png_ptr, "Saving unknown chunk:")` + `png_app_warning(png_ptr, "forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks")` |
| | `png_handle_unknown` | app asked to keep unknown chunks in a build without any save/store support: `keep > PNG_HANDLE_CHUNK_NEVER` (pngrutil.c:2892-2893) | `png_app_error(png_ptr, "no unknown chunk support available")` |
| | `png_handle_unknown` | chunk cache limit reached while storing: `png_ptr->user_chunk_cache_max == 2` (pngrutil.c:2908-2912) | `png_chunk_benign_error(png_ptr, "no space in chunk cache")`, chunk not stored |
| | `png_handle_unknown` | an unknown/disabled **critical** chunk was neither handled nor saved: `handled < handled_saved && PNG_CHUNK_CRITICAL(chunk_name)` (pngrutil.c:2956-2957) | `png_chunk_error(png_ptr, "unhandled critical chunk")` |
| | `png_handle_chunk` | any known chunk other than IHDR arriving before IHDR: `(png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngrutil.c:3133-3135) | `png_chunk_error(png_ptr, "missing IHDR")` — NORETURN |
| | `png_handle_chunk` | chunk after a `pos_before` marker or before a required `pos_after` marker (table `read_chunks[]`, pngrutil.c:3138-3143) | `errmsg = "out of place"` |
| | `png_handle_chunk` | second occurrence of a single-instance chunk: `multiple == 0 && png_file_has_chunk(...)` (pngrutil.c:3148-3152) | `errmsg = "duplicate"` |
| | `png_handle_chunk` | `length < read_chunks[chunk_index].min_length` (e.g. cHRM<32, gAMA<4, eXIf<4, iCCP<14, iTXt<6, tEXt<2, pCAL<14) (pngrutil.c:3154-3155) | `errmsg = "too short"` |
| | `png_handle_chunk` | `Limit` chunk (eXIf/zTXt/sCAL/fdAT) longer than the memory limit: `length > png_chunk_max(png_ptr)` (pngrutil.c:3169-3178) | `errmsg = "length exceeds libpng limit"` |
| | `png_handle_chunk` | fixed-max chunk longer than its spec maximum: `length > max_length` (e.g. IHDR>13, tRNS>256, hIST>1024, pHYs>9, tIME>7) (pngrutil.c:3180-3185) | `errmsg = "too long"` |
| | `png_handle_chunk` | any of the above `errmsg` cases on a **critical** chunk (pngrutil.c:3198-3201) | `png_chunk_error(png_ptr, errmsg)` — read aborted |
| | `png_handle_chunk` | any of the above `errmsg` cases on an ancillary chunk (pngrutil.c:3202-3207) | `png_crc_finish(length)` (data skipped) + `png_chunk_benign_error(png_ptr, errmsg)` |
| | `png_combine_row` | called before any row was transformed: `pixel_depth == 0` (pngrutil.c:3242-3243) | `png_error(png_ptr, "internal row logic error")` |
| | `png_combine_row` | app row size disagrees with libpng: `info_rowbytes != PNG_ROWBYTES(pixel_depth, row_width)` (pngrutil.c:3249-3251) | `png_error(png_ptr, "internal row size calculation error")` |
| | `png_combine_row` | `row_width == 0` (pngrutil.c:3254-3255) | `png_error(png_ptr, "internal row width error")` |
| | `png_combine_row` | interlace pass has no pixels in this row: `row_width <= PNG_PASS_START_COL(pass)` (pngrutil.c:3294-3295) | returns without copying |
| | `png_combine_row` | user transform produced a depth >= 8 that is not a whole number of bytes: `pixel_depth & 7` (pngrutil.c:3477-3478) | `png_error(png_ptr, "invalid user transform pixel depth")` |
| | `png_do_read_interlace` | `row == NULL \|\| row_info == NULL` (pngrutil.c:3715) | no-op, row left unchanged |
| | `png_read_filter_row` | filter byte 0 (NONE) or out of range: `!(filter > PNG_FILTER_VALUE_NONE && filter < PNG_FILTER_VALUE_LAST)` (pngrutil.c:4161-4167) | no un-filtering performed (invalid filter byte silently ignored) |
| | `png_read_IDAT_data` | the chunk following an exhausted IDAT is not another IDAT: `png_ptr->chunk_name != png_IDAT` (pngrutil.c:4192-4201) | `png_error(png_ptr, "Not enough image data")` |
| | `png_read_IDAT_data` | `png_read_buffer(avail_in)` returns `NULL` (pngrutil.c:4219-4222) | `png_chunk_error(png_ptr, "out of memory")` |
| | `png_read_IDAT_data` | LZ stream ended but IDAT bytes remain: `zstream.avail_in > 0 \|\| png_ptr->idat_size > 0` (pngrutil.c:4275-4276) | `png_chunk_benign_error(png_ptr, "Extra compressed data")` |
| | `png_read_IDAT_data` | `inflate` returns an error while producing image rows (`output != NULL`) (pngrutil.c:4280-4285) | `png_zstream_error` + `png_chunk_error(png_ptr, png_ptr->zstream.msg)` |
| | `png_read_IDAT_data` | `inflate` returns an error during the end-of-stream check (`output == NULL`) (pngrutil.c:4287-4291) | `png_chunk_benign_error(png_ptr, png_ptr->zstream.msg)` and return |
| | `png_read_IDAT_data` | stream ended before the requested row data was produced: `avail_out > 0` with `output != NULL` (pngrutil.c:4295-4301) | `png_error(png_ptr, "Not enough image data")` |
| | `png_read_IDAT_data` | extra decompressed data past the end of the image: `avail_out > 0` with `output == NULL` (pngrutil.c:4303-4304) | `png_chunk_benign_error(png_ptr, "Too much image data")` |
| | `png_read_start_row` | (`PNG_MAX_MALLOC_64K` builds) computed buffer `row_bytes > 65536L` (pngrutil.c:4599-4600) | `png_error(png_ptr, "This image requires a row greater than 64KB")` |
| | `png_read_start_row` | (`PNG_MAX_MALLOC_64K` builds) `png_ptr->rowbytes > 65535` (pngrutil.c:4644-4645) | `png_error(png_ptr, "This image requires a row greater than 64KB")` |
| | `png_read_start_row` | `png_ptr->rowbytes > (PNG_SIZE_MAX - 1)` (pngrutil.c:4648-4649) | `png_error(png_ptr, "Row has too many bytes to allocate in memory")` |
| | `png_read_start_row` | `png_inflate_claim(png_ptr, png_IDAT) != Z_OK` (bad deflate header / OOM) (pngrutil.c:4679-4680) | `png_error(png_ptr, png_ptr->zstream.msg)` |
| | `icc_check_length` | ICC profile shorter than the fixed header+tag-count: `profile_length < 132` (png.c:1588-1589, reached from `png_handle_iCCP`) | `png_icc_profile_error` → `png_chunk_benign_error(... "profile '<name>': too short")`, returns 0 |
| | `png_icc_check_length` | `profile_length > png_chunk_max(png_ptr)` (png.c:1606-1608) | `png_icc_profile_error(... "profile too long")`, returns 0 |
| | `png_icc_check_header` | profile's own length word differs from the chunk-derived length: `png_get_uint_32(profile) != profile_length` (png.c:1625-1628) | `png_icc_profile_error(... "length does not match profile")`, returns 0 |
| | `png_icc_check_header` | major version > 3 and length not a multiple of 4: `temp > 3 && (profile_length & 3)` (png.c:1630-1633) | `png_icc_profile_error(... "invalid length")`, returns 0 |
| | `png_icc_check_header` | `tag_count > 357913930` or `profile_length < 132 + 12*tag_count` (truncated tag table) (png.c:1635-1639) | `png_icc_profile_error(... "tag count too large")`, returns 0 |
| | `png_icc_check_header` | rendering intent field `>= 0xffff` (png.c:1644-1647) | `png_icc_profile_error(... "invalid rendering intent")`, returns 0 |
| | `png_icc_check_header` | rendering intent `>= PNG_sRGB_INTENT_LAST` but `< 0xffff` (png.c:1652-1654) | warning only: `png_icc_profile_error(... "intent outside defined range")`, profile still accepted |
| | `png_icc_check_header` | ICC file signature at offset 36 not `'acsp'` (`0x61637370`) (png.c:1668-1671) | `png_icc_profile_error(... "invalid signature")`, returns 0 |
| | `png_icc_check_header` | PCS illuminant at offset 68 is not the D50 nCIEXYZ value (png.c:1680-1682) | warning only: `png_icc_profile_error(... "PCS illuminant is not D50")` |
| | `png_icc_check_header` | data colour space `'RGB '` on a greyscale PNG: `(color_type & PNG_COLOR_MASK_COLOR) == 0` (png.c:1707-1710) | `png_icc_profile_error(... "RGB color space not permitted on grayscale PNG")`, returns 0 |
| | `png_icc_check_header` | data colour space `'GRAY'` on a colour PNG (png.c:1713-1716) | `png_icc_profile_error(... "Gray color space not permitted on RGB PNG")`, returns 0 |
| | `png_icc_check_header` | data colour space neither `'RGB '` nor `'GRAY'` (png.c:1719-1721) | `png_icc_profile_error(... "invalid ICC profile color space")`, returns 0 |
| | `png_icc_check_header` | profile class `'abst'` (abstract) embedded in a PNG (png.c:1743-1746) | `png_icc_profile_error(... "invalid embedded Abstract ICC profile")`, returns 0 |
| | `png_icc_check_header` | profile class `'link'` (DeviceLink) (png.c:1748-1756) | `png_icc_profile_error(... "unexpected DeviceLink ICC profile class")`, returns 0 |
| | `png_icc_check_header` | profile class `'nmcl'` (NamedColor) (png.c:1758-1765) | warning only: `png_icc_profile_error(... "unexpected NamedColor ICC profile class")` |
| | `png_icc_check_header` | profile class not one of scnr/mntr/prtr/spac/abst/link/nmcl (png.c:1767-1775) | warning only: `png_icc_profile_error(... "unrecognized ICC profile class")` |
| | `png_icc_check_header` | PCS encoding at offset 20 neither `'XYZ '` nor `'Lab '` (png.c:1781-1791) | `png_icc_profile_error(... "unexpected ICC PCS encoding")`, returns 0 |
| | `png_icc_check_tag_table` | a tag lies outside the profile: `tag_start > profile_length \|\| tag_length > profile_length - tag_start` (png.c:1824-1826) | `png_icc_profile_error(... "ICC profile tag outside profile")`, returns 0 |
| | `png_icc_check_tag_table` | tag offset not 4-byte aligned: `(tag_start & 3) != 0` (png.c:1828-1836) | warning only: `png_icc_profile_error(... "ICC profile tag start not a multiple of 4")` |
| | `write_unknown_chunks` | an app-supplied unknown chunk with `up->size == 0` (pngwrite.c:63-64) | `png_warning(png_ptr, "Writing zero-length unknown chunk")`, chunk still written |
| | `png_write_info_before_PLTE` | `png_ptr == NULL \|\| info_ptr == NULL` (pngwrite.c:87-88) | returns, nothing written |
| | `png_write_info_before_PLTE` | MNG features enabled while writing a real PNG stream: `(mode & PNG_HAVE_PNG_SIGNATURE) != 0 && mng_features_permitted != 0` (pngwrite.c:96-102) | `png_warning(png_ptr, "MNG features are not allowed in a PNG datastream")`, features cleared |
| | `png_write_info` | `png_ptr == NULL \|\| info_ptr == NULL` (pngwrite.c:231-232) | returns, nothing written |
| | `png_write_info` | `color_type == PNG_COLOR_TYPE_PALETTE` but `(info_ptr->valid & PNG_INFO_PLTE) == 0` (pngwrite.c:236-241) | `png_error(png_ptr, "Valid palette required for paletted images")` |
| | `png_write_info` | iTXt text supplied in a build without `PNG_WRITE_iTXt_SUPPORTED` (pngwrite.c:330-347) | `png_warning(png_ptr, "Unable to write international text")`, chunk dropped |
| | `png_write_info` | zTXt text supplied in a build without `PNG_WRITE_zTXt_SUPPORTED` (pngwrite.c:351-361) | `png_warning(png_ptr, "Unable to write compressed text")`, chunk dropped |
| | `png_write_info` | tEXt text supplied in a build without `PNG_WRITE_tEXt_SUPPORTED` (pngwrite.c:364-376) | `png_warning(png_ptr, "Unable to write uncompressed text")`, chunk dropped |
| | `png_write_end` | `png_ptr == NULL` (pngwrite.c:396-397) | returns, nothing written |
| | `png_write_end` | no image data written: `(png_ptr->mode & PNG_HAVE_IDAT) == 0` (pngwrite.c:399-400) | `png_error(png_ptr, "No IDATs written into file")` |
| | `png_write_end` | palette image where a written index exceeded the palette: `num_palette_max >= png_ptr->num_palette` (pngwrite.c:403-405) | `png_benign_error(png_ptr, "Wrote palette index exceeding num_palette")` |
| | `png_write_end` | trailer iTXt/zTXt/tEXt text in a build where that chunk type is not compiled in (pngwrite.c:444, 457, 470) | `png_warning(png_ptr, "Unable to write international/compressed/uncompressed text")` |
| | `png_convert_from_time_t` | `gmtime(&ttime)` returns `NULL` (unrepresentable `time_t`) (pngwrite.c:527-536) | `memset(ptime, 0, ...)` and return — silently produces an all-zero time |
| | `png_write_rows` | `png_ptr == NULL` (pngwrite.c:635-636) | returns, no rows written |
| | `png_write_image` | `png_ptr == NULL` (pngwrite.c:655-656) | returns, no rows written |
| | `png_do_write_intrapixel` | MNG filter 64 requested for a colour type that is not RGB/RGBA at 8 or 16 bits (pngwrite.c:695-702, 717-724) | returns without transforming the row |
| | `png_write_row` | `png_ptr == NULL` (pngwrite.c:754-755) | returns |
| | `png_write_row` | first row written without a preceding `png_write_info`: `(png_ptr->mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0` (pngwrite.c:761-763) | `png_error(png_ptr, "png_write_info was never called before png_write_row")` |
| | `png_write_row` | a read-side-only transform was set in a build where the write side is compiled out (`PNG_INVERT_MONO`, `PNG_FILLER`, `PNG_PACKSWAP`, `PNG_PACK`, `PNG_SHIFT`, `PNG_BGR`, `PNG_SWAP_BYTES`) (pngwrite.c:766-800) | `png_warning(png_ptr, "PNG_WRITE_<X>_SUPPORTED is not defined")` — one warning per transform, transform ignored |
| | `png_write_row` | interlaced write and the current `row_number` is not in the current pass (per-pass tests for passes 0-6, including `width < 5`, `< 3`, `< 2`) (pngwrite.c:810-871) | `png_write_finish_row(png_ptr)` and return — row silently discarded |
| | `png_write_row` | interlacing left the row empty: `row_info.width == 0` after `png_do_write_interlace` (pngwrite.c:899-903) | `png_write_finish_row(png_ptr)` and return |
| | `png_write_row` | transformed depth disagrees with the header: `row_info.pixel_depth != png_ptr->pixel_depth \|\| row_info.pixel_depth != png_ptr->transformed_pixel_depth` (pngwrite.c:916-918) | `png_error(png_ptr, "internal write transform logic error")` |
| | `png_set_flush` | `png_ptr == NULL` (pngwrite.c:960-961); `nrows < 0` (pngwrite.c:963) | returns / negative interval clamped to `0` (flushing off) |
| | `png_write_flush` | `png_ptr == NULL` (pngwrite.c:972-973), or all rows already written: `row_number >= num_rows` (pngwrite.c:976-977) | returns without flushing |
| | `png_destroy_write_struct` | `png_ptr_ptr == NULL` or `*png_ptr_ptr == NULL` (pngwrite.c:1041-1045) | silently does nothing |
| | `png_set_filter` | `png_ptr == NULL` (pngwrite.c:1062-1063) | returns |
| | `png_set_filter` | filter value 5, 6 or 7 for method 0: `filters & (PNG_ALL_FILTERS \| 0x07)` in {5,6,7} (pngwrite.c:1073-1078) | `png_app_error(png_ptr, "Unknown row filter for method 0")`, falls through to `PNG_FILTER_NONE` |
| | `png_set_filter` | any non-`NONE` filter in a build without `PNG_WRITE_FILTER_SUPPORTED` (pngwrite.c:1099-1101) | `png_app_error(png_ptr, "Unknown row filter for method 0")` |
| | `png_set_filter` | UP/AVG/PAETH requested after writing started with no `prev_row`: `(filters & (UP\|AVG\|PAETH)) != 0 && png_ptr->prev_row == NULL` (pngwrite.c:1134-1143) | `png_app_warning(png_ptr, "png_set_filter: UP/AVG/PAETH cannot be added after start")`, those filters removed |
| | `png_set_filter` | `method != PNG_FILTER_TYPE_BASE` (pngwrite.c:1179-1180) | `png_error(png_ptr, "Unknown custom filter method")` |
| | `png_set_compression_level` | `png_ptr == NULL` (pngwrite.c:1220-1221) | returns |
| | `png_set_compression_mem_level` | `png_ptr == NULL` (pngwrite.c:1231-1232) | returns |
| | `png_set_compression_strategy` | `png_ptr == NULL` (pngwrite.c:1242-1243) | returns |
| | `png_set_compression_window_bits` | `png_ptr == NULL` (pngwrite.c:1259-1260) | returns |
| | `png_set_compression_window_bits` | `window_bits > 15` (pngwrite.c:1268-1272) | `png_warning(png_ptr, "Only compression windows <= 32k supported by PNG")`, clamped to 15 |
| | `png_set_compression_window_bits` | `window_bits < 8` (incl. negative raw-deflate values) (pngwrite.c:1274-1278) | `png_warning(png_ptr, "Only compression windows >= 256 supported by PNG")`, clamped to 8 |
| | `png_set_compression_method` | `png_ptr == NULL` (pngwrite.c:1288-1289) | returns |
| | `png_set_compression_method` | `method != 8` (pngwrite.c:1294-1295) | `png_warning(png_ptr, "Only compression method 8 is supported by PNG")` (value still stored; deflate will fail) |
| | `png_set_text_compression_level` | `png_ptr == NULL` (pngwrite.c:1308-1309) | returns |
| | `png_set_text_compression_mem_level` | `png_ptr == NULL` (pngwrite.c:1319-1320) | returns |
| | `png_set_text_compression_strategy` | `png_ptr == NULL` (pngwrite.c:1330-1331) | returns |
| | `png_set_text_compression_window_bits` | `png_ptr == NULL` (pngwrite.c:1344-1345) | returns |
| | `png_set_text_compression_window_bits` | `window_bits > 15` (pngwrite.c:1347-1351) | `png_warning(png_ptr, "Only compression windows <= 32k supported by PNG")`, clamped to 15 |
| | `png_set_text_compression_window_bits` | `window_bits < 8` (pngwrite.c:1353-1357) | `png_warning(png_ptr, "Only compression windows >= 256 supported by PNG")`, clamped to 8 |
| | `png_set_text_compression_method` | `png_ptr == NULL` (pngwrite.c:1367-1368) | returns |
| | `png_set_text_compression_method` | `method != 8` (pngwrite.c:1370-1371) | `png_warning(png_ptr, "Only compression method 8 is supported by PNG")` |
| | `png_set_write_status_fn` | `png_ptr == NULL` (pngwrite.c:1383-1384) | returns |
| | `png_set_write_user_transform_fn` | `png_ptr == NULL` (pngwrite.c:1396-1397) | returns |
| | `png_write_png` | `png_ptr == NULL \|\| info_ptr == NULL` (pngwrite.c:1412-1413) | returns |
| | `png_write_png` | no rows attached: `(info_ptr->valid & PNG_INFO_IDAT) == 0` (pngwrite.c:1415-1419) | `png_app_error(png_ptr, "no rows for png_write_image to write")` and return |
| | `png_write_png` | a `PNG_TRANSFORM_*` bit set in a build where that write transform is compiled out (INVERT_MONO, SHIFT, PACKING, SWAP_ALPHA, STRIP_FILLER, BGR, SWAP_ENDIAN, PACKSWAP, INVERT_ALPHA) (pngwrite.c:1427-1516) | `png_app_error(png_ptr, "PNG_TRANSFORM_<X> not supported")` — one per transform |
| | `png_write_png` | both `PNG_TRANSFORM_STRIP_FILLER_AFTER` and `..._BEFORE` requested (pngwrite.c:1469-1473) | `png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported")`, AFTER used if ignored |
| | `png_image_write_init` | `png_create_write_struct`, `png_create_info_struct` or the `png_control` malloc fails (pngwrite.c:1536-1567) | cleans up and `return png_image_error(image, "png_image_write_: out of memory")` (returns 0) |
| | `png_write_image_16bit` | called for a format without an alpha channel: `(image->format & PNG_FORMAT_FLAG_ALPHA) == 0` (pngwrite.c:1612-1629) | `png_error(png_ptr, "png_write_image: internal call error")` |
| | `png_image_write_main` | `image->width > 0x7fffffffU/channels` (row stride computation would overflow) (pngwrite.c:2024, 2052-2053) | `png_error(image->opaque->png_ptr, "image row stride too large")` |
| | `png_image_write_main` | supplied `row_stride` smaller in magnitude than one row: `check < png_row_stride` (pngwrite.c:2032-2049) | `png_error(image->opaque->png_ptr, "supplied row stride too small")` |
| | `png_image_write_main` | total buffer would exceed 32 bits: `image->height > 0xffffffffU/png_row_stride` (pngwrite.c:2044-2045) | `png_error(image->opaque->png_ptr, "memory image too large")` |
| | `png_image_write_main` | colour-mapped format but `display->colormap == NULL` or `image->colormap_entries == 0` (pngwrite.c:2057-2073) | `png_error(image->opaque->png_ptr, "no color-map for color-mapped image")` |
| | `png_image_write_main` | `image->format` contains flags other than COLOR/LINEAR/ALPHA/COLORMAP after the handled transforms are removed (pngwrite.c:2154-2156) | `png_error(png_ptr, "png_write_image: unsupported transformation")` |
| | `png_image_write_main` | `png_safe_execute(png_write_image_16bit/8bit)` returns 0 (error inside row conversion) (pngwrite.c:2198-2207) | returns 0 without calling `png_write_end` |
| | `png_image_set_PLTE` | `image->colormap_entries > 256` (pngwrite.c:1856-1857) | silently truncated to 256 entries and `image->colormap_entries` rewritten |
| | `image_memory_write` | output byte count would overflow: `size > ((png_alloc_size_t)-1) - ob` (pngwrite.c:2239, 2252-2253) | `png_error(png_ptr, "png_image_write_to_memory: PNG too big")` |
| | `image_memory_write` | supplied buffer too small: `display->memory_bytes < ob+size` (pngwrite.c:2244-2248) | data not copied; only `output_bytes` accumulated (caller detects overflow) |
| | `png_image_write_to_memory` | `memory_bytes == NULL \|\| buffer == NULL` (pngwrite.c:2286, 2331-2333) | `png_image_error(image, "png_image_write_to_memory: invalid argument")`, returns 0 |
| | `png_image_write_to_memory` | `image->version != PNG_IMAGE_VERSION` (pngwrite.c:2284, 2336-2338) | `png_image_error(image, "png_image_write_to_memory: incorrect PNG_IMAGE_VERSION")` |
| | `png_image_write_to_memory` | `image == NULL` (pngwrite.c:2340-2341) | returns 0 (no error message possible) |
| | `png_image_write_to_memory` | `png_image_write_init` failed (pngwrite.c:2327-2328) | returns 0 |
| | `png_image_write_to_memory` | encoded PNG bigger than the supplied buffer: `memory != NULL && display.output_bytes > *memory_bytes` (pngwrite.c:2318-2321) | returns 0 but `*memory_bytes` set to the required size |
| | `png_image_write_to_stdio` | `file == NULL \|\| buffer == NULL` (pngwrite.c:2352, 2381-2383) | `png_image_error(image, "png_image_write_to_stdio: invalid argument")`, returns 0 |
| | `png_image_write_to_stdio` | `image->version != PNG_IMAGE_VERSION` (pngwrite.c:2350, 2386-2388) | `png_image_error(image, "png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION")` |
| | `png_image_write_to_stdio` | `image == NULL` (pngwrite.c:2390-2391) | returns 0 |
| | `png_image_write_to_stdio` | `png_image_write_init` failed (pngwrite.c:2377-2378) | returns 0 |
| | `png_image_write_to_file` | `file_name == NULL \|\| buffer == NULL` (pngwrite.c:2402, 2448-2450) | `png_image_error(image, "png_image_write_to_file: invalid argument")`, returns 0 |
| | `png_image_write_to_file` | `fopen(file_name, "wb")` returns `NULL` (pngwrite.c:2404, 2444-2445) | `png_image_error(image, strerror(errno))`, returns 0 |
| | `png_image_write_to_file` | `fflush`/`ferror`/`fclose` failure after a successful write (pngwrite.c:2414-2432) | file removed with `remove(file_name)`, `png_image_error(image, strerror(error))`, returns 0 |
| | `png_image_write_to_file` | `png_image_write_to_stdio` returned 0 (write error) (pngwrite.c:2435-2441) | `fclose` + `remove(file_name)`, returns 0 |
| | `png_image_write_to_file` | `image->version != PNG_IMAGE_VERSION` (pngwrite.c:2400, 2453-2455) | `png_image_error(image, "png_image_write_to_file: incorrect PNG_IMAGE_VERSION")` |
| | `png_image_write_to_file` | `image == NULL` (pngwrite.c:2457-2458) | returns 0 |
| | `png_write_chunk_header` | `png_ptr == NULL` (pngwutil.c:100-101) | returns, nothing written |
| | `png_write_chunk_data` | `png_ptr == NULL` (pngwutil.c:147-148) | returns |
| | `png_write_chunk_data` | `data == NULL \|\| length == 0` (pngwutil.c:150) | nothing written and CRC not updated (chunk length may then be wrong) |
| | `png_write_chunk_end` | `png_ptr == NULL` (pngwutil.c:167) | returns |
| | `png_write_complete_chunk` | `png_ptr == NULL` (pngwutil.c:195-196) | returns |
| | `png_write_complete_chunk` | chunk data longer than the PNG limit: `length > PNG_UINT_31_MAX` (pngwutil.c:199-200) | `png_error(png_ptr, "length exceeds PNG maximum")` |
| | `png_image_size` | `png_ptr->rowbytes >= 32768 \|\| height >= 32768` (pngwutil.c:228, 255-256) | returns `0xffffffffU` — forces the maximum deflate window instead of an exact size |
| | `png_deflate_claim` | zstream already owned: `png_ptr->zowner != 0`, release build (pngwutil.c:312-328, 337) | `png_warning(png_ptr, "<cHNK>: <owner> using zstream")`, ownership stolen |
| | `png_deflate_claim` | zstream owned by IDAT (release build): `png_ptr->zowner == png_IDAT` (pngwutil.c:331-335) | `zstream.msg = "in use by IDAT"`, returns `Z_STREAM_ERROR` |
| | `png_deflate_claim` | `png_ptr->zowner != 0`, non-release build (pngwutil.c:338-340) | `png_error(png_ptr, msg)` |
| | `png_deflate_claim` | `deflateEnd` fails when re-initializing with changed parameters (pngwutil.c:412-413) | `png_warning(png_ptr, "deflateEnd failed (ignored)")` |
| | `png_deflate_claim` | `deflateInit2`/`deflateReset` returns other than `Z_OK` (bad level/method/windowBits/memLevel/strategy or OOM) (pngwutil.c:429-450) | `png_zstream_error`, returns the zlib code — callers `png_error` with `zstream.msg` |
| | `png_text_compress` | `png_deflate_claim` fails (pngwutil.c:520-523) | returns the zlib error code to the chunk writer, which calls `png_error` |
| | `png_text_compress` | compressed output would exceed the chunk limit mid-stream: `output_len + prefix_len > PNG_UINT_31_MAX` (pngwutil.c:562-566) | `ret = Z_MEM_ERROR`, loop aborted |
| | `png_text_compress` | `png_malloc_base` for an extra compression buffer fails (pngwutil.c:574-581) | `ret = Z_MEM_ERROR` |
| | `png_text_compress` | final size check `output_len + prefix_len >= PNG_UINT_31_MAX` (pngwutil.c:619-623) | `zstream.msg = "compressed data too long"`, `ret = Z_MEM_ERROR` |
| | `png_text_compress` | `deflate` did not reach `Z_STREAM_END`, or input left unconsumed: `!(ret == Z_STREAM_END && input_len == 0)` (pngwutil.c:634, 647-648) | returns the zlib error code (caller `png_error`s with `zstream.msg`) |
| | `png_write_compressed_data_out` | buffer list exhausted before all compressed bytes were written: `output_len > 0` (pngwutil.c:679-680) | `png_error(png_ptr, "error writing ancillary chunked compressed data")` |
| | `png_write_IHDR` | greyscale with `bit_depth` not in {1,2,4,8,16} (pngwutil.c:701-716) | `png_error(png_ptr, "Invalid bit depth for grayscale image")` |
| | `png_write_IHDR` | RGB with `bit_depth` not 8 (or 16 when `PNG_WRITE_16BIT_SUPPORTED`) (pngwutil.c:719-725) | `png_error(png_ptr, "Invalid bit depth for RGB image")` |
| | `png_write_IHDR` | palette with `bit_depth` not in {1,2,4,8} (pngwutil.c:730-742) | `png_error(png_ptr, "Invalid bit depth for paletted image")` |
| | `png_write_IHDR` | grey+alpha with `bit_depth` not 8/16 (pngwutil.c:745-751) | `png_error(png_ptr, "Invalid bit depth for grayscale+alpha image")` |
| | `png_write_IHDR` | RGBA with `bit_depth` not 8/16 (pngwutil.c:756-762) | `png_error(png_ptr, "Invalid bit depth for RGBA image")` |
| | `png_write_IHDR` | `color_type` not 0/2/3/4/6 (pngwutil.c:767-768) | `png_error(png_ptr, "Invalid image color type specified")` |
| | `png_write_IHDR` | `compression_type != PNG_COMPRESSION_TYPE_BASE` (pngwutil.c:771-775) | `png_warning(png_ptr, "Invalid compression type specified")`, forced to 0 |
| | `png_write_IHDR` | `filter_type != PNG_FILTER_TYPE_BASE` and not a permitted MNG intrapixel case (pngwutil.c:786-798) | `png_warning(png_ptr, "Invalid filter type specified")`, forced to 0 |
| | `png_write_IHDR` | `interlace_type` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngwutil.c:801-806) | `png_warning(png_ptr, "Invalid interlace type specified")`, forced to ADAM7 |
| | `png_write_PLTE` | palette image with `num_pal == 0` (no MNG empty-PLTE permission) or `num_pal > (1 << bit_depth)` (pngwutil.c:871-880) | `png_error(png_ptr, "Invalid number of colors in palette")` |
| | `png_write_PLTE` | same condition for a non-palette (truecolour) image, `num_pal > PNG_MAX_PALETTE_LENGTH` or 0 (pngwutil.c:882-886) | `png_warning(png_ptr, "Invalid number of colors in palette")` and return (chunk dropped) |
| | `png_write_PLTE` | PLTE requested for a greyscale image: `(color_type & PNG_COLOR_MASK_COLOR) == 0` (pngwutil.c:889-895) | `png_warning(png_ptr, "Ignoring request to write a PLTE chunk in grayscale PNG")`, chunk dropped |
| | `png_compress_IDAT` | `png_deflate_claim(png_IDAT, ...)` fails (pngwutil.c:953-954) | `png_error(png_ptr, png_ptr->zstream.msg)` |
| | `png_compress_IDAT` | `deflate` returns `Z_OK` with `input_len == 0` while `flush == Z_FINISH` (pngwutil.c:1030-1033) | `png_error(png_ptr, "Z_OK on Z_FINISH with output space")` |
| | `png_compress_IDAT` | `deflate` returns anything other than `Z_OK`/`Z_STREAM_END`-on-FINISH (pngwutil.c:1063-1068) | `png_zstream_error` then `png_error(png_ptr, png_ptr->zstream.msg)` |
| | `png_write_sRGB` | `srgb_intent >= PNG_sRGB_INTENT_LAST` (pngwutil.c:1106-1108) | `png_warning(png_ptr, "Invalid sRGB rendering intent specified")`, chunk still written |
| | `png_write_iCCP` | `profile == NULL` (pngwutil.c:1131-1132) | `png_error(png_ptr, "No profile for iCCP chunk")` |
| | `png_write_iCCP` | `profile_len < 132` (pngwutil.c:1134-1135) | `png_error(png_ptr, "ICC profile too short")` |
| | `png_write_iCCP` | `png_get_uint_32(profile) != profile_len` (pngwutil.c:1137-1138) | `png_error(png_ptr, "Incorrect data in iCCP")` |
| | `png_write_iCCP` | `profile[8] > 3 && (profile_len & 0x03)` (pngwutil.c:1140-1142) | `png_error(png_ptr, "ICC profile length invalid (not a multiple of 4)")` |
| | `png_write_iCCP` | `profile_len != embedded_profile_len` (second, redundant check) (pngwutil.c:1144-1149) | `png_error(png_ptr, "Profile length does not match profile")` |
| | `png_write_iCCP` | keyword rejected by `png_check_keyword` (empty / all-space / invalid): `name_len == 0` (pngwutil.c:1151-1154) | `png_error(png_ptr, "iCCP: invalid keyword")` |
| | `png_write_iCCP` | `png_text_compress(png_iCCP, ...) != Z_OK` (pngwutil.c:1164-1165) | `png_error(png_ptr, png_ptr->zstream.msg)` |
| | `png_write_sPLT` | `png_check_keyword(spalette->name, ...)` returns 0 (pngwutil.c:1191-1194) | `png_error(png_ptr, "sPLT: invalid keyword")` |
| | `png_write_sBIT` | colour image with `sbit->red/green/blue == 0` or `> maxbits` (`8` for palette, else `usr_bit_depth`) (pngwutil.c:1250-1256) | `png_warning(png_ptr, "Invalid sBIT depth specified")` and return (chunk dropped) |
| | `png_write_sBIT` | greyscale with `sbit->gray == 0 \|\| sbit->gray > png_ptr->usr_bit_depth` (pngwutil.c:1266-1270) | `png_warning(png_ptr, "Invalid sBIT depth specified")`, chunk dropped |
| | `png_write_sBIT` | alpha channel with `sbit->alpha == 0 \|\| sbit->alpha > png_ptr->usr_bit_depth` (pngwutil.c:1278-1282) | `png_warning(png_ptr, "Invalid sBIT depth specified")`, chunk dropped |
| | `png_write_tRNS` | palette image with `num_trans <= 0` or `num_trans > png_ptr->num_palette` (pngwutil.c:1329-1334) | `png_app_warning(png_ptr, "Invalid number of transparent colors specified")`, chunk dropped |
| | `png_write_tRNS` | greyscale with `tran->gray >= (1 << png_ptr->bit_depth)` (pngwutil.c:1344-1350) | `png_app_warning(png_ptr, "Ignoring attempt to write tRNS chunk out-of-range for bit_depth")` |
| | `png_write_tRNS` | RGB at bit depth 8 with non-zero high bytes: `bit_depth == 8 && (buf[0] \| buf[2] \| buf[4]) != 0` (pngwutil.c:1362-1371) | `png_app_warning(png_ptr, "Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8")` |
| | `png_write_tRNS` | colour type that already has an alpha channel (pngwutil.c:1376-1379) | `png_app_warning(png_ptr, "Can't write tRNS with an alpha channel")`, chunk dropped |
| | `png_write_bKGD` | palette image with `back->index >= png_ptr->num_palette` (pngwutil.c:1392-1403) | `png_warning(png_ptr, "Invalid background palette index")`, chunk dropped |
| | `png_write_bKGD` | colour at bit depth 8 with non-zero high bytes (pngwutil.c:1414-1425) | `png_warning(png_ptr, "Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8")` |
| | `png_write_bKGD` | greyscale with `back->gray >= (1 << png_ptr->bit_depth)` (pngwutil.c:1432-1438) | `png_warning(png_ptr, "Ignoring attempt to write bKGD chunk out-of-range for bit_depth")` |
| | `png_write_hIST` | `num_hist > (int)png_ptr->num_palette` (pngwutil.c:1545-1552) | `png_warning(png_ptr, "Invalid number of histogram entries specified")`, chunk dropped |
| | `png_write_tEXt` | `png_check_keyword` rejects the key: `key_len == 0` (pngwutil.c:1577-1580) | `png_error(png_ptr, "tEXt: invalid keyword")` |
| | `png_write_tEXt` | `text_len > PNG_UINT_31_MAX - (key_len+1)` (pngwutil.c:1588-1589) | `png_error(png_ptr, "tEXt: text too long")` |
| | `png_write_zTXt` | `compression` neither `PNG_TEXT_COMPRESSION_NONE` nor `PNG_TEXT_COMPRESSION_zTXt` (pngwutil.c:1621-1628) | `png_error(png_ptr, "zTXt: invalid compression type")` |
| | `png_write_zTXt` | `png_check_keyword` rejects the key (pngwutil.c:1630-1633) | `png_error(png_ptr, "zTXt: invalid keyword")` |
| | `png_write_zTXt` | `png_text_compress(png_zTXt, ...) != Z_OK` (pngwutil.c:1643-1644) | `png_error(png_ptr, png_ptr->zstream.msg)` |
| | `png_write_iTXt` | `png_check_keyword` rejects the key (pngwutil.c:1673-1676) | `png_error(png_ptr, "iTXt: invalid keyword")` |
| | `png_write_iTXt` | `compression` not one of the four `PNG_(I)TXT_COMPRESSION_NONE/zTXt` values (pngwutil.c:1679-1693) | `png_error(png_ptr, "iTXt: invalid compression")` |
| | `png_write_iTXt` | language tag / translated keyword so long that the prefix overflows: `lang_len > PNG_UINT_31_MAX-prefix_len` or `lang_key_len > PNG_UINT_31_MAX-prefix_len` (pngwutil.c:1714-1723) | `prefix_len` saturated to `PNG_UINT_31_MAX`, forcing the length errors below |
| | `png_write_iTXt` | `png_text_compress(png_iTXt, ...) != Z_OK` (pngwutil.c:1727-1731) | `png_error(png_ptr, png_ptr->zstream.msg)` |
| | `png_write_iTXt` | uncompressed iTXt where `comp.input_len > PNG_UINT_31_MAX-prefix_len` (pngwutil.c:1735-1736) | `png_error(png_ptr, "iTXt: uncompressed text too long")` |
| | `png_write_oFFs` | `unit_type >= PNG_OFFSET_LAST` (pngwutil.c:1770-1771) | `png_warning(png_ptr, "Unrecognized unit type for oFFs chunk")`, chunk still written |
| | `png_write_pCAL` | `type >= PNG_EQUATION_LAST` (pngwutil.c:1796-1797) | `png_error(png_ptr, "Unrecognized equation type for pCAL chunk")` |
| | `png_write_pCAL` | `png_check_keyword` rejects the purpose string (pngwutil.c:1799-1802) | `png_error(png_ptr, "pCAL: invalid keyword")` |
| | `png_write_sCAL_s` | `total_len = strlen(width)+strlen(height)+2 > 64` (pngwutil.c:1856-1864) | `png_warning(png_ptr, "Can't write sCAL (buffer too small)")`, chunk dropped |
| | `png_write_pHYs` | `unit_type >= PNG_RESOLUTION_LAST` (pngwutil.c:1886-1887) | `png_warning(png_ptr, "Unrecognized unit type for pHYs chunk")`, chunk still written |
| | `png_write_tIME` | `month > 12 \|\| month < 1 \|\| day > 31 \|\| day < 1 \|\| hour > 23 \|\| second > 60` (pngwutil.c:1908-1914) | `png_warning(png_ptr, "Invalid time specified for tIME chunk")`, chunk dropped |
| | `png_write_start_row` | `png_ptr->height == 1` or `png_ptr->width == 1` with UP/AVG/PAETH/SUB selected (pngwutil.c:1955-1962) | those filters silently removed; if nothing remains, `filters = PNG_FILTER_NONE` |
| | `png_write_find_filter` | row so wide that the filter cost sum could overflow: `PNG_SIZE_MAX/128 <= row_bytes` (pngwutil.c:2600-2606) | filter search abandoned; `filter_to_do &= 0U-filter_to_do` selects only the lowest set filter |
| | `png_do_write_interlace` | `pass >= 6` (last pass) (pngwutil.c:2108) | no-op; row and `row_info` left unchanged |
| | `png_do_pack` | row not 8-bit single channel: `!(row_info->bit_depth == 8 && row_info->channels == 1)` (pngwtran.c:28-30) | no packing performed and `row_info` left unchanged (silently ignored) |
| | `png_do_pack` | target `bit_depth` not 1, 2 or 4 (`default:` case) (pngwtran.c:150-151) | no packing done, but `row_info->bit_depth/pixel_depth/rowbytes` still rewritten to the requested depth |
| | `png_do_shift` | `row_info->color_type == PNG_COLOR_TYPE_PALETTE` (pngwtran.c:176) | no shifting performed (silently ignored for colour-mapped rows) |
| | `png_do_write_transformations` | `png_ptr == NULL` (pngwtran.c:504-505) | returns, no transformations applied |
