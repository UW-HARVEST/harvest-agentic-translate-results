# Error surface

Mechanically inventoried from explicit C error/sentinel returns and assertions.
The source location disambiguates repeated conditions and multi-branch returns.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---:|----------|---------------------------------------------|-------------------|-----|
| 1 | `opcodes` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:273) | `FALSE` | [x] |
| 2 | `opcodes` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:276) | `FALSE` | [x] |
| 3 | `opcodes` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:279) | `FALSE` | [x] |
| 4 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:523) | `NULL` | [x] |
| 5 | `(internal/continued)` | `--(*rec_limit) <= 0` (src/pcre2_auto_possess.c:566) | `FALSE` | [x] |
| 6 | `variable` | `base_list[1] == 0` (src/pcre2_auto_possess.c:627) | `FALSE` | [x] |
| 7 | `variable` | `cb->had_recurse` (src/pcre2_auto_possess.c:641) | `FALSE` | [x] |
| 8 | `variable` | `base_list[0] != OP_CHAR && base_list[0] != OP_CHARI` (src/pcre2_auto_possess.c:650) | `FALSE` | [x] |
| 9 | `variable` | `bracode[1+LINK_SIZE] == OP_VREVERSE` (src/pcre2_auto_possess.c:671) | `FALSE` | [x] |
| 10 | `variable` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:682) | `FALSE` | [x] |
| 11 | `variable` | `!compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit` (src/pcre2_auto_possess.c:704) | `FALSE` | [x] |
| 12 | `variable` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:717) | `FALSE` | [x] |
| 13 | `variable` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:726) | `FALSE` | [x] |
| 14 | `variable` | `code == NULL` (src/pcre2_auto_possess.c:742) | `FALSE` | [x] |
| 15 | `if` | `(*xclass_flags & XCL_HASPROP) != 0` (src/pcre2_auto_possess.c:795) | `FALSE` | [x] |
| 16 | `if` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:829) | `FALSE` | [x] |
| 17 | `if` | `(*set1++ & ~(*set2++)) != 0` (src/pcre2_auto_possess.c:840) | `FALSE` | [x] |
| 18 | `if` | `(*set1++ & *set2++) != 0` (src/pcre2_auto_possess.c:848) | `FALSE` | [x] |
| 19 | `group` | `!accepted` (src/pcre2_auto_possess.c:977) | `FALSE` | [x] |
| 20 | `group` | `chr == *ochr_ptr` (src/pcre2_auto_possess.c:997) | `FALSE` | [x] |
| 21 | `group` | `*ochr_ptr == NOTACHAR` (src/pcre2_auto_possess.c:1012) | `FALSE` | [x] |
| 22 | `group` | `chr < 256 && (cb->ctypes[chr] & ctype_digit) != 0` (src/pcre2_auto_possess.c:1019) | `FALSE` | [x] |
| 23 | `group` | `chr > 255 \|\| (cb->ctypes[chr] & ctype_digit) == 0` (src/pcre2_auto_possess.c:1023) | `FALSE` | [x] |
| 24 | `group` | `chr < 256 && (cb->ctypes[chr] & ctype_space) != 0` (src/pcre2_auto_possess.c:1027) | `FALSE` | [x] |
| 25 | `group` | `chr > 255 \|\| (cb->ctypes[chr] & ctype_space) == 0` (src/pcre2_auto_possess.c:1031) | `FALSE` | [x] |
| 26 | `group` | `chr < 255 && (cb->ctypes[chr] & ctype_word) != 0` (src/pcre2_auto_possess.c:1035) | `FALSE` | [x] |
| 27 | `group` | `chr > 255 \|\| (cb->ctypes[chr] & ctype_word) == 0` (src/pcre2_auto_possess.c:1039) | `FALSE` | [x] |
| 28 | `group` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1045) | `FALSE` | [x] |
| 29 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1054) | `FALSE` | [x] |
| 30 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1062) | `FALSE` | [x] |
| 31 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1071) | `FALSE` | [x] |
| 32 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1088) | `FALSE` | [x] |
| 33 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1100) | `FALSE` | [x] |
| 34 | `(internal/continued)` | `chr > 255` (src/pcre2_auto_possess.c:1105) | `FALSE` | [x] |
| 35 | `(internal/continued)` | `(class_bitset[chr >> 3] & (1u << (chr & 7))) != 0` (src/pcre2_auto_possess.c:1112) | `FALSE` | [x] |
| 36 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1119) | `FALSE` | [x] |
| 37 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1127) | `FALSE` | [x] |
| 38 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1132) | `FALSE` | [x] |
| 39 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1146) | `FALSE` | [x] |
| 40 | `PRIV` | `unconditional rejection on this source path` (src/pcre2_auto_possess.c:1190) | `-1` | [x] |
| 41 | `PRIV` | assertion false: `a >= 0 && b >= 0);` (src/pcre2_chkdint.c:77) | C assertion failure | [x] |
| 42 | `PRIV` | `unconditional rejection on this source path` (src/pcre2_chkdint.c:91) | `FALSE` | [x] |
| 43 | `pcre2_code_copy` | `code == NULL` (src/pcre2_compile.c:1137) | `NULL` | [x] |
| 44 | `pcre2_code_copy` | `newcode == NULL` (src/pcre2_compile.c:1139) | `NULL` | [x] |
| 45 | `pcre2_code_copy_with_tables` | `code == NULL` (src/pcre2_compile.c:1172) | `NULL` | [x] |
| 46 | `pcre2_code_copy_with_tables` | `newcode == NULL` (src/pcre2_compile.c:1174) | `NULL` | [x] |
| 47 | `pcre2_code_copy_with_tables` | `unconditional rejection on this source path` (src/pcre2_compile.c:1183) | `NULL` | [x] |
| 48 | `pcre2_code_free` | assertion false: `max_value <= INT_MAX/10 - 1);` (src/pcre2_compile.c:1268) | C assertion failure | [x] |
| 49 | `if` | `ptr >= ptrend \|\| !IS_DIGIT(*ptr)` (src/pcre2_compile.c:1287) | `FALSE` | [x] |
| 50 | `if` | `pp >= ptrend` (src/pcre2_compile.c:1375) | `FALSE` | [x] |
| 51 | `if` | `!had_minimum` (src/pcre2_compile.c:1379) | `FALSE` | [x] |
| 52 | `if` | `*pp++ != CHAR_COMMA` (src/pcre2_compile.c:1383) | `FALSE` | [x] |
| 53 | `if` | `pp >= ptrend` (src/pcre2_compile.c:1385) | `FALSE` | [x] |
| 54 | `if` | `!had_minimum` (src/pcre2_compile.c:1390) | `FALSE` | [x] |
| 55 | `if` | `pp >= ptrend \|\| *pp != CHAR_RIGHT_CURLY_BRACKET` (src/pcre2_compile.c:1392) | `FALSE` | [x] |
| 56 | `if` | assertion false: `s == INT_MAX);` (src/pcre2_compile.c:1922) | C assertion failure | [x] |
| 57 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:2399) | `FALSE` | [x] |
| 58 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:2449) | `FALSE` | [x] |
| 59 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:2454) | `FALSE` | [x] |
| 60 | `if` | `(*ptr == CHAR_LEFT_SQUARE_BRACKET && ptr[1] == terminator` (src/pcre2_compile.c:2518) | `FALSE` | [x] |
| 61 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:2527) | `FALSE` | [x] |
| 62 | `check_posix_name` | `unconditional rejection on this source path` (src/pcre2_compile.c:2558) | `-1` | [x] |
| 63 | `a` | `unconditional rejection on this source path` (src/pcre2_compile.c:2706) | `FALSE` | [x] |
| 64 | `(internal/continued)` | assertion false: `i >= 0);` (src/pcre2_compile.c:2764) | C assertion failure | [x] |
| 65 | `if` | assertion false: `next_offset > 0);` (src/pcre2_compile.c:2793) | C assertion failure | [x] |
| 66 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:2828) | `NULL` | [x] |
| 67 | `parse_regex` | assertion false: `parsed_pattern != NULL);` (src/pcre2_compile.c:3161) | C assertion failure | [x] |
| 68 | `if` | assertion false: `(parsed_pattern - parsed_pattern_check) +` (src/pcre2_compile.c:3255) | C assertion failure | [x] |
| 69 | `negated` | assertion false: `parsed_pattern_extra >= 2);` (src/pcre2_compile.c:3869) | C assertion failure | [x] |
| 70 | `if` | assertion false: `start_c == CHAR_LEFT_SQUARE_BRACKET);` (src/pcre2_compile.c:4218) | C assertion failure | [x] |
| 71 | `if` | assertion false: `class_depth_m1 >= 0);` (src/pcre2_compile.c:4222) | C assertion failure | [x] |
| 72 | `first` | assertion false: `class_depth_m1 >= 0);` (src/pcre2_compile.c:4243) | C assertion failure | [x] |
| 73 | `if` | assertion false: `class_mode_state != CLASS_MODE_PERL_EXT_LEAF);` (src/pcre2_compile.c:4319) | C assertion failure | [x] |
| 74 | `if` | assertion false: `class_depth_m1 >= 0);` (src/pcre2_compile.c:4358) | C assertion failure | [x] |
| 75 | `if` | assertion false: `class_range_state != RANGE_STARTED &&` (src/pcre2_compile.c:4364) | C assertion failure | [x] |
| 76 | `if` | assertion false: `class_depth_m1 >= 0);` (src/pcre2_compile.c:4391) | C assertion failure | [x] |
| 77 | `if` | assertion false: `class_range_state != RANGE_STARTED &&` (src/pcre2_compile.c:4397) | C assertion failure | [x] |
| 78 | `if` | assertion false: `class_depth_m1 >= 0);` (src/pcre2_compile.c:4439) | C assertion failure | [x] |
| 79 | `because` | assertion false: `i >= 0);  /* NB (?0) is permitted, represented by i=0 */` (src/pcre2_compile.c:5245) | C assertion failure | [x] |
| 80 | `assertion` | assertion false: `i >= 0);` (src/pcre2_compile.c:5453) | C assertion failure | [x] |
| 81 | `if` | assertion false: `parsed_pattern_extra > 0);` (src/pcre2_compile.c:5852) | C assertion failure | [x] |
| 82 | `if` | assertion false: `(parsed_pattern - parsed_pattern_check) +` (src/pcre2_compile.c:5882) | C assertion failure | [x] |
| 83 | `match` | assertion false: `*pptr == META_CLASS_END);` (src/pcre2_compile.c:6463) | C assertion failure | [x] |
| 84 | `RREF_ANY` | assertion false: `start_pptr[0] == META_COND_RNUMBER);` (src/pcre2_compile.c:6710) | C assertion failure | [x] |
| 85 | `RREF_ANY` | assertion false: `meta != META_CAPTURE_NAME);` (src/pcre2_compile.c:6791) | C assertion failure | [x] |
| 86 | `(internal/continued)` | assertion false: `previous != NULL);` (src/pcre2_compile.c:7353) | C assertion failure | [x] |
| 87 | `META_CODE` | assertion false: `args != NULL && args->header.type == CDATA_RECURSE_ARGS);` (src/pcre2_compile.c:8219) | C assertion failure | [x] |
| 88 | `META_CODE` | assertion false: `end > current);` (src/pcre2_compile.c:8223) | C assertion failure | [x] |
| 89 | `OP_CIRC` | `!is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor` (src/pcre2_compile.c:8910) | `FALSE` | [x] |
| 90 | `if` | `!is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor)` (src/pcre2_compile.c:8920) | `FALSE` | [x] |
| 91 | `if` | `!is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor)` (src/pcre2_compile.c:8927) | `FALSE` | [x] |
| 92 | `if` | `scode[GET(scode,1)] != OP_ALT` (src/pcre2_compile.c:8934) | `FALSE` | [x] |
| 93 | `if` | `!is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor` (src/pcre2_compile.c:8936) | `FALSE` | [x] |
| 94 | `if` | `!is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor` (src/pcre2_compile.c:8944) | `FALSE` | [x] |
| 95 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:8959) | `FALSE` | [x] |
| 96 | `if` | `op != OP_SOD && op != OP_SOM && op != OP_CIRC` (src/pcre2_compile.c:8964) | `FALSE` | [x] |
| 97 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:9030) | `FALSE` | [x] |
| 98 | `if` | `!is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor` (src/pcre2_compile.c:9034) | `FALSE` | [x] |
| 99 | `if` | `!is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor` (src/pcre2_compile.c:9049) | `FALSE` | [x] |
| 100 | `if` | `!is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor` (src/pcre2_compile.c:9060) | `FALSE` | [x] |
| 101 | `if` | `!is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor` (src/pcre2_compile.c:9068) | `FALSE` | [x] |
| 102 | `if` | `!is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor` (src/pcre2_compile.c:9076) | `FALSE` | [x] |
| 103 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:9090) | `FALSE` | [x] |
| 104 | `if` | `op != OP_CIRC && op != OP_CIRCM` (src/pcre2_compile.c:9097) | `FALSE` | [x] |
| 105 | `find_recurse` | `c == OP_END` (src/pcre2_compile.c:9129) | `NULL` | [x] |
| 106 | `parsed_skip` | `unconditional rejection on this source path` (src/pcre2_compile.c:9414) | `NULL` | [x] |
| 107 | `parsed_skip` | `meta >= sizeof(meta_extra_lengths)` (src/pcre2_compile.c:9476) | `NULL` | [x] |
| 108 | `parsed_skip` | `(groupinfo & GI_NOT_FIXED_LENGTH) != 0` (src/pcre2_compile.c:9526) | `-1` | [x] |
| 109 | `(internal/continued)` | `group > 0` (src/pcre2_compile.c:9559) | `-1` | [x] |
| 110 | `when` | `(*lcptr)++ > 2000` (src/pcre2_compile.c:9603) | `-1` | [x] |
| 111 | `switch` | `escape == ESC_X` (src/pcre2_compile.c:9689) | `-1` | [x] |
| 112 | `if` | `(cb->external_options & PCRE2_UTF) != 0 && escape == ESC_C` (src/pcre2_compile.c:9701) | `-1` | [x] |
| 113 | `if` | `*errcodeptr != 0` (src/pcre2_compile.c:9717) | `-1` | [x] |
| 114 | `if` | `!set_lookbehind_lengths(&pptr, errcodeptr, lcptr, recurses, cb` (src/pcre2_compile.c:9755) | `-1` | [x] |
| 115 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:9785) | `-1` | [x] |
| 116 | `if` | `unconditional rejection on this source path` (src/pcre2_compile.c:9831) | `-1` | [x] |
| 117 | `get_grouplength` | `*errcodeptr == 0` (src/pcre2_compile.c:9862) | `-1` | [x] |
| 118 | `get_grouplength` | `grouplength < 0` (src/pcre2_compile.c:9905) | `-1` | [x] |
| 119 | `get_grouplength` | `unconditional rejection on this source path` (src/pcre2_compile.c:9935) | `-1` | [x] |
| 120 | `get_grouplength` | `unconditional rejection on this source path` (src/pcre2_compile.c:9950) | `-1` | [x] |
| 121 | `get_grouplength` | `unconditional rejection on this source path` (src/pcre2_compile.c:9962) | `-1` | [x] |
| 122 | `get_grouplength` | `unconditional rejection on this source path` (src/pcre2_compile.c:9982) | `-1` | [x] |
| 123 | `limit` | `cb->erroroffset == PCRE2_UNSET` (src/pcre2_compile.c:10044) | `FALSE` | [x] |
| 124 | `limit` | `unconditional rejection on this source path` (src/pcre2_compile.c:10070) | `FALSE` | [x] |
| 125 | `start` | `unconditional rejection on this source path` (src/pcre2_compile.c:10127) | `ERR70` | [x] |
| 126 | `(internal/continued)` | `erroroffset != NULL` (src/pcre2_compile.c:10343) | `NULL` | [x] |
| 127 | `(internal/continued)` | `errorptr != NULL` (src/pcre2_compile.c:10348) | `NULL` | [x] |
| 128 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_compile.c:10362) | `NULL` | [x] |
| 129 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_compile.c:10381) | `NULL` | [x] |
| 130 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_compile.c:10389) | `NULL` | [x] |
| 131 | `(internal/continued)` | `patlen > ccontext->max_pattern_length` (src/pcre2_compile.c:10402) | `NULL` | [x] |
| 132 | `PRIV` | assertion false: `skipatstart <= patlen);` (src/pcre2_compile.c:10597) | C assertion failure | [x] |
| 133 | `variable` | assertion false: `(cb.char_lists_size & 0x3) == 0);` (src/pcre2_compile.c:10839) | C assertion failure | [x] |
| 134 | `copied` | assertion false: `tablecount == cb.names_found);` (src/pcre2_compile.c:10959) | C assertion failure | [x] |
| 135 | `if` | assertion false: `cb.first_data == NULL);` (src/pcre2_compile.c:11280) | C assertion failure | [x] |
| 136 | `if` | assertion false: `ptr >= pattern);` (src/pcre2_compile.c:11307) | C assertion failure | [x] |
| 137 | `if` | assertion false: `ptr <= (pattern + patlen));` (src/pcre2_compile.c:11308) | C assertion failure | [x] |
| 138 | `if` | assertion false: `prev >= pattern);` (src/pcre2_compile.c:11318) | C assertion failure | [x] |
| 139 | `if` | assertion false: `PRIV(valid_utf)(prev, ptr - prev, &dummyoffset) == 0);` (src/pcre2_compile.c:11319) | C assertion failure | [x] |
| 140 | `PRIV` | assertion false: `length > 0);` (src/pcre2_compile_cgroup.c:63) | C assertion failure | [x] |
| 141 | `PRIV` | assertion false: `hash <= NAMED_GROUP_HASH_MASK);` (src/pcre2_compile_cgroup.c:66) | C assertion failure | [x] |
| 142 | `PRIV` | `unconditional rejection on this source path` (src/pcre2_compile_cgroup.c:98) | `NULL` | [x] |
| 143 | `PRIV` | assertion false: `length > 0);` (src/pcre2_compile_cgroup.c:129) | C assertion failure | [x] |
| 144 | `case` | `unconditional rejection on this source path` (src/pcre2_compile_cgroup.c:237) | `FALSE` | [x] |
| 145 | `(internal/continued)` | assertion false: `size > 0);` (src/pcre2_compile_cgroup.c:338) | C assertion failure | [x] |
| 146 | `(internal/continued)` | assertion false: `*pptr == META_OFFSET);` (src/pcre2_compile_cgroup.c:374) | C assertion failure | [x] |
| 147 | `(internal/continued)` | `PRIV(compile_process_capture_list)(pptr - 1, 0, errorcodeptr, cb) == 0` (src/pcre2_compile_cgroup.c:376) | `NULL` | [x] |
| 148 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_compile_cgroup.c:386) | `NULL` | [x] |
| 149 | `(internal/continued)` | assertion false: `(ng->hash_dup & NAMED_GROUP_IS_DUPNAME) != 0);` (src/pcre2_compile_cgroup.c:402) | C assertion failure | [x] |
| 150 | `(internal/continued)` | assertion false: `capture_ptr < captures + size);` (src/pcre2_compile_cgroup.c:412) | C assertion failure | [x] |
| 151 | `(internal/continued)` | assertion false: `capture_ptr < captures + size);` (src/pcre2_compile_cgroup.c:437) | C assertion failure | [x] |
| 152 | `do_heapify_u16` | `size == 0` (src/pcre2_compile_cgroup.c:524) | `FALSE` | [x] |
| 153 | `do_heapify_u16` | `unconditional rejection on this source path` (src/pcre2_compile_cgroup.c:533) | `FALSE` | [x] |
| 154 | `do_heapify_u16` | assertion false: `(ng->hash_dup & NAMED_GROUP_IS_DUPNAME) != 0);` (src/pcre2_compile_cgroup.c:566) | C assertion failure | [x] |
| 155 | `do_heapify_u16` | assertion false: `size == (size_t)(captures - (uint16_t*)(args + 1)));` (src/pcre2_compile_cgroup.c:586) | C assertion failure | [x] |
| 156 | `DAMAGES` | assertion false: `(meta) <= META_END); \` (src/pcre2_compile_class.c:67) | C assertion failure | [x] |
| 157 | `if` | assertion false: `options & PARSE_CLASS_UTF);` (src/pcre2_compile_class.c:166) | C assertion failure | [x] |
| 158 | `append_char_list` | assertion false: `*p < 0xffff);` (src/pcre2_compile_class.c:278) | C assertion failure | [x] |
| 159 | `append_negated_char_list` | assertion false: `*p > 0);` (src/pcre2_compile_class.c:318) | C assertion failure | [x] |
| 160 | `append_negated_char_list` | assertion false: `*p < 0xffff);` (src/pcre2_compile_class.c:325) | C assertion failure | [x] |
| 161 | `append_non_ascii_range` | `buffer == NULL` (src/pcre2_compile_class.c:353) | `NULL` | [x] |
| 162 | `parse_class` | assertion false: `*ptr < META_END \|\| *ptr == META_BIGVALUE);` (src/pcre2_compile_class.c:446) | C assertion failure | [x] |
| 163 | `(internal/continued)` | assertion false: `(range_list_size & 0x1) == 0);` (src/pcre2_compile_class.c:535) | C assertion failure | [x] |
| 164 | `(internal/continued)` | `cranges == NULL` (src/pcre2_compile_class.c:546) | `NULL` | [x] |
| 165 | `(internal/continued)` | assertion false: `dst[1] <= get_highest_char(class_options));` (src/pcre2_compile_class.c:612) | C assertion failure | [x] |
| 166 | `(internal/continued)` | assertion false: `tmp2 <= 3 * XCL_TYPE_BIT_LEN && tmp2 >= XCL_TYPE_BIT_LEN);` (src/pcre2_compile_class.c:640) | C assertion failure | [x] |
| 167 | `(internal/continued)` | assertion false: `(uint32_t*)next_char >= dst + 2);` (src/pcre2_compile_class.c:676) | C assertion failure | [x] |
| 168 | `(internal/continued)` | assertion false: `range_start < char_list_start);` (src/pcre2_compile_class.c:692) | C assertion failure | [x] |
| 169 | `(internal/continued)` | assertion false: `(uint32_t*)next_char >= dst + 2);` (src/pcre2_compile_class.c:705) | C assertion failure | [x] |
| 170 | `(internal/continued)` | assertion false: `range_start < XCL_CHAR_LIST_LOW_16_START);` (src/pcre2_compile_class.c:726) | C assertion failure | [x] |
| 171 | `(internal/continued)` | assertion false: `(tmp2 % XCL_TYPE_BIT_LEN) == 0);` (src/pcre2_compile_class.c:730) | C assertion failure | [x] |
| 172 | `(internal/continued)` | assertion false: `(uint16_t*)dst <= next_char);` (src/pcre2_compile_class.c:738) | C assertion failure | [x] |
| 173 | `PRIV` | assertion false: `ptype == PT_PXXDIGIT);` (src/pcre2_compile_class.c:855) | C assertion failure | [x] |
| 174 | `(internal/continued)` | `cranges == NULL` (src/pcre2_compile_class.c:1128) | `NULL` | [x] |
| 175 | `(internal/continued)` | assertion false: `cranges != NULL && cranges->header.type == CDATA_CRANGE);` (src/pcre2_compile_class.c:1143) | C assertion failure | [x] |
| 176 | `list` | assertion false: `cranges != NULL);` (src/pcre2_compile_class.c:1390) | C assertion failure | [x] |
| 177 | `list` | assertion false: `cranges != NULL);` (src/pcre2_compile_class.c:1402) | C assertion failure | [x] |
| 178 | `list` | assertion false: `cranges != NULL);` (src/pcre2_compile_class.c:1414) | C assertion failure | [x] |
| 179 | `list` | assertion false: `cranges != NULL);` (src/pcre2_compile_class.c:1426) | C assertion failure | [x] |
| 180 | `(internal/continued)` | assertion false: `cranges != NULL);` (src/pcre2_compile_class.c:1554) | C assertion failure | [x] |
| 181 | `(internal/continued)` | assertion false: `cranges != NULL);` (src/pcre2_compile_class.c:1569) | C assertion failure | [x] |
| 182 | `(internal/continued)` | assertion false: `(xclass_props & XCLASS_HAS_PROPS) == 0 \|\|` (src/pcre2_compile_class.c:1576) | C assertion failure | [x] |
| 183 | `(internal/continued)` | assertion false: `(xclass_props & XCLASS_HAS_8BIT_CHARS) != 0);` (src/pcre2_compile_class.c:1586) | C assertion failure | [x] |
| 184 | `(internal/continued)` | assertion false: `(xclass_props & XCLASS_HIGH_ANY) == 0);` (src/pcre2_compile_class.c:1601) | C assertion failure | [x] |
| 185 | `(internal/continued)` | assertion false: `range + 2 == end && range[0] <= 256 &&` (src/pcre2_compile_class.c:1608) | C assertion failure | [x] |
| 186 | `(internal/continued)` | assertion false: `(char_lists_size & 0x1) == 0 &&` (src/pcre2_compile_class.c:1749) | C assertion failure | [x] |
| 187 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:1772) | `NULL` | [x] |
| 188 | `(internal/continued)` | assertion false: `cranges->char_lists_types <= XCL_TYPE_MASK);` (src/pcre2_compile_class.c:1779) | C assertion failure | [x] |
| 189 | `if` | assertion false: `pop_info->op_single_type == ECL_XCLASS &&` (src/pcre2_compile_class.c:1912) | C assertion failure | [x] |
| 190 | `if` | assertion false: `rhs_op_info->code_start ==` (src/pcre2_compile_class.c:1980) | C assertion failure | [x] |
| 191 | `if` | assertion false: `rhs_op_info->code_start ==` (src/pcre2_compile_class.c:2036) | C assertion failure | [x] |
| 192 | `if` | assertion false: `rhs_op_info->code_start ==` (src/pcre2_compile_class.c:2099) | C assertion failure | [x] |
| 193 | `if` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2167) | `FALSE` | [x] |
| 194 | `if` | assertion false: `*ptr == META_CLASS_END);` (src/pcre2_compile_class.c:2169) | C assertion failure | [x] |
| 195 | `if` | `ptr == NULL` (src/pcre2_compile_class.c:2187) | `FALSE` | [x] |
| 196 | `if` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2196) | `FALSE` | [x] |
| 197 | `if` | assertion false: `*ptr == META_CLASS_END);` (src/pcre2_compile_class.c:2203) | C assertion failure | [x] |
| 198 | `version` | assertion false: `code > code_start);` (src/pcre2_compile_class.c:2210) | C assertion failure | [x] |
| 199 | `version` | assertion false: `code - code_start == 1 && extra_length == 0);` (src/pcre2_compile_class.c:2217) | C assertion failure | [x] |
| 200 | `if` | assertion false: `code - code_start == 1 + 32 / sizeof(PCRE2_UCHAR) &&` (src/pcre2_compile_class.c:2228) | C assertion failure | [x] |
| 201 | `space` | assertion false: `*code_start == OP_XCLASS);` (src/pcre2_compile_class.c:2260) | C assertion failure | [x] |
| 202 | `space` | assertion false: `code - code_start >= 1 + LINK_SIZE + 1);` (src/pcre2_compile_class.c:2263) | C assertion failure | [x] |
| 203 | `space` | assertion false: `lengthptr == NULL \|\| (code == code_start));` (src/pcre2_compile_class.c:2281) | C assertion failure | [x] |
| 204 | `space` | `!compile_class_operand(context, negated, &ptr, &code, pop_info, lengthptr` (src/pcre2_compile_class.c:2310) | `FALSE` | [x] |
| 205 | `space` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2335) | `FALSE` | [x] |
| 206 | `space` | assertion false: `lengthptr == NULL \|\| code == start_code);` (src/pcre2_compile_class.c:2343) | C assertion failure | [x] |
| 207 | `space` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2374) | `FALSE` | [x] |
| 208 | `space` | assertion false: `lengthptr == NULL \|\| *pcode == start_code);` (src/pcre2_compile_class.c:2376) | C assertion failure | [x] |
| 209 | `(internal/continued)` | `!compile_class_unary(context, negated, &ptr, &code, pop_info, lengthptr` (src/pcre2_compile_class.c:2400) | `FALSE` | [x] |
| 210 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2426) | `FALSE` | [x] |
| 211 | `(internal/continued)` | assertion false: `lengthptr == NULL \|\| code == start_code);` (src/pcre2_compile_class.c:2434) | C assertion failure | [x] |
| 212 | `codes` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2476) | `FALSE` | [x] |
| 213 | `codes` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2516) | `FALSE` | [x] |
| 214 | `codes` | assertion false: `lengthptr == NULL \|\| code == start_code);` (src/pcre2_compile_class.c:2525) | C assertion failure | [x] |
| 215 | `codes` | assertion false: `*ptr == (META_CLASS \| CLASS_IS_ECLASS) \|\|` (src/pcre2_compile_class.c:2553) | C assertion failure | [x] |
| 216 | `codes` | `unconditional rejection on this source path` (src/pcre2_compile_class.c:2564) | `FALSE` | [x] |
| 217 | `codes` | assertion false: `**pptr == META_CLASS_END);` (src/pcre2_compile_class.c:2566) | C assertion failure | [x] |
| 218 | `codes` | assertion false: `lengthptr == NULL \|\| *pcode == start_code);` (src/pcre2_compile_class.c:2567) | C assertion failure | [x] |
| 219 | `codes` | `!compile_eclass_nested(&context, FALSE, pptr, &code, &op_info, lengthptr` (src/pcre2_compile_class.c:2594) | `FALSE` | [x] |
| 220 | `compile_class_nested` | assertion false: `op_info.op_single_type != 0);` (src/pcre2_compile_class.c:2620) | C assertion failure | [x] |
| 221 | `if` | assertion false: `op_info.op_single_type == ECL_XCLASS);` (src/pcre2_compile_class.c:2671) | C assertion failure | [x] |
| 222 | `if` | assertion false: `op_info.length >= 1 + LINK_SIZE + 1);` (src/pcre2_compile_class.c:2698) | C assertion failure | [x] |
| 223 | `if` | assertion false: `(flags & XCL_MAP) == 0);` (src/pcre2_compile_class.c:2704) | C assertion failure | [x] |
| 224 | `pcre2_config` | `unconditional rejection on this source path` (src/pcre2_config.c:78) | `PCRE2_ERROR_BADOPTION` | [x] |
| 225 | `pcre2_config` | `unconditional rejection on this source path` (src/pcre2_config.c:108) | `PCRE2_ERROR_BADOPTION` | [x] |
| 226 | `pcre2_config` | `unconditional rejection on this source path` (src/pcre2_config.c:160) | `PCRE2_ERROR_BADOPTION` | [x] |
| 227 | `PRIV` | `yield == NULL` (src/pcre2_context.c:87) | `NULL` | [x] |
| 228 | `void` | `gcontext == NULL` (src/pcre2_context.c:118) | `NULL` | [x] |
| 229 | `pcre2_compile_context_create` | `ccontext == NULL` (src/pcre2_context.c:152) | `NULL` | [x] |
| 230 | `pcre2_match_context_create` | `mcontext == NULL` (src/pcre2_context.c:188) | `NULL` | [x] |
| 231 | `pcre2_convert_context_create` | `ccontext == NULL` (src/pcre2_context.c:218) | `NULL` | [x] |
| 232 | `pcre2_general_context_copy` | `newcontext == NULL` (src/pcre2_context.c:236) | `NULL` | [x] |
| 233 | `pcre2_compile_context_copy` | `newcontext == NULL` (src/pcre2_context.c:248) | `NULL` | [x] |
| 234 | `pcre2_match_context_copy` | `newcontext == NULL` (src/pcre2_context.c:260) | `NULL` | [x] |
| 235 | `pcre2_convert_context_copy` | `newcontext == NULL` (src/pcre2_context.c:272) | `NULL` | [x] |
| 236 | `pcre2_set_bsr` | `unconditional rejection on this source path` (src/pcre2_context.c:344) | `PCRE2_ERROR_BADDATA` | [x] |
| 237 | `pcre2_set_newline` | `unconditional rejection on this source path` (src/pcre2_context.c:377) | `PCRE2_ERROR_BADDATA` | [x] |
| 238 | `pcre2_set_optimize` | `ccontext == NULL` (src/pcre2_context.c:415) | `PCRE2_ERROR_NULL` | [x] |
| 239 | `pcre2_set_optimize` | `unconditional rejection on this source path` (src/pcre2_context.c:438) | `PCRE2_ERROR_BADOPTION` | [x] |
| 240 | `pcre2_set_glob_separator` | `unconditional rejection on this source path` (src/pcre2_context.c:532) | `PCRE2_ERROR_BADDATA` | [x] |
| 241 | `pcre2_set_glob_escape` | `escape > 255 \|\| (escape != 0 && strchr(globpunct, escape) == NULL` (src/pcre2_context.c:551) | `PCRE2_ERROR_BADDATA` | [x] |
| 242 | `DAMAGES` | `p >= endp` (src/pcre2_convert.c:77) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 243 | `the` | `p + clength > endp` (src/pcre2_convert.c:242) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 244 | `switch` | `p + 6 > endp` (src/pcre2_convert.c:269) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 245 | `switch` | `plength == 0` (src/pcre2_convert.c:303) | `PCRE2_ERROR_END_BACKSLASH` | [x] |
| 246 | `switch` | `p + 1 > endp` (src/pcre2_convert.c:309) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 247 | `switch` | `p + 1 > endp` (src/pcre2_convert.c:339) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 248 | `switch` | `p + clength > endp` (src/pcre2_convert.c:370) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 249 | `switch` | `posix_state >= POSIX_CLASS_NOT_STARTED` (src/pcre2_convert.c:379) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | [x] |
| 250 | `convert_glob_char_in_class` | `c > 0xff` (src/pcre2_convert.c:573) | `FALSE` | [x] |
| 251 | `convert_glob_char_in_class` | `c == CHAR_UNDERSCORE` (src/pcre2_convert.c:585) | `FALSE` | [x] |
| 252 | `convert_glob_char_in_class` | `((cbits + cbit_digit)[c/8] & (1u << (c&7))) != 0` (src/pcre2_convert.c:586) | `FALSE` | [x] |
| 253 | `convert_glob_char_in_class` | `c == CHAR_UNDERSCORE` (src/pcre2_convert.c:594) | `FALSE` | [x] |
| 254 | `convert_glob_char_in_class` | `c == CHAR_LF \|\| c == CHAR_VT \|\| c == CHAR_FF \|\| c == CHAR_CR` (src/pcre2_convert.c:605) | `FALSE` | [x] |
| 255 | `convert_glob_char_in_class` | `unconditional rejection on this source path` (src/pcre2_convert.c:617) | `FALSE` | [x] |
| 256 | `convert_glob_char_in_class` | `pattern >= pattern_end` (src/pcre2_convert.c:654) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | [x] |
| 257 | `convert_glob_char_in_class` | `pattern >= pattern_end` (src/pcre2_convert.c:665) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | [x] |
| 258 | `if` | `c == CHAR_LEFT_SQUARE_BRACKET && *pattern == CHAR_COLON` (src/pcre2_convert.c:765) | `PCRE2_ERROR_CONVERT_SYNTAX` | [x] |
| 259 | `if` | `prev_c > c` (src/pcre2_convert.c:771) | `PCRE2_ERROR_CONVERT_SYNTAX` | [x] |
| 260 | `if` | `unconditional rejection on this source path` (src/pcre2_convert.c:803) | `PCRE2_ERROR_MISSING_SQUARE_BRACKET` | [x] |
| 261 | `convert_glob_print_commit` | `utf && (separator >= 128 \|\| escape >= 128` (src/pcre2_convert.c:872) | `PCRE2_ERROR_CONVERT_SYNTAX` | [x] |
| 262 | `(internal/continued)` | `bufflenptr != NULL` (src/pcre2_convert.c:1135) | `PCRE2_ERROR_NULL` | [x] |
| 263 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_convert.c:1143) | `PCRE2_ERROR_BADOPTION` | [x] |
| 264 | `(internal/continued)` | `utf` (src/pcre2_convert.c:1156) | `PCRE2_ERROR_UNICODE_NOT_SUPPORTED` | [x] |
| 265 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_convert.c:1206) | `PCRE2_ERROR_INTERNAL` | [x] |
| 266 | `(internal/continued)` | `allocated == NULL` (src/pcre2_convert.c:1223) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 267 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_convert.c:1235) | `PCRE2_ERROR_INTERNAL` | [x] |
| 268 | `more_workspace` | `newsize < RWS_RSIZE + ovecsize + RWS_ANCHOR_SIZE` (src/pcre2_dfa_match.c:445) | `PCRE2_ERROR_HEAPLIMIT` | [x] |
| 269 | `more_workspace` | `new == NULL` (src/pcre2_dfa_match.c:447) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 270 | `vectors` | `mb->match_call_count++ >= mb->match_limit` (src/pcre2_dfa_match.c:566) | `PCRE2_ERROR_MATCHLIMIT` | [x] |
| 271 | `vectors` | `rlevel++ > mb->match_limit_depth` (src/pcre2_dfa_match.c:567) | `PCRE2_ERROR_DEPTHLIMIT` | [x] |
| 272 | `list` | `unconditional rejection on this source path` (src/pcre2_dfa_match.c:825) | `PCRE2_ERROR_DFA_UITEM` | [x] |
| 273 | `if` | `(mb->moptions & PCRE2_PARTIAL_HARD) != 0` (src/pcre2_dfa_match.c:964) | `PCRE2_ERROR_PARTIAL` | [x] |
| 274 | `if` | `(mb->moptions & PCRE2_PARTIAL_HARD) != 0` (src/pcre2_dfa_match.c:1016) | `PCRE2_ERROR_PARTIAL` | [x] |
| 275 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_dfa_match.c:2857) | `PCRE2_ERROR_DFA_UCOND` | [x] |
| 276 | `if` | `value != RREF_ANY` (src/pcre2_dfa_match.c:2877) | `PCRE2_ERROR_DFA_UCOND` | [x] |
| 277 | `if` | `code[1 + LINK_SIZE] == OP_CREF` (src/pcre2_dfa_match.c:2943) | `PCRE2_ERROR_DFA_UITEM` | [x] |
| 278 | `if` | `unconditional rejection on this source path` (src/pcre2_dfa_match.c:2966) | `PCRE2_ERROR_RECURSELOOP` | [x] |
| 279 | `if` | `rc == 0` (src/pcre2_dfa_match.c:2995) | `PCRE2_ERROR_DFA_RECURSE` | [x] |
| 280 | `if` | `unconditional rejection on this source path` (src/pcre2_dfa_match.c:3259) | `PCRE2_ERROR_DFA_UITEM` | [x] |
| 281 | `(internal/continued)` | `match_data == NULL` (src/pcre2_dfa_match.c:3396) | `PCRE2_ERROR_NULL` | [x] |
| 282 | `pcre2_get_error_message` | `size == 0` (src/pcre2_error.c:339) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 283 | `if` | `*message == CHAR_NUL` (src/pcre2_error.c:360) | `PCRE2_ERROR_BADDATA` | [x] |
| 284 | `DAMAGES` | `unconditional rejection on this source path` (src/pcre2_extuni.c:66) | `NULL` | [x] |
| 285 | `PRIV` | `c == OP_END` (src/pcre2_find_bracket.c:73) | `NULL` | [x] |
| 286 | `xclass_compute_ranges` | assertion false: `type == XCL_SINGLE \|\| type == XCL_RANGE);` (src/pcre2_jit_char_inc.h:110) | C assertion failure | [x] |
| 287 | `xclass_compute_ranges` | assertion false: `range_count <= XCLASS_LOCAL_RANGES_SIZE);` (src/pcre2_jit_char_inc.h:124) | C assertion failure | [x] |
| 288 | `xclass_compute_ranges` | assertion false: `cc[0] >= XCL_LIST);` (src/pcre2_jit_char_inc.h:129) | C assertion failure | [x] |
| 289 | `xclass_compute_ranges` | assertion false: `item_count >= XCL_ITEM_COUNT_MASK);` (src/pcre2_jit_char_inc.h:211) | C assertion failure | [x] |
| 290 | `if` | assertion false: `range_count > 0 && range_count <= (est_range_count << 1));` (src/pcre2_jit_char_inc.h:278) | C assertion failure | [x] |
| 291 | `if` | assertion false: `next_char <= (const uint8_t*)common->start);` (src/pcre2_jit_char_inc.h:279) | C assertion failure | [x] |
| 292 | `xclass_update_min_max` | assertion false: `common->utf);` (src/pcre2_jit_char_inc.h:315) | C assertion failure | [x] |
| 293 | `xclass_update_min_max` | assertion false: `type == XCL_SINGLE \|\| type == XCL_RANGE);` (src/pcre2_jit_char_inc.h:322) | C assertion failure | [x] |
| 294 | `xclass_update_min_max` | assertion false: `min <= MAX_UTF_CODE_POINT && max <= MAX_UTF_CODE_POINT && min <= max);` (src/pcre2_jit_char_inc.h:338) | C assertion failure | [x] |
| 295 | `xclass_update_min_max` | assertion false: `cc[0] >= XCL_LIST);` (src/pcre2_jit_char_inc.h:344) | C assertion failure | [x] |
| 296 | `xclass_update_min_max` | assertion false: `type != 0);` (src/pcre2_jit_char_inc.h:357) | C assertion failure | [x] |
| 297 | `xclass_update_min_max` | assertion false: `list_ind <= 2);` (src/pcre2_jit_char_inc.h:369) | C assertion failure | [x] |
| 298 | `xclass_update_min_max` | assertion false: `list_ind <= 2 && type != 0);` (src/pcre2_jit_char_inc.h:443) | C assertion failure | [x] |
| 299 | `(internal/continued)` | assertion false: `min <= MAX_UTF_CODE_POINT && max <= MAX_UTF_CODE_POINT && min <= max);` (src/pcre2_jit_char_inc.h:477) | C assertion failure | [x] |
| 300 | `compile_xclass_matchingpath` | assertion false: `common->locals_size >= SSIZE_OF(sw));` (src/pcre2_jit_char_inc.h:526) | C assertion failure | [x] |
| 301 | `compile_xclass_matchingpath` | assertion false: `category_list == 0);` (src/pcre2_jit_char_inc.h:642) | C assertion failure | [x] |
| 302 | `compile_xclass_matchingpath` | assertion false: `compares > 0 \|\| category_list != 0);` (src/pcre2_jit_char_inc.h:655) | C assertion failure | [x] |
| 303 | `compile_xclass_matchingpath` | assertion false: `compares > 0);` (src/pcre2_jit_char_inc.h:657) | C assertion failure | [x] |
| 304 | `(internal/continued)` | assertion false: `!(status & XCLASS_IS_ECLASS));` (src/pcre2_jit_char_inc.h:678) | C assertion failure | [x] |
| 305 | `(internal/continued)` | assertion false: `compares == 1);` (src/pcre2_jit_char_inc.h:1047) | C assertion failure | [x] |
| 306 | `(internal/continued)` | assertion false: `ranges.ranges[0] == min && ranges.ranges[ranges.range_count - 1] == max);` (src/pcre2_jit_char_inc.h:1065) | C assertion failure | [x] |
| 307 | `(internal/continued)` | assertion false: `ranges.stack == ranges.local_stack);` (src/pcre2_jit_char_inc.h:1086) | C assertion failure | [x] |
| 308 | `(internal/continued)` | assertion false: `first_item < last_item && charoffset == ranges.ranges[0]);` (src/pcre2_jit_char_inc.h:1113) | C assertion failure | [x] |
| 309 | `(internal/continued)` | assertion false: `last_item >= mid_item + 4);` (src/pcre2_jit_char_inc.h:1119) | C assertion failure | [x] |
| 310 | `(internal/continued)` | assertion false: `ranges.stack == ranges.local_stack ?` (src/pcre2_jit_char_inc.h:1136) | C assertion failure | [x] |
| 311 | `(internal/continued)` | assertion false: `first_item <= last_item);` (src/pcre2_jit_char_inc.h:1165) | C assertion failure | [x] |
| 312 | `compile_eclass_matchingpath` | assertion false: `*cc == ECL_XCLASS);` (src/pcre2_jit_char_inc.h:1258) | C assertion failure | [x] |
| 313 | `compile_eclass_matchingpath` | assertion false: `*cc == ECL_XCLASS);` (src/pcre2_jit_char_inc.h:1291) | C assertion failure | [x] |
| 314 | `byte_sequence_compare` | assertion false: `othercasebit);` (src/pcre2_jit_char_inc.h:1337) | C assertion failure | [x] |
| 315 | `compile_clist` | assertion false: `cc[1] == PT_CLIST);` (src/pcre2_jit_char_inc.h:1737) | C assertion failure | [x] |
| 316 | `compile_clist` | assertion false: `other_cases[0] != NOTACHAR && other_cases[1] != NOTACHAR);` (src/pcre2_jit_char_inc.h:1755) | C assertion failure | [x] |
| 317 | `compile_clist` | assertion false: `other_cases[0] < other_cases[1] && other_cases[1] < other_cases[2]);` (src/pcre2_jit_char_inc.h:1757) | C assertion failure | [x] |
| 318 | `if` | assertion false: `other_cases[2] != NOTACHAR);` (src/pcre2_jit_char_inc.h:1775) | C assertion failure | [x] |
| 319 | `(internal/continued)` | assertion false: `TMP1 == SLJIT_R0 && STR_PTR == SLJIT_R1);` (src/pcre2_jit_char_inc.h:1996) | C assertion failure | [x] |
| 320 | `(internal/continued)` | assertion false: `type == OP_CHARI && char_has_othercase(common, cc));` (src/pcre2_jit_char_inc.h:2056) | C assertion failure | [x] |
| 321 | `(internal/continued)` | assertion false: `!is_powerof2(c ^ oc));` (src/pcre2_jit_char_inc.h:2064) | C assertion failure | [x] |
| 322 | `bracketend` | assertion false: `(*cc >= OP_ASSERT && *cc <= OP_ASSERT_SCS) \|\| (*cc >= OP_ONCE && *cc <= OP_SCOND));` (src/pcre2_jit_compile.c:876) | C assertion failure | [x] |
| 323 | `bracketend` | assertion false: `*cc >= OP_KET && *cc <= OP_KETRPOS);` (src/pcre2_jit_compile.c:878) | C assertion failure | [x] |
| 324 | `no_alternatives` | assertion false: `(*cc >= OP_ASSERT && *cc <= OP_ASSERT_SCS) \|\| (*cc >= OP_ONCE && *cc <= OP_SCOND));` (src/pcre2_jit_compile.c:886) | C assertion failure | [x] |
| 325 | `no_alternatives` | assertion false: `*cc >= OP_KET && *cc <= OP_KETRPOS);` (src/pcre2_jit_compile.c:893) | C assertion failure | [x] |
| 326 | `find_vreverse` | assertion false: `*cc == OP_ASSERTBACK \|\| *cc == OP_ASSERTBACK_NOT \|\|  *cc == OP_ASSERTBACK_NA);` (src/pcre2_jit_compile.c:899) | C assertion failure | [x] |
| 327 | `find_vreverse` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:909) | `FALSE` | [x] |
| 328 | `(internal/continued)` | `common->utf` (src/pcre2_jit_compile.c:1104) | `NULL` | [x] |
| 329 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:1127) | `NULL` | [x] |
| 330 | `check_opcode_types` | `cc[1 + LINK_SIZE] == OP_CALLOUT \|\| cc[1 + LINK_SIZE] == OP_CALLOUT_STR` (src/pcre2_jit_compile.c:1241) | `FALSE` | [x] |
| 331 | `check_opcode_types` | `cc < assert_na_end` (src/pcre2_jit_compile.c:1323) | `FALSE` | [x] |
| 332 | `(internal/continued)` | `cc == NULL` (src/pcre2_jit_compile.c:1346) | `FALSE` | [x] |
| 333 | `(internal/continued)` | assertion false: `(locals_size & (SSIZE_OF(sw) - 1)) == 0);` (src/pcre2_jit_compile.c:1351) | C assertion failure | [x] |
| 334 | `(internal/continued)` | assertion false: `common->mark_ptr == 0);` (src/pcre2_jit_compile.c:1361) | C assertion failure | [x] |
| 335 | `(internal/continued)` | assertion false: `common->recursive_head_ptr == 0);` (src/pcre2_jit_compile.c:1368) | C assertion failure | [x] |
| 336 | `(internal/continued)` | assertion false: `common->capture_last_ptr == 0);` (src/pcre2_jit_compile.c:1375) | C assertion failure | [x] |
| 337 | `detect_early_fail` | assertion false: `*cc == OP_ONCE \|\| *cc == OP_BRA \|\| *cc == OP_CBRA);` (src/pcre2_jit_compile.c:1407) | C assertion failure | [x] |
| 338 | `detect_early_fail` | assertion false: `*cc != OP_CBRA \|\| is_optimized_cbracket(common, GET2(cc, 1 + LINK_SIZE)));` (src/pcre2_jit_compile.c:1408) | C assertion failure | [x] |
| 339 | `detect_early_fail` | assertion false: `start < EARLY_FAIL_ENHANCE_MAX);` (src/pcre2_jit_compile.c:1409) | C assertion failure | [x] |
| 340 | `(internal/continued)` | assertion false: `PRIVATE_DATA(cc) == 0);` (src/pcre2_jit_compile.c:1705) | C assertion failure | [x] |
| 341 | `detect_repeat` | `end[-(1 + LINK_SIZE)] != OP_KET \|\| PRIVATE_DATA(begin) != 0` (src/pcre2_jit_compile.c:1817) | `FALSE` | [x] |
| 342 | `detect_repeat` | `min == 2` (src/pcre2_jit_compile.c:1838) | `FALSE` | [x] |
| 343 | `detect_repeat` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:1889) | `FALSE` | [x] |
| 344 | `(internal/continued)` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:2109) | C assertion failure | [x] |
| 345 | `get_framesize` | assertion false: `common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:2171) | C assertion failure | [x] |
| 346 | `get_framesize` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:2191) | C assertion failure | [x] |
| 347 | `get_framesize` | assertion false: `common->has_set_som);` (src/pcre2_jit_compile.c:2196) | C assertion failure | [x] |
| 348 | `get_framesize` | assertion false: `common->mark_ptr != 0);` (src/pcre2_jit_compile.c:2210) | C assertion failure | [x] |
| 349 | `(internal/continued)` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:2350) | C assertion failure | [x] |
| 350 | `init_frame` | assertion false: `stackpos >= stacktop + 2);` (src/pcre2_jit_compile.c:2374) | C assertion failure | [x] |
| 351 | `init_frame` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:2387) | C assertion failure | [x] |
| 352 | `init_frame` | assertion false: `common->has_set_som);` (src/pcre2_jit_compile.c:2392) | C assertion failure | [x] |
| 353 | `init_frame` | assertion false: `common->mark_ptr != 0);` (src/pcre2_jit_compile.c:2409) | C assertion failure | [x] |
| 354 | `init_frame` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:2503) | C assertion failure | [x] |
| 355 | `init_frame` | assertion false: `stackpos == STACK(stacktop));` (src/pcre2_jit_compile.c:2508) | C assertion failure | [x] |
| 356 | `delayed_mem_copy_init` | assertion false: `status->tmp_regs[i] >= 0);` (src/pcre2_jit_compile.c:2528) | C assertion failure | [x] |
| 357 | `delayed_mem_copy_init` | assertion false: `sljit_get_register_index(SLJIT_GP_REGISTER, status->saved_tmp_regs[i]) < 0 \|\| status->tmp_regs[i] == status->saved_tmp_regs[i]);` (src/pcre2_jit_compile.c:2529) | C assertion failure | [x] |
| 358 | `delayed_mem_copy_move` | assertion false: `load_base > 0 && store_base > 0);` (src/pcre2_jit_compile.c:2544) | C assertion failure | [x] |
| 359 | `recurse_check_bit` | assertion false: `(bit_index & (sizeof(sljit_sw) - 1)) == 0);` (src/pcre2_jit_compile.c:2593) | C assertion failure | [x] |
| 360 | `recurse_check_bit` | assertion false: `(bit_index >> 3) < common->recurse_bitset_size);` (src/pcre2_jit_compile.c:2597) | C assertion failure | [x] |
| 361 | `recurse_check_bit` | `*byte & mask` (src/pcre2_jit_compile.c:2603) | `FALSE` | [x] |
| 362 | `get_recurse_data_length` | assertion false: `common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:2641) | C assertion failure | [x] |
| 363 | `get_recurse_data_length` | assertion false: `common->has_set_som);` (src/pcre2_jit_compile.c:2652) | C assertion failure | [x] |
| 364 | `get_recurse_data_length` | assertion false: `PRIVATE_DATA(cc + 1) != 0);` (src/pcre2_jit_compile.c:2675) | C assertion failure | [x] |
| 365 | `get_recurse_data_length` | assertion false: `PRIVATE_DATA(cc) != 0);` (src/pcre2_jit_compile.c:2693) | C assertion failure | [x] |
| 366 | `get_recurse_data_length` | assertion false: `recurse_check_bit(common, OVECTOR((offset << 1) + 1)));` (src/pcre2_jit_compile.c:2705) | C assertion failure | [x] |
| 367 | `get_recurse_data_length` | assertion false: `PRIVATE_DATA(cc) != 0);` (src/pcre2_jit_compile.c:2716) | C assertion failure | [x] |
| 368 | `get_recurse_data_length` | assertion false: `recurse_check_bit(common, OVECTOR((offset << 1) + 1)));` (src/pcre2_jit_compile.c:2727) | C assertion failure | [x] |
| 369 | `get_recurse_data_length` | assertion false: `recurse_check_bit(common, OVECTOR((offset << 1) + 1)));` (src/pcre2_jit_compile.c:2742) | C assertion failure | [x] |
| 370 | `get_recurse_data_length` | assertion false: `recurse_check_bit(common, offset + sizeof(sljit_sw)));` (src/pcre2_jit_compile.c:2776) | C assertion failure | [x] |
| 371 | `(internal/continued)` | assertion false: `recurse_check_bit(common, offset + sizeof(sljit_sw)));` (src/pcre2_jit_compile.c:2789) | C assertion failure | [x] |
| 372 | `(internal/continued)` | assertion false: `recurse_check_bit(common, offset + sizeof(sljit_sw)));` (src/pcre2_jit_compile.c:2809) | C assertion failure | [x] |
| 373 | `(internal/continued)` | assertion false: `recurse_check_bit(common, offset + sizeof(sljit_sw)));` (src/pcre2_jit_compile.c:2819) | C assertion failure | [x] |
| 374 | `(internal/continued)` | assertion false: `common->mark_ptr != 0);` (src/pcre2_jit_compile.c:2845) | C assertion failure | [x] |
| 375 | `(internal/continued)` | assertion false: `common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:2868) | C assertion failure | [x] |
| 376 | `(internal/continued)` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:2881) | C assertion failure | [x] |
| 377 | `(internal/continued)` | assertion false: `cc == ccend);` (src/pcre2_jit_compile.c:2885) | C assertion failure | [x] |
| 378 | `copy_recurse_data` | assertion false: `common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:2936) | C assertion failure | [x] |
| 379 | `copy_recurse_data` | assertion false: `type == recurse_swap_global);` (src/pcre2_jit_compile.c:2955) | C assertion failure | [x] |
| 380 | `copy_recurse_data` | assertion false: `type == recurse_copy_from_global \|\| type == recurse_copy_private_to_global \|\| type == recurse_swap_global);` (src/pcre2_jit_compile.c:2991) | C assertion failure | [x] |
| 381 | `copy_recurse_data` | assertion false: `common->has_set_som);` (src/pcre2_jit_compile.c:3024) | C assertion failure | [x] |
| 382 | `copy_recurse_data` | assertion false: `PRIVATE_DATA(cc + 1) != 0);` (src/pcre2_jit_compile.c:3065) | C assertion failure | [x] |
| 383 | `(internal/continued)` | assertion false: `recurse_check_bit(common, shared_srcw[1]));` (src/pcre2_jit_compile.c:3097) | C assertion failure | [x] |
| 384 | `(internal/continued)` | assertion false: `recurse_check_bit(common, shared_srcw[1]));` (src/pcre2_jit_compile.c:3122) | C assertion failure | [x] |
| 385 | `(internal/continued)` | assertion false: `recurse_check_bit(common, shared_srcw[1]));` (src/pcre2_jit_compile.c:3149) | C assertion failure | [x] |
| 386 | `(internal/continued)` | assertion false: `recurse_check_bit(common, private_srcw[1]));` (src/pcre2_jit_compile.c:3200) | C assertion failure | [x] |
| 387 | `(internal/continued)` | assertion false: `recurse_check_bit(common, private_srcw[1]));` (src/pcre2_jit_compile.c:3214) | C assertion failure | [x] |
| 388 | `(internal/continued)` | assertion false: `recurse_check_bit(common, private_srcw[1]));` (src/pcre2_jit_compile.c:3235) | C assertion failure | [x] |
| 389 | `(internal/continued)` | assertion false: `recurse_check_bit(common, private_srcw[1]));` (src/pcre2_jit_compile.c:3246) | C assertion failure | [x] |
| 390 | `(internal/continued)` | assertion false: `recurse_check_bit(common, private_srcw[1]));` (src/pcre2_jit_compile.c:3274) | C assertion failure | [x] |
| 391 | `(internal/continued)` | assertion false: `common->mark_ptr != 0);` (src/pcre2_jit_compile.c:3290) | C assertion failure | [x] |
| 392 | `(internal/continued)` | assertion false: `common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:3305) | C assertion failure | [x] |
| 393 | `(internal/continued)` | assertion false: `cc != NULL);` (src/pcre2_jit_compile.c:3316) | C assertion failure | [x] |
| 394 | `(internal/continued)` | assertion false: `type == recurse_copy_from_global \|\| type == recurse_copy_private_to_global \|\| type == recurse_swap_global);` (src/pcre2_jit_compile.c:3322) | C assertion failure | [x] |
| 395 | `(internal/continued)` | assertion false: `private_srcw[i] != 0);` (src/pcre2_jit_compile.c:3326) | C assertion failure | [x] |
| 396 | `(internal/continued)` | assertion false: `type == recurse_copy_from_global \|\| type == recurse_copy_shared_to_global \|\| type == recurse_swap_global);` (src/pcre2_jit_compile.c:3342) | C assertion failure | [x] |
| 397 | `(internal/continued)` | assertion false: `shared_srcw[i] != 0);` (src/pcre2_jit_compile.c:3346) | C assertion failure | [x] |
| 398 | `(internal/continued)` | assertion false: `type == recurse_copy_from_global \|\| type == recurse_copy_shared_to_global \|\| type == recurse_copy_kept_shared_to_global);` (src/pcre2_jit_compile.c:3362) | C assertion failure | [x] |
| 399 | `(internal/continued)` | assertion false: `kept_shared_srcw[i] != 0);` (src/pcre2_jit_compile.c:3366) | C assertion failure | [x] |
| 400 | `(internal/continued)` | assertion false: `cc == ccend && stackptr == stacktop);` (src/pcre2_jit_compile.c:3381) | C assertion failure | [x] |
| 401 | `allocate_stack` | assertion false: `size > 0);` (src/pcre2_jit_compile.c:3530) | C assertion failure | [x] |
| 402 | `allocate_stack` | assertion false: `common->locals_size >= 2 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:3537) | C assertion failure | [x] |
| 403 | `free_stack` | assertion false: `size > 0);` (src/pcre2_jit_compile.c:3550) | C assertion failure | [x] |
| 404 | `allocate_read_only_data` | `SLJIT_UNLIKELY(sljit_get_compiler_error(compiler)` (src/pcre2_jit_compile.c:3560) | `NULL` | [x] |
| 405 | `allocate_read_only_data` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:3566) | `NULL` | [x] |
| 406 | `reset_ovector` | assertion false: `length > 1);` (src/pcre2_jit_compile.c:3581) | C assertion failure | [x] |
| 407 | `reset_early_fail` | assertion false: `common->early_fail_start_ptr < common->early_fail_end_ptr);` (src/pcre2_jit_compile.c:3622) | C assertion failure | [x] |
| 408 | `do_reset_match` | assertion false: `length > 1);` (src/pcre2_jit_compile.c:3669) | C assertion failure | [x] |
| 409 | `do_search_mark` | assertion false: `current[0] == 0 \|\| current < (sljit_sw*)current[0]);` (src/pcre2_jit_compile.c:3735) | C assertion failure | [x] |
| 410 | `copy_ovector` | assertion false: `sizeof(PCRE2_SIZE) == 4 \|\| sizeof(PCRE2_SIZE) == 8);` (src/pcre2_jit_compile.c:3799) | C assertion failure | [x] |
| 411 | `return_with_partial_match` | assertion false: `common->start_used_ptr != 0 && common->start_ptr != 0` (src/pcre2_jit_compile.c:3845) | C assertion failure | [x] |
| 412 | `char_get_othercase_bit` | assertion false: `c != oc);` (src/pcre2_jit_compile.c:3969) | C assertion failure | [x] |
| 413 | `check_partial` | assertion false: `!force \|\| common->mode != PCRE2_JIT_COMPLETE);` (src/pcre2_jit_compile.c:4021) | C assertion failure | [x] |
| 414 | `max` | assertion false: `min <= max);` (src/pcre2_jit_compile.c:4312) | C assertion failure | [x] |
| 415 | `is_char7_bitset` | `*bitset++ != value` (src/pcre2_jit_compile.c:4532) | `FALSE` | [x] |
| 416 | `read_char7_type` | assertion false: `common->utf);` (src/pcre2_jit_compile.c:4546) | C assertion failure | [x] |
| 417 | `if` | assertion false: `nltype == NLTYPE_FIXED && common->newline < 256);` (src/pcre2_jit_compile.c:4810) | C assertion failure | [x] |
| 418 | `character` | assertion false: `common->nltype != NLTYPE_FIXED \|\| common->newline < 128);` (src/pcre2_jit_compile.c:5073) | C assertion failure | [x] |
| 419 | `do_getucd` | assertion false: `record->script == ucp_Unknown && record->chartype == ucp_Cn && record->gbprop == ucp_gbOther);` (src/pcre2_jit_compile.c:5560) | C assertion failure | [x] |
| 420 | `do_getucd` | assertion false: `record->caseset == 0 && record->other_case == 0);` (src/pcre2_jit_compile.c:5561) | C assertion failure | [x] |
| 421 | `do_getucd` | assertion false: `UCD_BLOCK_SIZE == 128 && sizeof(ucd_record) == 12);` (src/pcre2_jit_compile.c:5564) | C assertion failure | [x] |
| 422 | `do_getucdtype` | assertion false: `record->script == ucp_Unknown && record->chartype == ucp_Cn && record->gbprop == ucp_gbOther);` (src/pcre2_jit_compile.c:5599) | C assertion failure | [x] |
| 423 | `do_getucdtype` | assertion false: `record->caseset == 0 && record->other_case == 0);` (src/pcre2_jit_compile.c:5600) | C assertion failure | [x] |
| 424 | `do_getucdtype` | assertion false: `UCD_BLOCK_SIZE == 128 && sizeof(ucd_record) == 12);` (src/pcre2_jit_compile.c:5603) | C assertion failure | [x] |
| 425 | `mainloop_entry` | assertion false: `common->abort_label == NULL);` (src/pcre2_jit_compile.c:5656) | C assertion failure | [x] |
| 426 | `mainloop_entry` | assertion false: `common->match_end_ptr != 0);` (src/pcre2_jit_compile.c:5661) | C assertion failure | [x] |
| 427 | `if` | assertion false: `common->match_end_ptr != 0);` (src/pcre2_jit_compile.c:5695) | C assertion failure | [x] |
| 428 | `scan_prefix` | assertion false: `chars <= chars_start + MAX_N_CHARS);` (src/pcre2_jit_compile.c:5911) | C assertion failure | [x] |
| 429 | `scan_prefix` | assertion false: `*cc == OP_ALT);` (src/pcre2_jit_compile.c:5929) | C assertion failure | [x] |
| 430 | `scan_prefix` | assertion false: `stack_ptr < SCAN_PREFIX_STACK_END);` (src/pcre2_jit_compile.c:5933) | C assertion failure | [x] |
| 431 | `scan_prefix` | assertion false: `chars_stack[stack_ptr] == chars);` (src/pcre2_jit_compile.c:5934) | C assertion failure | [x] |
| 432 | `scan_prefix` | assertion false: `next_alternative_stack[stack_ptr] == 1);` (src/pcre2_jit_compile.c:5935) | C assertion failure | [x] |
| 433 | `(internal/continued)` | assertion false: `chars < chars_end);` (src/pcre2_jit_compile.c:6197) | C assertion failure | [x] |
| 434 | `(internal/continued)` | assertion false: `last == TRUE && repeat == 1);` (src/pcre2_jit_compile.c:6217) | C assertion failure | [x] |
| 435 | `if` | assertion false: `(chr & 0x7) == 0);` (src/pcre2_jit_compile.c:6273) | C assertion failure | [x] |
| 436 | `check_fast_forward_char_pair_simd` | `max_pri == 0` (src/pcre2_jit_compile.c:6434) | `FALSE` | [x] |
| 437 | `fast_forward_first_char2` | assertion false: `common->mode == PCRE2_JIT_COMPLETE \|\| offset == 0);` (src/pcre2_jit_compile.c:6451) | C assertion failure | [x] |
| 438 | `fast_forward_first_n_chars` | `max < 1` (src/pcre2_jit_compile.c:6551) | `FALSE` | [x] |
| 439 | `fast_forward_first_n_chars` | assertion false: `chars[i].last_count <= chars[i].count);` (src/pcre2_jit_compile.c:6556) | C assertion failure | [x] |
| 440 | `fast_forward_first_n_chars` | assertion false: `chars[i].chars[0] != chars[i].chars[1]);` (src/pcre2_jit_compile.c:6572) | C assertion failure | [x] |
| 441 | `fast_forward_first_n_chars` | assertion false: `chars[i].count > 0);` (src/pcre2_jit_compile.c:6605) | C assertion failure | [x] |
| 442 | `fast_forward_first_n_chars` | assertion false: `chars[range_right - i].count > 0 && chars[range_right - i].count < 255);` (src/pcre2_jit_compile.c:6625) | C assertion failure | [x] |
| 443 | `if` | assertion false: `offset == -1 \|\| (chars[offset].count >= 1 && chars[offset].count <= 2));` (src/pcre2_jit_compile.c:6655) | C assertion failure | [x] |
| 444 | `if` | `offset < 0` (src/pcre2_jit_compile.c:6660) | `FALSE` | [x] |
| 445 | `if` | assertion false: `range_right != offset);` (src/pcre2_jit_compile.c:6666) | C assertion failure | [x] |
| 446 | `if` | assertion false: `range_right >= 0);` (src/pcre2_jit_compile.c:6683) | C assertion failure | [x] |
| 447 | `search_requested_char` | assertion false: `common->req_char_ptr != 0);` (src/pcre2_jit_compile.c:7033) | C assertion failure | [x] |
| 448 | `check_wordboundary` | assertion false: `common->locals_size >= 2 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:7180) | C assertion failure | [x] |
| 449 | `optimize_class_ranges` | `length >= MAX_CLASS_RANGE_SIZE` (src/pcre2_jit_compile.c:7336) | `FALSE` | [x] |
| 450 | `optimize_class_ranges` | `length >= MAX_CLASS_RANGE_SIZE` (src/pcre2_jit_compile.c:7349) | `FALSE` | [x] |
| 451 | `optimize_class_ranges` | `length < 0 \|\| length > 4` (src/pcre2_jit_compile.c:7355) | `FALSE` | [x] |
| 452 | `is_powerof2` | assertion false: `(ranges[0] & (ranges[2] - ranges[0])) == 0 && (ranges[2] & ranges[3] & (ranges[2] - ranges[0])) != 0);` (src/pcre2_jit_compile.c:7414) | C assertion failure | [x] |
| 453 | `is_powerof2` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:7461) | `FALSE` | [x] |
| 454 | `optimize_class_chars` | `!sljit_has_cpu_feature(SLJIT_HAS_CMOV` (src/pcre2_jit_compile.c:7475) | `FALSE` | [x] |
| 455 | `optimize_class_chars` | `len >= MAX_CLASS_CHARS_SIZE` (src/pcre2_jit_compile.c:7508) | `FALSE` | [x] |
| 456 | `optimize_class_chars` | `len == 0` (src/pcre2_jit_compile.c:7519) | `FALSE` | [x] |
| 457 | `do_casefulcmp` | assertion false: `common->locals_size >= SSIZE_OF(sw));` (src/pcre2_jit_compile.c:7712) | C assertion failure | [x] |
| 458 | `if` | assertion false: `common->locals_size >= 2 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:7801) | C assertion failure | [x] |
| 459 | `compile_dnref_search` | assertion false: `*cc == OP_DNREF \|\| *cc == OP_DNREFI);` (src/pcre2_jit_compile.c:8160) | C assertion failure | [x] |
| 460 | `compile_ref_matchingpath` | assertion false: `common->locals_size >= 3 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:8219) | C assertion failure | [x] |
| 461 | `compile_ref_iterator_matchingpath` | assertion false: `local_start + 2 * SSIZE_OF(sw) <= (int)LOCAL0 + common->locals_size);` (src/pcre2_jit_compile.c:8424) | C assertion failure | [x] |
| 462 | `compile_ref_iterator_matchingpath` | assertion false: `min > 1 \|\| max > 1);` (src/pcre2_jit_compile.c:8466) | C assertion failure | [x] |
| 463 | `if` | assertion false: `local_start + 3 * SSIZE_OF(sw) <= (int)LOCAL0 + common->locals_size);` (src/pcre2_jit_compile.c:8502) | C assertion failure | [x] |
| 464 | `do_callout_jit` | assertion false: `oveccount >= 1);` (src/pcre2_jit_compile.c:8800) | C assertion failure | [x] |
| 465 | `compile_callout_matchingpath` | assertion false: `common->capture_last_ptr != 0);` (src/pcre2_jit_compile.c:8860) | C assertion failure | [x] |
| 466 | `compile_callout_matchingpath` | assertion false: `TMP1 == SLJIT_R0 && STR_PTR == SLJIT_R1);` (src/pcre2_jit_compile.c:8895) | C assertion failure | [x] |
| 467 | `compile_callout_matchingpath` | assertion false: `common->locals_size >= SSIZE_OF(sw));` (src/pcre2_jit_compile.c:8898) | C assertion failure | [x] |
| 468 | `compile_reverse_matchingpath` | assertion false: `parent->top == NULL);` (src/pcre2_jit_compile.c:8931) | C assertion failure | [x] |
| 469 | `compile_reverse_matchingpath` | assertion false: `lmin > 0);` (src/pcre2_jit_compile.c:8940) | C assertion failure | [x] |
| 470 | `compile_reverse_matchingpath` | assertion false: `*cc == OP_VREVERSE);` (src/pcre2_jit_compile.c:8944) | C assertion failure | [x] |
| 471 | `compile_reverse_matchingpath` | assertion false: `lmin < lmax);` (src/pcre2_jit_compile.c:8952) | C assertion failure | [x] |
| 472 | `assert_needs_str_ptr_saving` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:9044) | `FALSE` | [x] |
| 473 | `compile_assert_matchingpath` | assertion false: `!conditional);` (src/pcre2_jit_compile.c:9087) | C assertion failure | [x] |
| 474 | `compile_assert_matchingpath` | assertion false: `private_data_ptr != 0);` (src/pcre2_jit_compile.c:9093) | C assertion failure | [x] |
| 475 | `compile_assert_matchingpath` | assertion false: `opcode >= OP_ASSERT && opcode <= OP_ASSERTBACK_NOT);` (src/pcre2_jit_compile.c:9098) | C assertion failure | [x] |
| 476 | `compile_assert_matchingpath` | assertion false: `extrasize == end_block_size + 2);` (src/pcre2_jit_compile.c:9139) | C assertion failure | [x] |
| 477 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:9221) | `NULL` | [x] |
| 478 | `(internal/continued)` | assertion false: `altbacktrack.top != NULL);` (src/pcre2_jit_compile.c:9226) | C assertion failure | [x] |
| 479 | `if` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:9319) | `NULL` | [x] |
| 480 | `if` | assertion false: `common->positive_assertion_quit == NULL);` (src/pcre2_jit_compile.c:9332) | C assertion failure | [x] |
| 481 | `if` | assertion false: `framesize != no_stack);` (src/pcre2_jit_compile.c:9342) | C assertion failure | [x] |
| 482 | `if` | assertion false: `extrasize == 3 + end_block_size);` (src/pcre2_jit_compile.c:9438) | C assertion failure | [x] |
| 483 | `if` | assertion false: `framesize != 0);` (src/pcre2_jit_compile.c:9454) | C assertion failure | [x] |
| 484 | `if` | assertion false: `found == &backtrack->common.own_backtracks);` (src/pcre2_jit_compile.c:9508) | C assertion failure | [x] |
| 485 | `do_script_run` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:9607) | `NULL` | [x] |
| 486 | `do_script_run_utf` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:9616) | `NULL` | [x] |
| 487 | `match_script_run_common` | assertion false: `TMP1 == SLJIT_R0 && STR_PTR == SLJIT_R1);` (src/pcre2_jit_compile.c:9625) | C assertion failure | [x] |
| 488 | `compile_bracket_matchingpath` | assertion false: `repeat_length != 0 && repeat_type != 0 && repeat_count != 0);` (src/pcre2_jit_compile.c:9736) | C assertion failure | [x] |
| 489 | `compile_bracket_matchingpath` | assertion false: `ket == OP_KET \|\| ket == OP_KETRMAX \|\| ket == OP_KETRMIN);` (src/pcre2_jit_compile.c:9744) | C assertion failure | [x] |
| 490 | `compile_bracket_matchingpath` | assertion false: `!((bra == OP_BRAZERO && ket == OP_KETRMIN) \|\| (bra == OP_BRAMINZERO && ket == OP_KETRMAX)));` (src/pcre2_jit_compile.c:9745) | C assertion failure | [x] |
| 491 | `if` | assertion false: `private_data_ptr != 0);` (src/pcre2_jit_compile.c:9781) | C assertion failure | [x] |
| 492 | `if` | assertion false: `private_data_ptr == OVECTOR(offset));` (src/pcre2_jit_compile.c:9946) | C assertion failure | [x] |
| 493 | `if` | assertion false: `*matchingpath == OP_DNCREF);` (src/pcre2_jit_compile.c:10027) | C assertion failure | [x] |
| 494 | `if` | assertion false: `has_alternatives);` (src/pcre2_jit_compile.c:10082) | C assertion failure | [x] |
| 495 | `if` | assertion false: `has_alternatives);` (src/pcre2_jit_compile.c:10089) | C assertion failure | [x] |
| 496 | `if` | assertion false: `!has_alternatives);` (src/pcre2_jit_compile.c:10112) | C assertion failure | [x] |
| 497 | `if` | assertion false: `has_alternatives && *matchingpath >= OP_ASSERT && *matchingpath <= OP_ASSERTBACK_NOT);` (src/pcre2_jit_compile.c:10172) | C assertion failure | [x] |
| 498 | `if` | `SLJIT_UNLIKELY(sljit_get_compiler_error(compiler)` (src/pcre2_jit_compile.c:10176) | `NULL` | [x] |
| 499 | `if` | `SLJIT_UNLIKELY(sljit_get_compiler_error(compiler)` (src/pcre2_jit_compile.c:10182) | `NULL` | [x] |
| 500 | `if` | `SLJIT_UNLIKELY(sljit_get_compiler_error(compiler)` (src/pcre2_jit_compile.c:10188) | `NULL` | [x] |
| 501 | `if` | assertion false: `backtrack->top != NULL && PRIVATE_DATA(ccbegin + 1));` (src/pcre2_jit_compile.c:10195) | C assertion failure | [x] |
| 502 | `if` | assertion false: `private_data_ptr == OVECTOR(offset + 0));` (src/pcre2_jit_compile.c:10276) | C assertion failure | [x] |
| 503 | `if` | assertion false: `framesize != 0);` (src/pcre2_jit_compile.c:10362) | C assertion failure | [x] |
| 504 | `if` | assertion false: `SHRT_MIN <= framesize && framesize < SHRT_MAX/2);` (src/pcre2_jit_compile.c:10386) | C assertion failure | [x] |
| 505 | `compile_bracketpos_matchingpath` | assertion false: `private_data_ptr != 0);` (src/pcre2_jit_compile.c:10422) | C assertion failure | [x] |
| 506 | `compile_bracketpos_matchingpath` | assertion false: `!is_optimized_cbracket(common, offset));` (src/pcre2_jit_compile.c:10436) | C assertion failure | [x] |
| 507 | `(internal/continued)` | `SLJIT_UNLIKELY(sljit_get_compiler_error(compiler)` (src/pcre2_jit_compile.c:10556) | `NULL` | [x] |
| 508 | `(internal/continued)` | `SLJIT_UNLIKELY(sljit_get_compiler_error(compiler)` (src/pcre2_jit_compile.c:10626) | `NULL` | [x] |
| 509 | `if` | assertion false: `*opcode == OP_CLASS \|\| *opcode == OP_NCLASS \|\| *opcode == OP_XCLASS \|\| *opcode == OP_ECLASS);` (src/pcre2_jit_compile.c:10712) | C assertion failure | [x] |
| 510 | `if` | assertion false: `*opcode == OP_CRRANGE \|\| *opcode == OP_CRMINRANGE \|\| *opcode == OP_CRPOSRANGE);` (src/pcre2_jit_compile.c:10744) | C assertion failure | [x] |
| 511 | `if` | assertion false: `*exact > 1);` (src/pcre2_jit_compile.c:10751) | C assertion failure | [x] |
| 512 | `if` | assertion false: `*exact > 0 \|\| *max > 1);` (src/pcre2_jit_compile.c:10766) | C assertion failure | [x] |
| 513 | `compile_iterator_matchingpath` | assertion false: `common->fast_forward_bc_ptr != NULL \|\| early_fail_ptr == 0` (src/pcre2_jit_compile.c:10854) | C assertion failure | [x] |
| 514 | `compile_iterator_matchingpath` | assertion false: `early_fail_ptr == 0 && exact >= 2);` (src/pcre2_jit_compile.c:10875) | C assertion failure | [x] |
| 515 | `compile_iterator_matchingpath` | assertion false: `tmp_base == TMP3 \|\| common->locals_size >= 3 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:10903) | C assertion failure | [x] |
| 516 | `compile_iterator_matchingpath` | assertion false: `backtrack != NULL && (early_fail_ptr == 0 \|\| opcode == OP_STAR));` (src/pcre2_jit_compile.c:10932) | C assertion failure | [x] |
| 517 | `compile_iterator_matchingpath` | assertion false: `private_data_ptr == 0);` (src/pcre2_jit_compile.c:10937) | C assertion failure | [x] |
| 518 | `compile_iterator_matchingpath` | assertion false: `early_fail_ptr == 0);` (src/pcre2_jit_compile.c:10938) | C assertion failure | [x] |
| 519 | `compile_iterator_matchingpath` | assertion false: `opcode == OP_STAR);` (src/pcre2_jit_compile.c:10942) | C assertion failure | [x] |
| 520 | `compile_iterator_matchingpath` | assertion false: `exact == 0);` (src/pcre2_jit_compile.c:10949) | C assertion failure | [x] |
| 521 | `compile_iterator_matchingpath` | assertion false: `common->locals_size >= 3 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:10958) | C assertion failure | [x] |
| 522 | `if` | assertion false: `exact == 0);` (src/pcre2_jit_compile.c:11013) | C assertion failure | [x] |
| 523 | `if` | assertion false: `tmp_base == TMP3);` (src/pcre2_jit_compile.c:11046) | C assertion failure | [x] |
| 524 | `if` | assertion false: `(charpos_othercasebit >> 8) == 0);` (src/pcre2_jit_compile.c:11067) | C assertion failure | [x] |
| 525 | `if` | assertion false: `(charpos_othercasebit >> 9) == 0);` (src/pcre2_jit_compile.c:11069) | C assertion failure | [x] |
| 526 | `if` | assertion false: `backtrack != NULL && early_fail_ptr == 0);` (src/pcre2_jit_compile.c:11288) | C assertion failure | [x] |
| 527 | `if` | assertion false: `backtrack != NULL && (opcode == OP_MINSTAR \|\| early_fail_ptr == 0));` (src/pcre2_jit_compile.c:11298) | C assertion failure | [x] |
| 528 | `if` | assertion false: `tmp_base == TMP3 && early_fail_ptr == 0);` (src/pcre2_jit_compile.c:11307) | C assertion failure | [x] |
| 529 | `if` | assertion false: `backtrack != NULL && early_fail_ptr == 0);` (src/pcre2_jit_compile.c:11341) | C assertion failure | [x] |
| 530 | `if` | assertion false: `tmp_base == TMP3);` (src/pcre2_jit_compile.c:11357) | C assertion failure | [x] |
| 531 | `if` | assertion false: `backtrack == NULL);` (src/pcre2_jit_compile.c:11376) | C assertion failure | [x] |
| 532 | `if` | assertion false: `backtrack == NULL);` (src/pcre2_jit_compile.c:11380) | C assertion failure | [x] |
| 533 | `if` | assertion false: `tmp_base == TMP3 \|\| common->locals_size >= 3 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:11400) | C assertion failure | [x] |
| 534 | `(internal/continued)` | assertion false: `backtrack == NULL && early_fail_ptr == 0);` (src/pcre2_jit_compile.c:11456) | C assertion failure | [x] |
| 535 | `(internal/continued)` | assertion false: `common->locals_size >= 3 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:11465) | C assertion failure | [x] |
| 536 | `(internal/continued)` | assertion false: `tmp_base == TMP3);` (src/pcre2_jit_compile.c:11506) | C assertion failure | [x] |
| 537 | `(internal/continued)` | assertion false: `backtrack == NULL && early_fail_ptr == 0);` (src/pcre2_jit_compile.c:11540) | C assertion failure | [x] |
| 538 | `(internal/continued)` | assertion false: `tmp_base == TMP3 \|\| common->locals_size >= 3 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:11541) | C assertion failure | [x] |
| 539 | `compile_matchingpath` | assertion false: `*ccend == OP_END \|\| (*ccend >= OP_ALT && *ccend <= OP_KETRPOS));` (src/pcre2_jit_compile.c:11706) | C assertion failure | [x] |
| 540 | `compile_matchingpath` | assertion false: `*ccend != OP_END && common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:11710) | C assertion failure | [x] |
| 541 | `(internal/continued)` | assertion false: `common->mark_ptr != 0);` (src/pcre2_jit_compile.c:11956) | C assertion failure | [x] |
| 542 | `(internal/continued)` | assertion false: `cc == ccend);` (src/pcre2_jit_compile.c:12018) | C assertion failure | [x] |
| 543 | `compile_iterator_backtrackingpath` | assertion false: `private_data_ptr == 0);` (src/pcre2_jit_compile.c:12082) | C assertion failure | [x] |
| 544 | `compile_assert_backtrackingpath` | assertion false: `*cc != OP_BRAMINZERO);` (src/pcre2_jit_compile.c:12300) | C assertion failure | [x] |
| 545 | `compile_assert_backtrackingpath` | assertion false: `current->own_backtracks == NULL);` (src/pcre2_jit_compile.c:12309) | C assertion failure | [x] |
| 546 | `compile_bracket_backtrackingpath` | assertion false: `repeat_type != 0 && repeat_count != 0);` (src/pcre2_jit_compile.c:12400) | C assertion failure | [x] |
| 547 | `if` | assertion false: `!is_optimized_cbracket(common, offset >> 1));` (src/pcre2_jit_compile.c:12495) | C assertion failure | [x] |
| 548 | `if` | assertion false: `framesize != 0);` (src/pcre2_jit_compile.c:12530) | C assertion failure | [x] |
| 549 | `if` | assertion false: `CURRENT_AS(bracket_backtrack)->matching_mov_addr != NULL);` (src/pcre2_jit_compile.c:12560) | C assertion failure | [x] |
| 550 | `if` | assertion false: `has_alternatives);` (src/pcre2_jit_compile.c:12577) | C assertion failure | [x] |
| 551 | `if` | assertion false: `assert->framesize != 0);` (src/pcre2_jit_compile.c:12579) | C assertion failure | [x] |
| 552 | `if` | assertion false: `has_alternatives);` (src/pcre2_jit_compile.c:12593) | C assertion failure | [x] |
| 553 | `if` | assertion false: `!has_alternatives);` (src/pcre2_jit_compile.c:12598) | C assertion failure | [x] |
| 554 | `if` | assertion false: `private_data_ptr != 0);` (src/pcre2_jit_compile.c:12621) | C assertion failure | [x] |
| 555 | `if` | assertion false: `current->top != NULL && PRIVATE_DATA(ccbegin + 1));` (src/pcre2_jit_compile.c:12654) | C assertion failure | [x] |
| 556 | `if` | assertion false: `private_data_ptr == OVECTOR(offset + 0));` (src/pcre2_jit_compile.c:12728) | C assertion failure | [x] |
| 557 | `if` | assertion false: `alt_count == 2 && alt_max == 3);` (src/pcre2_jit_compile.c:12742) | C assertion failure | [x] |
| 558 | `(internal/continued)` | assertion false: `!current->simple_backtracks);` (src/pcre2_jit_compile.c:12756) | C assertion failure | [x] |
| 559 | `(internal/continued)` | assertion false: `opcode == OP_COND \|\| opcode == OP_SCOND);` (src/pcre2_jit_compile.c:12762) | C assertion failure | [x] |
| 560 | `(internal/continued)` | assertion false: `assert->framesize != 0);` (src/pcre2_jit_compile.c:12766) | C assertion failure | [x] |
| 561 | `compile_braminzero_backtrackingpath` | assertion false: `!current->simple_backtracks && !current->own_backtracks);` (src/pcre2_jit_compile.c:12979) | C assertion failure | [x] |
| 562 | `compile_control_verb_backtrackingpath` | assertion false: `common->control_head_ptr != 0);` (src/pcre2_jit_compile.c:12993) | C assertion failure | [x] |
| 563 | `if` | assertion false: `common->control_head_ptr != 0 && TMP1 == SLJIT_R0 && STR_PTR == SLJIT_R1);` (src/pcre2_jit_compile.c:13030) | C assertion failure | [x] |
| 564 | `compile_then_trap_backtrackingpath` | assertion false: `framesize != 0);` (src/pcre2_jit_compile.c:13097) | C assertion failure | [x] |
| 565 | `compile_recurse` | assertion false: `*cc == OP_BRA \|\| *cc == OP_CBRA \|\| *cc == OP_CBRAPOS \|\| *cc == OP_SCBRA \|\| *cc == OP_SCBRAPOS);` (src/pcre2_jit_compile.c:13334) | C assertion failure | [x] |
| 566 | `compile_recurse` | assertion false: `common->currententry->entry_label == NULL && common->recursive_head_ptr != 0);` (src/pcre2_jit_compile.c:13340) | C assertion failure | [x] |
| 567 | `compile_recurse` | assertion false: `common->currententry->backtrack_label == NULL);` (src/pcre2_jit_compile.c:13402) | C assertion failure | [x] |
| 568 | `if` | assertion false: `alt_count == 1 && alt_max == 3);` (src/pcre2_jit_compile.c:13444) | C assertion failure | [x] |
| 569 | `if` | assertion false: `recurse_flags & recurse_flag_quit_found);` (src/pcre2_jit_compile.c:13476) | C assertion failure | [x] |
| 570 | `if` | assertion false: `recurse_flags & recurse_flag_accept_found);` (src/pcre2_jit_compile.c:13502) | C assertion failure | [x] |
| 571 | `jit_compile` | assertion false: `tables);` (src/pcre2_jit_compile.c:13556) | C assertion failure | [x] |
| 572 | `jit_compile` | assertion false: `sljit_get_register_index(SLJIT_GP_REGISTER, TMP3) < 0 && sljit_get_register_index(SLJIT_GP_REGISTER, ARGUMENTS) < 0 && sljit_get_register_index(SLJIT_GP_REGISTER, RETURN_ADDR) < 0);` (src/pcre2_jit_compile.c:13559) | C assertion failure | [x] |
| 573 | `jit_compile` | assertion false: `sljit_get_register_index(SLJIT_GP_REGISTER, TMP3) >= 0 && sljit_get_register_index(SLJIT_GP_REGISTER, ARGUMENTS) >= 0 && sljit_get_register_index(SLJIT_GP_REGISTER, RETURN_ADDR) >= 0);` (src/pcre2_jit_compile.c:13561) | C assertion failure | [x] |
| 574 | `jit_compile` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13593) | `PCRE2_ERROR_INTERNAL` | [x] |
| 575 | `if` | `private_data_length > ~(sljit_uw)0 / sizeof(sljit_s32` (src/pcre2_jit_compile.c:13655) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 576 | `if` | `~(sljit_uw)0 - private_data_length < total_length` (src/pcre2_jit_compile.c:13662) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 577 | `if` | `!common->private_data_ptrs` (src/pcre2_jit_compile.c:13667) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 578 | `if` | assertion false: `*common->start == OP_BRA && ccend[-(1 + LINK_SIZE)] == OP_KET);` (src/pcre2_jit_compile.c:13678) | C assertion failure | [x] |
| 579 | `if` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13686) | `PCRE2_ERROR_JIT_UNSUPPORTED` | [x] |
| 580 | `if` | assertion false: `!(common->req_char_ptr != 0 && common->start_used_ptr != 0));` (src/pcre2_jit_compile.c:13743) | C assertion failure | [x] |
| 581 | `if` | assertion false: `common->early_fail_start_ptr <= common->early_fail_end_ptr);` (src/pcre2_jit_compile.c:13754) | C assertion failure | [x] |
| 582 | `if` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13759) | `PCRE2_ERROR_JIT_UNSUPPORTED` | [x] |
| 583 | `if` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13769) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 584 | `if` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13781) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 585 | `(internal/continued)` | assertion false: `(private_data_size & (sizeof(sljit_sw) - 1)) == 0);` (src/pcre2_jit_compile.c:13786) | C assertion failure | [x] |
| 586 | `if` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13876) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 587 | `position` | assertion false: `common->mode == PCRE2_JIT_COMPLETE);` (src/pcre2_jit_compile.c:13935) | C assertion failure | [x] |
| 588 | `position` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:13961) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 589 | `position` | assertion false: `rootbacktrack.prev == NULL);` (src/pcre2_jit_compile.c:13964) | C assertion failure | [x] |
| 590 | `position` | assertion false: `common->restore_end_ptr == 0);` (src/pcre2_jit_compile.c:14040) | C assertion failure | [x] |
| 591 | `position` | assertion false: `common->recurse_bitset_size > 0);` (src/pcre2_jit_compile.c:14046) | C assertion failure | [x] |
| 592 | `(internal/continued)` | assertion false: `sljit_get_compiler_error(compiler) \|\| common->recurse_bitset == NULL);` (src/pcre2_jit_compile.c:14068) | C assertion failure | [x] |
| 593 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:14075) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 594 | `(internal/continued)` | assertion false: `common->restore_end_ptr == 0);` (src/pcre2_jit_compile.c:14081) | C assertion failure | [x] |
| 595 | `(internal/continued)` | assertion false: `common->locals_size >= 2 * SSIZE_OF(sw));` (src/pcre2_jit_compile.c:14087) | C assertion failure | [x] |
| 596 | `(internal/continued)` | assertion false: `TMP1 == SLJIT_R0 && STR_PTR == SLJIT_R1);` (src/pcre2_jit_compile.c:14090) | C assertion failure | [x] |
| 597 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:14236) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 598 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:14251) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 599 | `(internal/continued)` | assertion false: `mode < JIT_NUMBER_OF_COMPILE_MODES);` (src/pcre2_jit_compile.c:14265) | C assertion failure | [x] |
| 600 | `pcre2_jit_compile` | `options != PCRE2_JIT_TEST_ALLOC` (src/pcre2_jit_compile.c:14319) | `PCRE2_ERROR_JIT_BADOPTION` | [x] |
| 601 | `pcre2_jit_compile` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:14324) | `PCRE2_ERROR_JIT_UNSUPPORTED` | [x] |
| 602 | `pcre2_jit_compile` | `code == NULL` (src/pcre2_jit_compile.c:14329) | `PCRE2_ERROR_NULL` | [x] |
| 603 | `pcre2_jit_compile` | `(options & ~PUBLIC_JIT_COMPILE_OPTIONS) != 0` (src/pcre2_jit_compile.c:14332) | `PCRE2_ERROR_JIT_BADOPTION` | [x] |
| 604 | `PCRE2_MATCH_INVALID_UTF` | `functions != NULL` (src/pcre2_jit_compile.c:14369) | `PCRE2_ERROR_JIT_BADOPTION` | [x] |
| 605 | `options` | `unconditional rejection on this source path` (src/pcre2_jit_compile.c:14381) | `PCRE2_ERROR_JIT_BADOPTION` | [x] |
| 606 | `options` | `!executable_allocator_is_working` (src/pcre2_jit_compile.c:14389) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 607 | `pcre2_jit_free_unused_memory` | `unconditional rejection on this source path` (src/pcre2_jit_misc_inc.h:134) | `NULL` | [x] |
| 608 | `pcre2_jit_free_unused_memory` | `startsize == 0 \|\| maxsize == 0 \|\| maxsize > SIZE_MAX - STACK_GROWTH_RATE` (src/pcre2_jit_misc_inc.h:141) | `NULL` | [x] |
| 609 | `pcre2_jit_free_unused_memory` | `jit_stack == NULL` (src/pcre2_jit_misc_inc.h:148) | `NULL` | [x] |
| 610 | `pcre2_jit_free_unused_memory` | `unconditional rejection on this source path` (src/pcre2_jit_misc_inc.h:153) | `NULL` | [x] |
| 611 | `fast_forward_char_pair_sse2_compare` | assertion false: `step >= 0 && step <= 3);` (src/pcre2_jit_simd_inc.h:142) | C assertion failure | [x] |
| 612 | `fast_forward_char_pair_sse2_compare` | assertion false: `reg_type == SLJIT_SIMD_REG_128);` (src/pcre2_jit_simd_inc.h:191) | C assertion failure | [x] |
| 613 | `fast_forward_char_simd` | assertion false: `tmp1_reg_ind < 8);` (src/pcre2_jit_simd_inc.h:338) | C assertion failure | [x] |
| 614 | `fast_forward_char_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE);` (src/pcre2_jit_simd_inc.h:360) | C assertion failure | [x] |
| 615 | `fast_requested_char_simd` | assertion false: `tmp1_reg_ind < 8);` (src/pcre2_jit_simd_inc.h:473) | C assertion failure | [x] |
| 616 | `fast_forward_char_pair_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE && offs1 > offs2 && offs2 >= 0);` (src/pcre2_jit_simd_inc.h:528) | C assertion failure | [x] |
| 617 | `fast_forward_char_pair_simd` | assertion false: `diff <= (unsigned)IN_UCHARS(max_fast_forward_char_pair_offset()));` (src/pcre2_jit_simd_inc.h:529) | C assertion failure | [x] |
| 618 | `fast_forward_char_pair_simd` | assertion false: `sljit_get_register_index(SLJIT_GP_REGISTER, STR_PTR) <= 7);` (src/pcre2_jit_simd_inc.h:645) | C assertion failure | [x] |
| 619 | `(internal/continued)` | assertion false: `tmp1_reg_ind < 8);` (src/pcre2_jit_simd_inc.h:728) | C assertion failure | [x] |
| 620 | `fast_forward_char_pair_sse2_compare` | assertion false: `step >= 0 && step <= 2);` (src/pcre2_jit_simd_inc.h:795) | C assertion failure | [x] |
| 621 | `fast_forward_char_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE);` (src/pcre2_jit_simd_inc.h:942) | C assertion failure | [x] |
| 622 | `fast_forward_char_pair_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE && offs1 > offs2 && offs2 >= 0);` (src/pcre2_jit_simd_inc.h:1075) | C assertion failure | [x] |
| 623 | `fast_forward_char_pair_simd` | assertion false: `diff <= (unsigned)IN_UCHARS(max_fast_forward_char_pair_offset()));` (src/pcre2_jit_simd_inc.h:1076) | C assertion failure | [x] |
| 624 | `replicate_imm_vector` | assertion false: `step >= 0 && step <= 1);` (src/pcre2_jit_simd_inc.h:1295) | C assertion failure | [x] |
| 625 | `fast_forward_char_pair_sse2_compare` | assertion false: `step >= 0 && step <= 2);` (src/pcre2_jit_simd_inc.h:1336) | C assertion failure | [x] |
| 626 | `(internal/continued)` | assertion false: `common->mode == PCRE2_JIT_COMPLETE);` (src/pcre2_jit_simd_inc.h:1565) | C assertion failure | [x] |
| 627 | `fast_forward_char_pair_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE && offs1 > offs2);` (src/pcre2_jit_simd_inc.h:1776) | C assertion failure | [x] |
| 628 | `fast_forward_char_pair_simd` | assertion false: `-diff <= (sljit_s32)IN_UCHARS(max_fast_forward_char_pair_offset()));` (src/pcre2_jit_simd_inc.h:1777) | C assertion failure | [x] |
| 629 | `fast_forward_char_pair_simd` | assertion false: `tmp1_reg_ind != 0 && tmp2_reg_ind != 0);` (src/pcre2_jit_simd_inc.h:1778) | C assertion failure | [x] |
| 630 | `(internal/continued)` | assertion false: `common->mode == PCRE2_JIT_COMPLETE);` (src/pcre2_jit_simd_inc.h:1967) | C assertion failure | [x] |
| 631 | `fast_forward_char_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE);` (src/pcre2_jit_simd_inc.h:2186) | C assertion failure | [x] |
| 632 | `fast_forward_char_pair_simd` | assertion false: `common->mode == PCRE2_JIT_COMPLETE && offs1 > offs2);` (src/pcre2_jit_simd_inc.h:2329) | C assertion failure | [x] |
| 633 | `fast_forward_char_pair_simd` | assertion false: `diff <= (unsigned)IN_UCHARS(max_fast_forward_char_pair_offset()));` (src/pcre2_jit_simd_inc.h:2330) | C assertion failure | [x] |
| 634 | `pcre2_maketables` | `yield == NULL` (src/pcre2_maketables.c:94) | `NULL` | [x] |
| 635 | `offsets` | `unconditional rejection on this source path` (src/pcre2_match.c:380) | `-1` | [x] |
| 636 | `offsets` | assertion false: `eptr <= mb->end_subject);` (src/pcre2_match.c:388) | C assertion failure | [x] |
| 637 | `offsets` | `c != d` (src/pcre2_match.c:431) | `-1` | [x] |
| 638 | `if` | `caseless_restrict && *pp < 128` (src/pcre2_match.c:439) | `-1` | [x] |
| 639 | `if` | `c < *pp` (src/pcre2_match.c:443) | `-1` | [x] |
| 640 | `if` | `TABLE_GET(cp, mb->lcc, cp) != TABLE_GET(cc, mb->lcc, cc` (src/pcre2_match.c:461) | `-1` | [x] |
| 641 | `if` | `*p++ != *eptr++` (src/pcre2_match.c:479) | `-1` | [x] |
| 642 | `if` | `(PCRE2_SIZE)(mb->end_subject - eptr` (src/pcre2_match.c:488) | `-1` | [x] |
| 643 | `either` | `mb->partial > 1` (src/pcre2_match.c:629) | `PCRE2_ERROR_PARTIAL` | [x] |
| 644 | `structure` | `match_data->heapframes_size == PCRE2_SIZE_MAX - 1` (src/pcre2_match.c:768) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 645 | `structure` | `mb->heap_limit <= old_size` (src/pcre2_match.c:778) | `PCRE2_ERROR_HEAPLIMIT` | [x] |
| 646 | `structure` | `newsize - usedsize < frame_size` (src/pcre2_match.c:791) | `PCRE2_ERROR_HEAPLIMIT` | [x] |
| 647 | `structure` | `new == NULL` (src/pcre2_match.c:793) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 648 | `limit` | `mb->match_call_count++ >= mb->match_limit` (src/pcre2_match.c:873) | `PCRE2_ERROR_MATCHLIMIT` | [x] |
| 649 | `limit` | `Frdepth >= mb->match_limit_depth` (src/pcre2_match.c:874) | `PCRE2_ERROR_DEPTHLIMIT` | [x] |
| 650 | `limit` | assertion false: `offset != PCRE2_UNSET);` (src/pcre2_match.c:908) | C assertion failure | [x] |
| 651 | `limit` | `offset == PCRE2_UNSET` (src/pcre2_match.c:909) | `PCRE2_ERROR_INTERNAL` | [x] |
| 652 | `limit` | assertion false: `offset != PCRE2_UNSET);` (src/pcre2_match.c:950) | C assertion failure | [x] |
| 653 | `limit` | `offset == PCRE2_UNSET` (src/pcre2_match.c:951) | `PCRE2_ERROR_INTERNAL` | [x] |
| 654 | `way` | assertion false: `mb->hasbsk);` (src/pcre2_match.c:1027) | C assertion failure | [x] |
| 655 | `way` | `!mb->allowlookaroundbsk` (src/pcre2_match.c:1030) | `PCRE2_ERROR_BAD_BACKSLASH_K` | [x] |
| 656 | `way` | `mb->partial > 1` (src/pcre2_match.c:1070) | `PCRE2_ERROR_PARTIAL` | [x] |
| 657 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:2876) | `PCRE2_ERROR_INTERNAL` | [x] |
| 658 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:3229) | `PCRE2_ERROR_INTERNAL` | [x] |
| 659 | `if` | `mb->partial > 1` (src/pcre2_match.c:3279) | `PCRE2_ERROR_PARTIAL` | [x] |
| 660 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:3507) | `PCRE2_ERROR_INTERNAL` | [x] |
| 661 | `(internal/continued)` | `mb->partial > 1` (src/pcre2_match.c:3535) | `PCRE2_ERROR_PARTIAL` | [x] |
| 662 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:3762) | `PCRE2_ERROR_INTERNAL` | [x] |
| 663 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:4051) | `PCRE2_ERROR_INTERNAL` | [x] |
| 664 | `if` | `mb->partial > 1` (src/pcre2_match.c:4110) | `PCRE2_ERROR_PARTIAL` | [x] |
| 665 | `if` | `unconditional rejection on this source path` (src/pcre2_match.c:4208) | `PCRE2_ERROR_INTERNAL` | [x] |
| 666 | `(internal/continued)` | `mb->partial > 1` (src/pcre2_match.c:4241) | `PCRE2_ERROR_PARTIAL` | [x] |
| 667 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:4355) | `PCRE2_ERROR_INTERNAL` | [x] |
| 668 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:4626) | `PCRE2_ERROR_INTERNAL` | [x] |
| 669 | `characters` | `mb->partial > 1` (src/pcre2_match.c:4740) | `PCRE2_ERROR_PARTIAL` | [x] |
| 670 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:4947) | `PCRE2_ERROR_INTERNAL` | [x] |
| 671 | `(internal/continued)` | `mb->partial > 1` (src/pcre2_match.c:4992) | `PCRE2_ERROR_PARTIAL` | [x] |
| 672 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:5207) | `PCRE2_ERROR_INTERNAL` | [x] |
| 673 | `(internal/continued)` | `mb->partial > 1` (src/pcre2_match.c:5401) | `PCRE2_ERROR_PARTIAL` | [x] |
| 674 | `(internal/continued)` | assertion false: `(*current_branch == OP_BRA \|\| *current_branch == OP_ALT) &&` (src/pcre2_match.c:5640) | C assertion failure | [x] |
| 675 | `(internal/continued)` | assertion false: `(*Fecode == OP_BRA \|\| *Fecode == OP_ALT) &&` (src/pcre2_match.c:5650) | C assertion failure | [x] |
| 676 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_match.c:5729) | `PCRE2_ERROR_RECURSELOOP` | [x] |
| 677 | `(internal/continued)` | assertion false: `offset != PCRE2_UNSET);` (src/pcre2_match.c:6376) | C assertion failure | [x] |
| 678 | `(internal/continued)` | `offset == PCRE2_UNSET` (src/pcre2_match.c:6377) | `PCRE2_ERROR_INTERNAL` | [x] |
| 679 | `(internal/continued)` | `mb->partial > 1` (src/pcre2_match.c:6596) | `PCRE2_ERROR_PARTIAL` | [x] |
| 680 | `IS_NEWLINE` | `mb->partial > 1` (src/pcre2_match.c:6615) | `PCRE2_ERROR_PARTIAL` | [x] |
| 681 | `IS_NEWLINE` | `mb->partial > 1` (src/pcre2_match.c:6625) | `PCRE2_ERROR_PARTIAL` | [x] |
| 682 | `IS_NEWLINE` | `mb->partial > 1` (src/pcre2_match.c:6663) | `PCRE2_ERROR_PARTIAL` | [x] |
| 683 | `it` | `unconditional rejection on this source path` (src/pcre2_match.c:6889) | `PCRE2_ERROR_INTERNAL` | [x] |
| 684 | `it` | `unconditional rejection on this source path` (src/pcre2_match.c:6941) | `PCRE2_ERROR_INTERNAL` | [x] |
| 685 | `string` | `match_data == NULL` (src/pcre2_match.c:7042) | `PCRE2_ERROR_NULL` | [x] |
| 686 | `pcre2_match_data_create` | `yield == NULL` (src/pcre2_match_data.c:62) | `NULL` | [x] |
| 687 | `pcre2_match_data_create` | `code == NULL` (src/pcre2_match_data.c:84) | `NULL` | [x] |
| 688 | `do_bumpalong` | `rc < 0` (src/pcre2_match_next.c:109) | `FALSE` | [x] |
| 689 | `do_bumpalong` | assertion false: `ovector[1] >= start_offset);` (src/pcre2_match_next.c:116) | C assertion failure | [x] |
| 690 | `do_bumpalong` | `start_offset >= match_data->subject_length` (src/pcre2_match_next.c:134) | `FALSE` | [x] |
| 691 | `do_bumpalong` | `ovector[0] >= match_data->subject_length` (src/pcre2_match_next.c:152) | `FALSE` | [x] |
| 692 | `DAMAGES` | `unconditional rejection on this source path` (src/pcre2_newline.c:98) | `FALSE` | [x] |
| 693 | `switch` | `unconditional rejection on this source path` (src/pcre2_newline.c:139) | `FALSE` | [x] |
| 694 | `switch` | `unconditional rejection on this source path` (src/pcre2_newline.c:194) | `FALSE` | [x] |
| 695 | `switch` | `unconditional rejection on this source path` (src/pcre2_newline.c:235) | `FALSE` | [x] |
| 696 | `pcre2_pattern_info` | `re == NULL` (src/pcre2_pattern_info.c:107) | `PCRE2_ERROR_NULL` | [x] |
| 697 | `pcre2_pattern_info` | `re->magic_number != MAGIC_NUMBER` (src/pcre2_pattern_info.c:112) | `PCRE2_ERROR_BADMAGIC` | [x] |
| 698 | `pcre2_pattern_info` | `(re->flags & (PCRE2_CODE_UNIT_WIDTH/8)) == 0` (src/pcre2_pattern_info.c:116) | `PCRE2_ERROR_BADMODE` | [x] |
| 699 | `pcre2_pattern_info` | `re->limit_depth == UINT32_MAX` (src/pcre2_pattern_info.c:142) | `PCRE2_ERROR_UNSET` | [x] |
| 700 | `pcre2_pattern_info` | `re->limit_heap == UINT32_MAX` (src/pcre2_pattern_info.c:179) | `PCRE2_ERROR_UNSET` | [x] |
| 701 | `pcre2_pattern_info` | `re->limit_match == UINT32_MAX` (src/pcre2_pattern_info.c:210) | `PCRE2_ERROR_UNSET` | [x] |
| 702 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_pattern_info.c:242) | `PCRE2_ERROR_BADOPTION` | [x] |
| 703 | `int` | `re == NULL` (src/pcre2_pattern_info.c:276) | `PCRE2_ERROR_NULL` | [x] |
| 704 | `int` | `re->magic_number != MAGIC_NUMBER` (src/pcre2_pattern_info.c:285) | `PCRE2_ERROR_BADMAGIC` | [x] |
| 705 | `int` | `(re->flags & (PCRE2_CODE_UNIT_WIDTH/8)) == 0` (src/pcre2_pattern_info.c:289) | `PCRE2_ERROR_BADMODE` | [x] |
| 706 | `UTF` | assertion false: `ccode == code + (GET(code, 0) - 1));` (src/pcre2_printint_inc.h:626) | C assertion failure | [x] |
| 707 | `one` | `script == ucp_Unknown` (src/pcre2_script_run.c:132) | `FALSE` | [x] |
| 708 | `one` | `chspecial == 0` (src/pcre2_script_run.c:213) | `FALSE` | [x] |
| 709 | `if` | `MAPBIT(map, ucp_Han) + MAPBIT(map, ucp_Hiragana` (src/pcre2_script_run.c:230) | `FALSE` | [x] |
| 710 | `if` | `MAPBIT(map, ucp_Han) + MAPBIT(map, ucp_Bopomofo) == 0` (src/pcre2_script_run.c:234) | `FALSE` | [x] |
| 711 | `if` | `MAPBIT(map, ucp_Han) + MAPBIT(map, ucp_Hangul) == 0` (src/pcre2_script_run.c:238) | `FALSE` | [x] |
| 712 | `if` | `!OK` (src/pcre2_script_run.c:256) | `FALSE` | [x] |
| 713 | `if` | `digitset != require_digitset` (src/pcre2_script_run.c:326) | `FALSE` | [x] |
| 714 | `DAMAGES` | `codes == NULL \|\| serialized_bytes == NULL \|\| serialized_size == NULL` (src/pcre2_serialize.c:86) | `PCRE2_ERROR_NULL` | [x] |
| 715 | `DAMAGES` | `number_of_codes <= 0` (src/pcre2_serialize.c:88) | `PCRE2_ERROR_BADDATA` | [x] |
| 716 | `DAMAGES` | `codes[i] == NULL` (src/pcre2_serialize.c:96) | `PCRE2_ERROR_NULL` | [x] |
| 717 | `DAMAGES` | `re->magic_number != MAGIC_NUMBER` (src/pcre2_serialize.c:98) | `PCRE2_ERROR_BADMAGIC` | [x] |
| 718 | `if` | `tables != re->tables` (src/pcre2_serialize.c:102) | `PCRE2_ERROR_MIXEDTABLES` | [x] |
| 719 | `if` | `bytes == NULL` (src/pcre2_serialize.c:108) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 720 | `if` | `data == NULL \|\| codes == NULL` (src/pcre2_serialize.c:176) | `PCRE2_ERROR_NULL` | [x] |
| 721 | `if` | `number_of_codes <= 0` (src/pcre2_serialize.c:177) | `PCRE2_ERROR_BADDATA` | [x] |
| 722 | `if` | `data->number_of_codes <= 0` (src/pcre2_serialize.c:178) | `PCRE2_ERROR_BADSERIALIZEDDATA` | [x] |
| 723 | `if` | `data->magic != SERIALIZED_DATA_MAGIC` (src/pcre2_serialize.c:179) | `PCRE2_ERROR_BADMAGIC` | [x] |
| 724 | `if` | `data->version != SERIALIZED_DATA_VERSION` (src/pcre2_serialize.c:180) | `PCRE2_ERROR_BADMODE` | [x] |
| 725 | `if` | `data->config != SERIALIZED_DATA_CONFIG` (src/pcre2_serialize.c:181) | `PCRE2_ERROR_BADMODE` | [x] |
| 726 | `if` | `tables == NULL` (src/pcre2_serialize.c:192) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 727 | `pcre2_serialize_get_number_of_codes` | `data == NULL` (src/pcre2_serialize.c:272) | `PCRE2_ERROR_NULL` | [x] |
| 728 | `pcre2_serialize_get_number_of_codes` | `data->magic != SERIALIZED_DATA_MAGIC` (src/pcre2_serialize.c:273) | `PCRE2_ERROR_BADMAGIC` | [x] |
| 729 | `pcre2_serialize_get_number_of_codes` | `data->version != SERIALIZED_DATA_VERSION` (src/pcre2_serialize.c:274) | `PCRE2_ERROR_BADMODE` | [x] |
| 730 | `pcre2_serialize_get_number_of_codes` | `data->config != SERIALIZED_DATA_CONFIG` (src/pcre2_serialize.c:275) | `PCRE2_ERROR_BADMODE` | [x] |
| 731 | `count` | `(*countptr)++ > 1000` (src/pcre2_study.c:129) | `-1` | [x] |
| 732 | `count` | `unconditional rejection on this source path` (src/pcre2_study.c:222) | `-1` | [x] |
| 733 | `(internal/continued)` | `utf` (src/pcre2_study.c:388) | `-1` | [x] |
| 734 | `string` | `cs == NULL` (src/pcre2_study.c:500) | `-2` | [x] |
| 735 | `string` | `cs == NULL` (src/pcre2_study.c:560) | `-2` | [x] |
| 736 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_study.c:758) | `-3` | [x] |
| 737 | `(internal/continued)` | `unconditional rejection on this source path` (src/pcre2_study.c:765) | `-3` | [x] |
| 738 | `text` | `unconditional rejection on this source path` (src/pcre2_substitute.c:260) | `FALSE` | [x] |
| 739 | `pessimistic_case_inflation` | assertion false: `(char *)(input + input_len) <= (char *)output \|\|` (src/pcre2_substitute.c:320) | C assertion failure | [x] |
| 740 | `(internal/continued)` | assertion false: `input_len != 0);` (src/pcre2_substitute.c:468) | C assertion failure | [x] |
| 741 | `(internal/continued)` | assertion false: `ch_end <= input + input_len && ch_end - input <= 6);` (src/pcre2_substitute.c:515) | C assertion failure | [x] |
| 742 | `(internal/continued)` | assertion false: `output_cap >= input_len && input_len >= rest_len);` (src/pcre2_substitute.c:534) | C assertion failure | [x] |
| 743 | `(internal/continued)` | assertion false: `rest_len <= output_cap - rc);` (src/pcre2_substitute.c:567) | C assertion failure | [x] |
| 744 | `xform` | assertion false: `!(ch1_overflow \|\| rest_overflow) \|\| rc + rc2 > output_cap);` (src/pcre2_substitute.c:601) | C assertion failure | [x] |
| 745 | `supported` | `partial && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) == 0` (src/pcre2_substitute.c:797) | `PCRE2_ERROR_BADOPTION` | [x] |
| 746 | `supported` | `rlength != 0` (src/pcre2_substitute.c:804) | `PCRE2_ERROR_NULL` | [x] |
| 747 | `supported` | `length != 0` (src/pcre2_substitute.c:815) | `PCRE2_ERROR_NULL` | [x] |
| 748 | `supported` | `use_existing_match && match_data == NULL` (src/pcre2_substitute.c:827) | `PCRE2_ERROR_NULL` | [x] |
| 749 | `supported` | `match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER` (src/pcre2_substitute.c:850) | `PCRE2_ERROR_DFA_UFUNC` | [x] |
| 750 | `supported` | `code != match_data->code` (src/pcre2_substitute.c:853) | `PCRE2_ERROR_DIFFSUBSPATTERN` | [x] |
| 751 | `supported` | `unconditional rejection on this source path` (src/pcre2_substitute.c:864) | `PCRE2_ERROR_DIFFSUBSSUBJECT` | [x] |
| 752 | `supported` | `start_offset != match_data->start_offset` (src/pcre2_substitute.c:867) | `PCRE2_ERROR_DIFFSUBSOFFSET` | [x] |
| 753 | `supported` | `unconditional rejection on this source path` (src/pcre2_substitute.c:871) | `PCRE2_ERROR_DIFFSUBSOPTIONS` | [x] |
| 754 | `pcre2_general_context_create` | `internal_match_data == NULL` (src/pcre2_substitute.c:895) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 755 | `if` | `internal_match_data == NULL` (src/pcre2_substitute.c:909) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 756 | `pcre2_next_match` | assertion false: `start_offset == ovector[1]);` (src/pcre2_substitute.c:1728) | C assertion failure | [x] |
| 757 | `DAMAGES` | `match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER` (src/pcre2_substring.c:75) | `PCRE2_ERROR_DFA_UFUNC` | [x] |
| 758 | `DAMAGES` | `size + 1 > *sizeptr` (src/pcre2_substring.c:124) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 759 | `DAMAGES` | `match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER` (src/pcre2_substring.c:163) | `PCRE2_ERROR_DFA_UFUNC` | [x] |
| 760 | `(internal/continued)` | `yield == NULL` (src/pcre2_substring.c:215) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 761 | `pcre2_substring_free` | `match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER` (src/pcre2_substring.c:270) | `PCRE2_ERROR_DFA_UFUNC` | [x] |
| 762 | `end` | `stringnumber > 0` (src/pcre2_substring.c:319) | `PCRE2_ERROR_PARTIAL` | [x] |
| 763 | `end` | `stringnumber > match_data->code->top_bracket` (src/pcre2_substring.c:327) | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 764 | `end` | `stringnumber >= match_data->oveccount` (src/pcre2_substring.c:329) | `PCRE2_ERROR_UNAVAILABLE` | [x] |
| 765 | `end` | `match_data->ovector[stringnumber*2] == PCRE2_UNSET` (src/pcre2_substring.c:331) | `PCRE2_ERROR_UNSET` | [x] |
| 766 | `end` | `stringnumber >= match_data->oveccount` (src/pcre2_substring.c:335) | `PCRE2_ERROR_UNAVAILABLE` | [x] |
| 767 | `end` | `count != 0 && stringnumber >= (uint32_t)count` (src/pcre2_substring.c:336) | `PCRE2_ERROR_UNSET` | [x] |
| 768 | `end` | `unconditional rejection on this source path` (src/pcre2_substring.c:347) | `PCRE2_ERROR_INVALIDOFFSET` | [x] |
| 769 | `lengths` | `memp == NULL` (src/pcre2_substring.c:404) | `PCRE2_ERROR_NOMEMORY` | [x] |
| 770 | `Find` | `unconditional rejection on this source path` (src/pcre2_substring.c:525) | `PCRE2_ERROR_NOSUBSTRING` | [x] |
| 771 | `DAMAGES` | assertion false: `x) can be used to inject an assert() for conditions` (src/pcre2_util.h:52) | C assertion failure | [x] |
| 772 | `assert` | assertion false: `), and therefore the expression used` (src/pcre2_util.h:57) | C assertion failure | [x] |
| 773 | `assert` | assertion false: `x) assert(x)` (src/pcre2_util.h:62) | C assertion failure | [x] |
| 774 | `assert` | assertion false: `x) do                                            \` (src/pcre2_util.h:64) | C assertion failure | [x] |
| 775 | `there` | assertion false: `((void)"Execution reached unexpected point", 0))` (src/pcre2_util.h:92) | C assertion failure | [x] |
| 776 | `there` | assertion false: `x) do {} while(0)` (src/pcre2_util.h:117) | C assertion failure | [x] |
| 777 | `byte` | `c < 0xc0` (src/pcre2_valid_utf.c:145) | `PCRE2_ERROR_UTF8_ERR20` | [x] |
| 778 | `byte` | `c >= 0xfe` (src/pcre2_valid_utf.c:151) | `PCRE2_ERROR_UTF8_ERR21` | [x] |
| 779 | `byte` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:160) | `PCRE2_ERROR_UTF8_ERR1` | [x] |
| 780 | `byte` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:161) | `PCRE2_ERROR_UTF8_ERR2` | [x] |
| 781 | `byte` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:162) | `PCRE2_ERROR_UTF8_ERR3` | [x] |
| 782 | `byte` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:163) | `PCRE2_ERROR_UTF8_ERR4` | [x] |
| 783 | `byte` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:164) | `PCRE2_ERROR_UTF8_ERR5` | [x] |
| 784 | `byte` | `((d = *(++p)) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:174) | `PCRE2_ERROR_UTF8_ERR6` | [x] |
| 785 | `byte` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:189) | `PCRE2_ERROR_UTF8_ERR15` | [x] |
| 786 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:201) | `PCRE2_ERROR_UTF8_ERR7` | [x] |
| 787 | `byte` | `c == 0xe0 && (d & 0x20) == 0` (src/pcre2_valid_utf.c:206) | `PCRE2_ERROR_UTF8_ERR16` | [x] |
| 788 | `byte` | `c == 0xed && d >= 0xa0` (src/pcre2_valid_utf.c:211) | `PCRE2_ERROR_UTF8_ERR14` | [x] |
| 789 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:223) | `PCRE2_ERROR_UTF8_ERR7` | [x] |
| 790 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:228) | `PCRE2_ERROR_UTF8_ERR8` | [x] |
| 791 | `byte` | `c == 0xf0 && (d & 0x30) == 0` (src/pcre2_valid_utf.c:233) | `PCRE2_ERROR_UTF8_ERR17` | [x] |
| 792 | `byte` | `c > 0xf4 \|\| (c == 0xf4 && d > 0x8f` (src/pcre2_valid_utf.c:238) | `PCRE2_ERROR_UTF8_ERR13` | [x] |
| 793 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:254) | `PCRE2_ERROR_UTF8_ERR7` | [x] |
| 794 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:259) | `PCRE2_ERROR_UTF8_ERR8` | [x] |
| 795 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:264) | `PCRE2_ERROR_UTF8_ERR9` | [x] |
| 796 | `byte` | `c == 0xf8 && (d & 0x38) == 0` (src/pcre2_valid_utf.c:269) | `PCRE2_ERROR_UTF8_ERR18` | [x] |
| 797 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:280) | `PCRE2_ERROR_UTF8_ERR7` | [x] |
| 798 | `byte` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:285) | `PCRE2_ERROR_UTF8_ERR8` | [x] |
| 799 | `(internal/continued)` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:290) | `PCRE2_ERROR_UTF8_ERR9` | [x] |
| 800 | `(internal/continued)` | `(*(++p) & 0xc0) != 0x80` (src/pcre2_valid_utf.c:295) | `PCRE2_ERROR_UTF8_ERR10` | [x] |
| 801 | `(internal/continued)` | `c == 0xfc && (d & 0x3c) == 0` (src/pcre2_valid_utf.c:300) | `PCRE2_ERROR_UTF8_ERR19` | [x] |
| 802 | `if` | `length == 0` (src/pcre2_valid_utf.c:343) | `PCRE2_ERROR_UTF16_ERR1` | [x] |
| 803 | `if` | `(*p & 0xfc00) != 0xdc00` (src/pcre2_valid_utf.c:350) | `PCRE2_ERROR_UTF16_ERR2` | [x] |
| 804 | `if` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:357) | `PCRE2_ERROR_UTF16_ERR3` | [x] |
| 805 | `if` | `c > 0x10ffffu` (src/pcre2_valid_utf.c:382) | `PCRE2_ERROR_UTF32_ERR2` | [x] |
| 806 | `if` | `unconditional rejection on this source path` (src/pcre2_valid_utf.c:389) | `PCRE2_ERROR_UTF32_ERR1` | [x] |
| 807 | `C` | `unconditional rejection on this source path` (src/pcre2_xclass.c:256) | `FALSE` | [x] |
| 808 | `C` | assertion false: `t == XCL_RANGE);` (src/pcre2_xclass.c:292) | C assertion failure | [x] |
| 809 | `C` | assertion false: `((uintptr_t)next_char & 0x1) == 0);` (src/pcre2_xclass.c:323) | C assertion failure | [x] |
| 810 | `C` | assertion false: `max_index >= XCL_ITEM_COUNT_MASK);` (src/pcre2_xclass.c:331) | C assertion failure | [x] |
| 811 | `C` | assertion false: `max_index >= XCL_ITEM_COUNT_MASK);` (src/pcre2_xclass.c:348) | C assertion failure | [x] |
| 812 | `if` | assertion false: `max_index >= XCL_ITEM_COUNT_MASK);` (src/pcre2_xclass.c:382) | C assertion failure | [x] |
| 813 | `if` | assertion false: `((uintptr_t)next_char & 0x3) == 0);` (src/pcre2_xclass.c:390) | C assertion failure | [x] |
| 814 | `if` | assertion false: `max_index >= XCL_ITEM_COUNT_MASK);` (src/pcre2_xclass.c:400) | C assertion failure | [x] |
| 815 | `if` | assertion false: `data_start < data_end);` (src/pcre2_xclass.c:470) | C assertion failure | [x] |
| 816 | `if` | assertion false: `(flags & ECL_MAP) == 0 \|\|` (src/pcre2_xclass.c:472) | C assertion failure | [x] |
| 817 | `if` | assertion false: `stack_depth >= 2);` (src/pcre2_xclass.c:495) | C assertion failure | [x] |
| 818 | `if` | assertion false: `stack_depth >= 2);` (src/pcre2_xclass.c:502) | C assertion failure | [x] |
| 819 | `if` | assertion false: `stack_depth >= 2);` (src/pcre2_xclass.c:509) | C assertion failure | [x] |
| 820 | `if` | assertion false: `stack_depth >= 1);` (src/pcre2_xclass.c:516) | C assertion failure | [x] |
| 821 | `if` | `unconditional rejection on this source path` (src/pcre2_xclass.c:535) | `FALSE` | [x] |
| 822 | `if` | assertion false: `stack_depth == 1);` (src/pcre2_xclass.c:540) | C assertion failure | [x] |
