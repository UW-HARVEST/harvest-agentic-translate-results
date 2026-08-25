# Error surface

Mechanically extracted from C rejection returns, `RETURN_ERROR`, error-code assignments, and assertions. Each source location is retained so multi-line conditions can be audited against the ground truth.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|----------|---------------------------------------------|-------------------|
| 1 | `pcre2_code_copy` | `if (code == NULL) [c_src/src/pcre2_compile.c:1137]` | [x] `return NULL` |
| 2 | `pcre2_code_copy` | `if (newcode == NULL) [c_src/src/pcre2_compile.c:1139]` | [x] `return NULL` |
| 3 | `pcre2_code_copy_with_tables` | `if (code == NULL) [c_src/src/pcre2_compile.c:1172]` | [x] `return NULL` |
| 4 | `pcre2_code_copy_with_tables` | `if (newcode == NULL) [c_src/src/pcre2_compile.c:1174]` | [x] `return NULL` |
| 5 | `pcre2_code_copy_with_tables` | `if (newtables == NULL) { code->memctl.free((void *)newcode, code->memctl.memory_data); [c_src/src/pcre2_compile.c:1183]` | [x] `return NULL` |
| 6 | `read_number` | `if (n == 0) [c_src/src/pcre2_compile.c:1303]` | [x] `set *errorcodeptr to ERR26` |
| 7 | `read_number` | `else if (n > (uint32_t)allow_sign) [c_src/src/pcre2_compile.c:1310]` | [x] `set *errorcodeptr to ERR15` |
| 8 | `read_repeat_counts` | `if (max < min) [c_src/src/pcre2_compile.c:1433]` | [x] `set *errorcodeptr to ERR4` |
| 9 | `PRIV(check_escape)` | `if (ptr >= ptrend) [c_src/src/pcre2_compile.c:1506]` | [x] `set *errorcodeptr to ERR1` |
| 10 | `PRIV(check_escape)` | `if (ptr < ptrend && *ptr == CHAR_RIGHT_CURLY_BRACKET) ptr++; [c_src/src/pcre2_compile.c:1576]` | [x] `set *errorcodeptr to ERR93` |
| 11 | `PRIV(check_escape)` | `else if (isclass \|\| cb == NULL) { ptr++; /* Skip over the opening brace */ [c_src/src/pcre2_compile.c:1585]` | [x] `set *errorcodeptr to ERR37` |
| 12 | `PRIV(check_escape)` | `if (!read_repeat_counts(&p, ptrend, NULL, NULL, errorcodeptr) && { ptr++; /* Skip over the opening brace */ [c_src/src/pcre2_compile.c:1597]` | [x] `set *errorcodeptr to ERR37` |
| 13 | `PRIV(check_escape)` | `if (!(c >= CHAR_0 && c <= CHAR_9) && c != CHAR_c && c != CHAR_o && c != CHAR_x && c != CHAR_g) [c_src/src/pcre2_compile.c:1622]` | [x] `set *errorcodeptr to ERR3` |
| 14 | `PRIV(check_escape)` | `case CHAR_L: [c_src/src/pcre2_compile.c:1636]` | [x] `set *errorcodeptr to ERR37` |
| 15 | `PRIV(check_escape)` | `if (!alt_bsux) [c_src/src/pcre2_compile.c:1649]` | [x] `set *errorcodeptr to ERR37` |
| 16 | `PRIV(check_escape)` | `if ((cc & 0xf0000000) != 0) /* Test for 32-bit overflow */ [c_src/src/pcre2_compile.c:1665]` | [x] `set *errorcodeptr to ERR77` |
| 17 | `PRIV(check_escape)` | `if (c > 0x10ffffU) [c_src/src/pcre2_compile.c:1702]` | [x] `set *errorcodeptr to ERR77` |
| 18 | `PRIV(check_escape)` | `if (c >= 0xd800 && c <= 0xdfff && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0) [c_src/src/pcre2_compile.c:1706]` | [x] `set *errorcodeptr to ERR73` |
| 19 | `PRIV(check_escape)` | `else if (c > MAX_NON_UTF_CHAR) [c_src/src/pcre2_compile.c:1708]` | [x] `set *errorcodeptr to ERR77` |
| 20 | `PRIV(check_escape)` | `if (!alt_bsux) [c_src/src/pcre2_compile.c:1716]` | [x] `set *errorcodeptr to ERR37` |
| 21 | `PRIV(check_escape)` | `if (ptr >= ptrend) [c_src/src/pcre2_compile.c:1751]` | [x] `set *errorcodeptr to ERR57` |
| 22 | `PRIV(check_escape)` | `if (*ptr != CHAR_LESS_THAN_SIGN) [c_src/src/pcre2_compile.c:1761]` | [x] `set *errorcodeptr to ERR57` |
| 23 | `PRIV(check_escape)` | `if (p >= ptrend \|\| *p != CHAR_GREATER_THAN_SIGN) { ptr = p; [c_src/src/pcre2_compile.c:1777]` | [x] `set *errorcodeptr to ERR119` |
| 24 | `PRIV(check_escape)` | `if (p >= ptrend \|\| *p != CHAR_RIGHT_CURLY_BRACKET) { ptr = p; [c_src/src/pcre2_compile.c:1814]` | [x] `set *errorcodeptr to ERR119` |
| 25 | `PRIV(check_escape)` | `if ( [c_src/src/pcre2_compile.c:1827]` | [x] `set *errorcodeptr to ERR57` |
| 26 | `PRIV(check_escape)` | `if (s <= 0) [c_src/src/pcre2_compile.c:1834]` | [x] `set *errorcodeptr to ERR15` |
| 27 | `PRIV(check_escape)` | `if (!read_number(&ptr, ptrend, -1, MAX_GROUP_NUMBER, 0, &s, errorcodeptr)) [c_src/src/pcre2_compile.c:1887]` | [x] `set *errorcodeptr to ERR61` |
| 28 | `PRIV(check_escape)` | `if ((unsigned)s > MAX_GROUP_NUMBER) { PCRE2_ASSERT(s == INT_MAX); [c_src/src/pcre2_compile.c:1923]` | [x] `set *errorcodeptr to ERR61` |
| 29 | `PRIV(check_escape)` | `if ((xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0) [c_src/src/pcre2_compile.c:1953]` | [x] `set *errorcodeptr to ERR102` |
| 30 | `PRIV(check_escape)` | `else if (!utf) [c_src/src/pcre2_compile.c:1955]` | [x] `set *errorcodeptr to ERR51` |
| 31 | `PRIV(check_escape)` | `if ((xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1) [c_src/src/pcre2_compile.c:1963]` | [x] `set *errorcodeptr to ERR98` |
| 32 | `PRIV(check_escape)` | `if (ptr >= ptrend \|\| *ptr != CHAR_LEFT_CURLY_BRACKET) [c_src/src/pcre2_compile.c:1973]` | [x] `set *errorcodeptr to ERR55` |
| 33 | `PRIV(check_escape)` | `if (ptr >= ptrend \|\| *ptr == CHAR_RIGHT_CURLY_BRACKET) [c_src/src/pcre2_compile.c:1981]` | [x] `set *errorcodeptr to ERR78` |
| 34 | `PRIV(check_escape)` | `if (overflow) { while (ptr < ptrend && *ptr >= CHAR_0 && *ptr <= CHAR_7) ptr++; [c_src/src/pcre2_compile.c:2009]` | [x] `set *errorcodeptr to ERR34` |
| 35 | `PRIV(check_escape)` | `else if (utf && c >= 0xd800 && c <= 0xdfff && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0) [c_src/src/pcre2_compile.c:2014]` | [x] `set *errorcodeptr to ERR73` |
| 36 | `PRIV(check_escape)` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:2022]` | [x] `set *errorcodeptr to ERR64` |
| 37 | `PRIV(check_escape)` | `if (ptr >= ptrend \|\| *ptr == CHAR_RIGHT_CURLY_BRACKET) [c_src/src/pcre2_compile.c:2060]` | [x] `set *errorcodeptr to ERR78` |
| 38 | `PRIV(check_escape)` | `if (overflow) { while (ptr < ptrend && XDIGIT(*ptr) != 0xff) ptr++; [c_src/src/pcre2_compile.c:2090]` | [x] `set *errorcodeptr to ERR34` |
| 39 | `PRIV(check_escape)` | `else if (utf && c >= 0xd800 && c <= 0xdfff && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0) [c_src/src/pcre2_compile.c:2095]` | [x] `set *errorcodeptr to ERR73` |
| 40 | `PRIV(check_escape)` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:2109]` | [x] `set *errorcodeptr to ERR67` |
| 41 | `PRIV(check_escape)` | `if (ptr >= ptrend \|\| (cc = XDIGIT(*ptr)) == 0xff) [c_src/src/pcre2_compile.c:2126]` | [x] `set *errorcodeptr to ERR78` |
| 42 | `PRIV(check_escape)` | `if (ptr >= ptrend) [c_src/src/pcre2_compile.c:2165]` | [x] `set *errorcodeptr to ERR2` |
| 43 | `PRIV(check_escape)` | `if (c < 32 \|\| c > 126) /* Excludes all non-printable ASCII */ [c_src/src/pcre2_compile.c:2176]` | [x] `set *errorcodeptr to ERR68` |
| 44 | `PRIV(check_escape)` | `default: [c_src/src/pcre2_compile.c:2212]` | [x] `set *errorcodeptr to ERR3` |
| 45 | `get_ucp` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:2398]` | [x] `set *errorcodeptr to ERR47` |
| 46 | `get_ucp` | `if (r > 0) bot = i + 1; else top = i; } [c_src/src/pcre2_compile.c:2448]` | [x] `set *errorcodeptr to ERR47` |
| 47 | `get_ucp` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:2452]` | [x] `set *errorcodeptr to ERR46` |
| 48 | `read_name` | `if (type == ucp_Nd) { ptr = p; [c_src/src/pcre2_compile.c:2632]` | [x] `set *errorcodeptr to ERR44` |
| 49 | `read_name` | `if (is_group && IS_DIGIT(*ptr)) { ++ptr; [c_src/src/pcre2_compile.c:2659]` | [x] `set *errorcodeptr to ERR44` |
| 50 | `read_name` | `if (ptr - *nameptr > MAX_NAME_SIZE) [c_src/src/pcre2_compile.c:2673]` | [x] `set *errorcodeptr to ERR48` |
| 51 | `read_name` | `if (ptr == *nameptr) [c_src/src/pcre2_compile.c:2685]` | [x] `set *errorcodeptr to ERR62` |
| 52 | `read_name` | `if (ptr >= ptrend \|\| *ptr != (PCRE2_UCHAR)terminator) [c_src/src/pcre2_compile.c:2694]` | [x] `set *errorcodeptr to ERR42` |
| 53 | `parse_capture_list` | `if (ptr >= ptrend \|\| *ptr != CHAR_LEFT_PARENTHESIS) [c_src/src/pcre2_compile.c:2745]` | [x] `set *errorcodeptr to ERR118` |
| 54 | `parse_capture_list` | `if (ptr >= ptrend) [c_src/src/pcre2_compile.c:2756]` | [x] `set *errorcodeptr to ERR117` |
| 55 | `parse_capture_list` | `if (i <= 0) [c_src/src/pcre2_compile.c:2767]` | [x] `set *errorcodeptr to ERR15` |
| 56 | `parse_capture_list` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:2783]` | [x] `set *errorcodeptr to ERR117` |
| 57 | `parse_capture_list` | `if (*ptr != CHAR_COMMA) [c_src/src/pcre2_compile.c:2815]` | [x] `set *errorcodeptr to ERR24` |
| 58 | `parse_capture_list` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:2824]` | [x] `set *errorcodeptr to ERR14` |
| 59 | `parse_regex` | `if (parsed_pattern >= parsed_pattern_end) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:3193]` | [x] `set errorcode to ERR63` |
| 60 | `parse_regex` | `if (nest_depth > cb->cx->parens_nest_limit) [c_src/src/pcre2_compile.c:3240]` | [x] `set errorcode to ERR19` |
| 61 | `parse_regex` | `if (parsed_pattern >= parsed_pattern_end) { (but the code below can write many chars). Better than nothing. */ PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:3269]` | [x] `set errorcode to ERR63` |
| 62 | `parse_regex` | `if (ptr - verbnamestart - 1 > (int)MAX_MARK) { ptr--; [c_src/src/pcre2_compile.c:3367]` | [x] `set errorcode to ERR76` |
| 63 | `parse_regex` | `default: [c_src/src/pcre2_compile.c:3414]` | [x] `set errorcode to ERR40` |
| 64 | `parse_regex` | `if (expect_cond_assert > 0 && *ptr == CHAR_Q && !(ptrend - ptr >= 3 && ptr[1] == CHAR_BACKSLASH && ptr[2] == CHAR_E)) { ptr--; [c_src/src/pcre2_compile.c:3436]` | [x] `set errorcode to ERR28` |
| 65 | `parse_regex` | `if (ptr >= ptrend) [c_src/src/pcre2_compile.c:3485]` | [x] `set errorcode to ERR18` |
| 66 | `parse_regex` | `if (!ok) [c_src/src/pcre2_compile.c:3548]` | [x] `set errorcode to ERR28` |
| 67 | `parse_regex` | `if ((options & PCRE2_NEVER_BACKSLASH_C) != 0) [c_src/src/pcre2_compile.c:3666]` | [x] `set errorcode to ERR83` |
| 68 | `parse_regex` | `if (p >= ptrend \|\| *p != terminator) { ptr = p; [c_src/src/pcre2_compile.c:3767]` | [x] `set errorcode to ERR119` |
| 69 | `parse_regex` | `if (!prev_okquantifier) [c_src/src/pcre2_compile.c:3850]` | [x] `set errorcode to ERR9` |
| 70 | `parse_regex` | `if (class_mode_state == CLASS_MODE_PERL_EXT) [c_src/src/pcre2_compile.c:3993]` | [x] `set errorcode to ERR116` |
| 71 | `parse_regex` | `if (class_range_state == RANGE_STARTED) { ptr = tempptr + 2; [c_src/src/pcre2_compile.c:4032]` | [x] `set errorcode to ERR50` |
| 72 | `parse_regex` | `if (class_range_state == RANGE_FORBID_STARTED) { ptr = class_range_forbid_ptr; [c_src/src/pcre2_compile.c:4047]` | [x] `set errorcode to ERR50` |
| 73 | `parse_regex` | `if (class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT) { ptr = tempptr + 2; [c_src/src/pcre2_compile.c:4057]` | [x] `set errorcode to ERR113` |
| 74 | `parse_regex` | `if (*ptr != CHAR_COLON) { ptr = tempptr + 2; [c_src/src/pcre2_compile.c:4064]` | [x] `set errorcode to ERR13` |
| 75 | `parse_regex` | `if (posix_class < 0) [c_src/src/pcre2_compile.c:4078]` | [x] `set errorcode to ERR30` |
| 76 | `parse_regex` | `if (class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT) [c_src/src/pcre2_compile.c:4161]` | [x] `set errorcode to ERR113` |
| 77 | `parse_regex` | `if (class_depth_m1 >= ECLASS_NEST_LIMIT - 1) { ptr--; /* Point rightwards at the paren, same as ERR19. */ [c_src/src/pcre2_compile.c:4169]` | [x] `set errorcode to ERR107` |
| 78 | `parse_regex` | `if (start_c == CHAR_LEFT_PARENTHESIS) [c_src/src/pcre2_compile.c:4184]` | [x] `set errorcode to ERR14` |
| 79 | `parse_regex` | `if (start_c == CHAR_LEFT_PARENTHESIS) errorcode = ERR14; /* Missing terminating ')' */ else [c_src/src/pcre2_compile.c:4186]` | [x] `set errorcode to ERR6` |
| 80 | `parse_regex` | `if (c == CHAR_RIGHT_SQUARE_BRACKET && class_depth_m1 != 0) [c_src/src/pcre2_compile.c:4284]` | [x] `set errorcode to ERR14` |
| 81 | `parse_regex` | `if (c == CHAR_RIGHT_PARENTHESIS && class_depth_m1 < 1) [c_src/src/pcre2_compile.c:4290]` | [x] `set errorcode to ERR22` |
| 82 | `parse_regex` | `if (class_op_state == CLASS_OP_OPERATOR) [c_src/src/pcre2_compile.c:4298]` | [x] `set errorcode to ERR110` |
| 83 | `parse_regex` | `if (class_mode_state == CLASS_MODE_PERL_EXT && class_op_state == CLASS_OP_EMPTY) [c_src/src/pcre2_compile.c:4306]` | [x] `set errorcode to ERR114` |
| 84 | `parse_regex` | `if (ptr >= ptrend \|\| *ptr != CHAR_RIGHT_PARENTHESIS) [c_src/src/pcre2_compile.c:4324]` | [x] `set errorcode to ERR115` |
| 85 | `parse_regex` | `if (class_op_state != CLASS_OP_OPERAND) [c_src/src/pcre2_compile.c:4352]` | [x] `set errorcode to ERR109` |
| 86 | `parse_regex` | `if (class_op_state == CLASS_OP_OPERAND) [c_src/src/pcre2_compile.c:4385]` | [x] `set errorcode to ERR113` |
| 87 | `parse_regex` | `if (ptr < ptrend && *ptr == c) { while (ptr < ptrend && *ptr == c) ++ptr; /* Improve error offset. */ [c_src/src/pcre2_compile.c:4418]` | [x] `set errorcode to ERR108` |
| 88 | `parse_regex` | `if (class_op_state != CLASS_OP_OPERAND) [c_src/src/pcre2_compile.c:4425]` | [x] `set errorcode to ERR109` |
| 89 | `parse_regex` | `if (cb->class_op_used[class_depth_m1] != 0 && cb->class_op_used[class_depth_m1] != (uint8_t)c) [c_src/src/pcre2_compile.c:4433]` | [x] `set errorcode to ERR111` |
| 90 | `parse_regex` | `case ESC_X: [c_src/src/pcre2_compile.c:4505]` | [x] `set errorcode to ERR7` |
| 91 | `parse_regex` | `case ESC_N: /* Not permitted by Perl either */ [c_src/src/pcre2_compile.c:4509]` | [x] `set errorcode to ERR71` |
| 92 | `parse_regex` | `case ESC_C: [c_src/src/pcre2_compile.c:4578]` | [x] `set errorcode to ERR7` |
| 93 | `parse_regex` | `if (class_range_state == RANGE_STARTED) [c_src/src/pcre2_compile.c:4593]` | [x] `set errorcode to ERR50` |
| 94 | `parse_regex` | `if (class_range_state == RANGE_FORBID_STARTED) { ptr = class_range_forbid_ptr; [c_src/src/pcre2_compile.c:4603]` | [x] `set errorcode to ERR50` |
| 95 | `parse_regex` | `if (class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT) [c_src/src/pcre2_compile.c:4612]` | [x] `set errorcode to ERR113` |
| 96 | `parse_regex` | `else if (class_mode_state == CLASS_MODE_PERL_EXT) [c_src/src/pcre2_compile.c:4625]` | [x] `set errorcode to ERR116` |
| 97 | `parse_regex` | `if (class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT) [c_src/src/pcre2_compile.c:4658]` | [x] `set errorcode to ERR113` |
| 98 | `parse_regex` | `else if (parsed_pattern[-2] > c) /* Check range is in order */ [c_src/src/pcre2_compile.c:4668]` | [x] `set errorcode to ERR8` |
| 99 | `parse_regex` | `else if (class_range_state == RANGE_FORBID_STARTED) { ptr = class_range_forbid_ptr; [c_src/src/pcre2_compile.c:4683]` | [x] `set errorcode to ERR50` |
| 100 | `parse_regex` | `if (class_mode_state == CLASS_MODE_PERL_EXT && class_depth_m1 > 0) [c_src/src/pcre2_compile.c:4701]` | [x] `set errorcode to ERR14` |
| 101 | `parse_regex` | `if (class_mode_state == CLASS_MODE_ALT_EXT && class_depth_m1 == 0 && class_maxdepth_m1 == 1) [c_src/src/pcre2_compile.c:4704]` | [x] `set errorcode to ERR112` |
| 102 | `parse_regex` | `if (class_mode_state == CLASS_MODE_ALT_EXT && class_depth_m1 == 0 && class_maxdepth_m1 == 1) errorcode = ERR112; /* Missing terminating ']', but we saw '[ [ ]...' */ else [c_src/src/pcre2_compile.c:4706]` | [x] `set errorcode to ERR6` |
| 103 | `parse_regex` | `if (cb->bracount >= MAX_GROUP_NUMBER) [c_src/src/pcre2_compile.c:4737]` | [x] `set errorcode to ERR97` |
| 104 | `parse_regex` | `if (*ptr != CHAR_COLON) [c_src/src/pcre2_compile.c:4768]` | [x] `set errorcode to ERR95` |
| 105 | `parse_regex` | `if (i >= alascount) [c_src/src/pcre2_compile.c:4784]` | [x] `set errorcode to ERR95` |
| 106 | `parse_regex` | `if (prev_expect_cond_assert > 0 && (meta < META_LOOKAHEAD \|\| meta > META_LOOKBEHINDNOT)) [c_src/src/pcre2_compile.c:4795]` | [x] `set errorcode to ERR28` |
| 107 | `parse_regex` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:4807]` | [x] `set errorcode to ERR89` |
| 108 | `parse_regex` | `else if (++top_nest >= end_nests) [c_src/src/pcre2_compile.c:4856]` | [x] `set errorcode to ERR84` |
| 109 | `parse_regex` | `if (ptr >= ptrend \|\| (*ptr != CHAR_COLON && [c_src/src/pcre2_compile.c:4889]` | [x] `set errorcode to ERR60` |
| 110 | `parse_regex` | `if (i >= verbcount) [c_src/src/pcre2_compile.c:4905]` | [x] `set errorcode to ERR60` |
| 111 | `parse_regex` | `if (verbs[i].has_arg > 0 && *ptr != CHAR_COLON) [c_src/src/pcre2_compile.c:4919]` | [x] `set errorcode to ERR66` |
| 112 | `parse_regex` | `else if (++top_nest >= end_nests) [c_src/src/pcre2_compile.c:5000]` | [x] `set errorcode to ERR84` |
| 113 | `parse_regex` | `if (!hyphenok) [c_src/src/pcre2_compile.c:5055]` | [x] `set errorcode to ERR94` |
| 114 | `parse_regex` | `default: [c_src/src/pcre2_compile.c:5128]` | [x] `set errorcode to ERR11` |
| 115 | `parse_regex` | `if (*ptr != CHAR_EQUALS_SIGN) [c_src/src/pcre2_compile.c:5196]` | [x] `set errorcode to ERR41` |
| 116 | `parse_regex` | `if (ptr >= ptrend \|\| (*ptr != CHAR_RIGHT_PARENTHESIS && *ptr != CHAR_LEFT_PARENTHESIS)) [c_src/src/pcre2_compile.c:5215]` | [x] `set errorcode to ERR58` |
| 117 | `parse_regex` | `if (!IS_DIGIT(ptr[1])) [c_src/src/pcre2_compile.c:5232]` | [x] `set errorcode to ERR29` |
| 118 | `parse_regex` | `if ((xoptions & PCRE2_EXTRA_NEVER_CALLOUT) != 0) { ptr++; [c_src/src/pcre2_compile.c:5291]` | [x] `set errorcode to ERR103` |
| 119 | `parse_regex` | `if (delimiter == 0) [c_src/src/pcre2_compile.c:5342]` | [x] `set errorcode to ERR82` |
| 120 | `parse_regex` | `if (++ptr >= ptrend) [c_src/src/pcre2_compile.c:5353]` | [x] `set errorcode to ERR81` |
| 121 | `parse_regex` | `if (calloutlength > UINT32_MAX) [c_src/src/pcre2_compile.c:5364]` | [x] `set errorcode to ERR72` |
| 122 | `parse_regex` | `if (n > 255) [c_src/src/pcre2_compile.c:5385]` | [x] `set errorcode to ERR38` |
| 123 | `parse_regex` | `if (ptr >= ptrend \|\| *ptr != CHAR_RIGHT_PARENTHESIS) [c_src/src/pcre2_compile.c:5396]` | [x] `set errorcode to ERR39` |
| 124 | `parse_regex` | `if (i <= 0) [c_src/src/pcre2_compile.c:5456]` | [x] `set errorcode to ERR15` |
| 125 | `parse_regex` | `if (*ptr != CHAR_EQUALS_SIGN \|\| (ptr++, !IS_DIGIT(*ptr))) [c_src/src/pcre2_compile.c:5488]` | [x] `set errorcode to ERR79` |
| 126 | `parse_regex` | `if (++ptr >= ptrend \|\| !IS_DIGIT(*ptr)) [c_src/src/pcre2_compile.c:5500]` | [x] `set errorcode to ERR79` |
| 127 | `parse_regex` | `if (ptr >= ptrend \|\| *ptr != CHAR_RIGHT_PARENTHESIS) [c_src/src/pcre2_compile.c:5509]` | [x] `set errorcode to ERR79` |
| 128 | `parse_regex` | `if (ptr >= ptrend \|\| *ptr != CHAR_RIGHT_PARENTHESIS) [c_src/src/pcre2_compile.c:5590]` | [x] `set errorcode to ERR24` |
| 129 | `parse_regex` | `else if (++top_nest >= end_nests) [c_src/src/pcre2_compile.c:5669]` | [x] `set errorcode to ERR84` |
| 130 | `parse_regex` | `if (cb->bracount >= MAX_GROUP_NUMBER) [c_src/src/pcre2_compile.c:5698]` | [x] `set errorcode to ERR97` |
| 131 | `parse_regex` | `if (cb->names_found >= MAX_NAME_COUNT) [c_src/src/pcre2_compile.c:5709]` | [x] `set errorcode to ERR49` |
| 132 | `parse_regex` | `if ((options & PCRE2_DUPNAMES) == 0) [c_src/src/pcre2_compile.c:5738]` | [x] `set errorcode to ERR43` |
| 133 | `parse_regex` | `else if (ng->number == cb->bracount) [c_src/src/pcre2_compile.c:5759]` | [x] `set errorcode to ERR65` |
| 134 | `parse_regex` | `if (newspace == NULL) [c_src/src/pcre2_compile.c:5777]` | [x] `set errorcode to ERR21` |
| 135 | `parse_regex` | `if (nest_depth == 0) /* Unmatched closing parenthesis */ [c_src/src/pcre2_compile.c:5862]` | [x] `set errorcode to ERR22` |
| 136 | `parse_regex` | `if (inverbname && ptr >= ptrend) [c_src/src/pcre2_compile.c:5875]` | [x] `set errorcode to ERR60` |
| 137 | `parse_regex` | `if (parsed_pattern >= parsed_pattern_end) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:5912]` | [x] `set errorcode to ERR63` |
| 138 | `parse_regex` | `if (nest_depth == 0) return 0; UNCLOSED_PARENTHESIS: [c_src/src/pcre2_compile.c:5921]` | [x] `set errorcode to ERR14` |
| 139 | `compile_branch` | `if (code >= cb->start_workspace + cb->workspace_size) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:6170]` | [x] `set *errorcodeptr to ERR52` |
| 140 | `compile_branch` | `if (code > cb->start_workspace + cb->workspace_size - WORK_SIZE_SAFETY_MARGIN) /* Check for overrun */ [c_src/src/pcre2_compile.c:6179]` | [x] `set *errorcodeptr to ERR86` |
| 141 | `compile_branch` | `if (OFLOW_MAX - *lengthptr < (PCRE2_SIZE)(code - orig_code)) [c_src/src/pcre2_compile.c:6200]` | [x] `set *errorcodeptr to ERR20` |
| 142 | `compile_branch` | `if (*lengthptr > MAX_PATTERN_SIZE) [c_src/src/pcre2_compile.c:6207]` | [x] `set *errorcodeptr to ERR20` |
| 143 | `compile_branch` | `if (groupnumber > MAX_GROUP_NUMBER) [c_src/src/pcre2_compile.c:6691]` | [x] `set *errorcodeptr to ERR61` |
| 144 | `compile_branch` | `if (meta != META_COND_RNUMBER \|\| groupnumber > cb->bracount) [c_src/src/pcre2_compile.c:6700]` | [x] `set *errorcodeptr to ERR15` |
| 145 | `compile_branch` | `if (groupnumber > cb->bracount) [c_src/src/pcre2_compile.c:6814]` | [x] `set *errorcodeptr to ERR15` |
| 146 | `compile_branch` | `if (condcount > 1) { cb->erroroffset = offset; [c_src/src/pcre2_compile.c:6991]` | [x] `set *errorcodeptr to ERR54` |
| 147 | `compile_branch` | `if (condcount > 2) { cb->erroroffset = offset; [c_src/src/pcre2_compile.c:7008]` | [x] `set *errorcodeptr to ERR27` |
| 148 | `compile_branch` | `if (OFLOW_MAX - *lengthptr < length_prevgroup - 2 - 2*LINK_SIZE) [c_src/src/pcre2_compile.c:7025]` | [x] `set *errorcodeptr to ERR20` |
| 149 | `compile_branch` | `if (ng == NULL) [c_src/src/pcre2_compile.c:7143]` | [x] `set *errorcodeptr to ERR15` |
| 150 | `compile_branch` | `if (PRIV(ckd_smul)(&delta, replicate, (int)length_prevgroup) \|\| OFLOW_MAX - *lengthptr < delta) [c_src/src/pcre2_compile.c:7480]` | [x] `set *errorcodeptr to ERR20` |
| 151 | `compile_branch` | `if (PRIV(ckd_smul)(&delta, repeat_min - 1, (int)length_prevgroup) \|\| OFLOW_MAX - *lengthptr < delta) [c_src/src/pcre2_compile.c:7651]` | [x] `set *errorcodeptr to ERR20` |
| 152 | `compile_branch` | `if (PRIV(ckd_smul)(&delta, repeat_max, (int)length_prevgroup + 1 + 2 + 2*LINK_SIZE) \|\| OFLOW_MAX + (2 + 2*LINK_SIZE) - *lengthptr < delta) [c_src/src/pcre2_compile.c:7701]` | [x] `set *errorcodeptr to ERR20` |
| 153 | `compile_branch` | `if (op_previous >= OP_EODN \|\| op_previous <= OP_WORD_BOUNDARY) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:7860]` | [x] `set *errorcodeptr to ERR10` |
| 154 | `compile_branch` | `if (meta_arg > cb->bracount) { cb->erroroffset = offset; [c_src/src/pcre2_compile.c:8143]` | [x] `set *errorcodeptr to ERR15` |
| 155 | `compile_branch` | `if (meta_arg > cb->bracount) { cb->erroroffset = offset; [c_src/src/pcre2_compile.c:8187]` | [x] `set *errorcodeptr to ERR15` |
| 156 | `compile_branch` | `if (cb->assert_depth > 0 && meta_arg == ESC_K && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) == 0) [c_src/src/pcre2_compile.c:8341]` | [x] `set *errorcodeptr to ERR99` |
| 157 | `compile_branch` | `if (meta >= META_END) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:8399]` | [x] `set *errorcodeptr to ERR89` |
| 158 | `compile_regex` | `if (cb->cx->stack_guard != NULL && cb->cx->stack_guard(cb->parens_depth, cb->cx->stack_guard_data)) [c_src/src/pcre2_compile.c:8601]` | [x] `set *errorcodeptr to ERR33` |
| 159 | `compile_regex` | `if (OFLOW_MAX - *lengthptr < length) [c_src/src/pcre2_compile.c:8807]` | [x] `set *errorcodeptr to ERR20` |
| 160 | `pcre2_compile` | `if (erroroffset != NULL) *erroroffset = 0; [c_src/src/pcre2_compile.c:10343]` | [x] `return NULL` |
| 161 | `pcre2_compile` | `if (errorptr != NULL) *errorptr = ERR120; [c_src/src/pcre2_compile.c:10348]` | [x] `return NULL` |
| 162 | `pcre2_compile` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_compile.c:10362]` | [x] `return NULL` |
| 163 | `pcre2_compile` | `if ((options & ~PUBLIC_COMPILE_OPTIONS) != 0 \|\| (ccontext->extra_options & ~PUBLIC_COMPILE_EXTRA_OPTIONS) != 0) [c_src/src/pcre2_compile.c:10381]` | [x] `return NULL` |
| 164 | `pcre2_compile` | `if ((options & PCRE2_LITERAL) != 0 && ((options & ~PUBLIC_LITERAL_COMPILE_OPTIONS) != 0 \|\| (ccontext->extra_options & ~PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS) != 0)) [c_src/src/pcre2_compile.c:10389]` | [x] `return NULL` |
| 165 | `pcre2_compile` | `if (patlen > ccontext->max_pattern_length) [c_src/src/pcre2_compile.c:10402]` | [x] `return NULL` |
| 166 | `pcre2_compile` | `if (pp >= patlen \|\| pp == skipatstart \|\| ptr[pp] != CHAR_RIGHT_PARENTHESIS) [c_src/src/pcre2_compile.c:10549]` | [x] `set errorcode to ERR60` |
| 167 | `pcre2_compile` | `if ((options & PCRE2_NEVER_UTF) != 0) [c_src/src/pcre2_compile.c:10624]` | [x] `set errorcode to ERR74` |
| 168 | `pcre2_compile` | `if (ucp && (cb.external_options & PCRE2_NEVER_UCP) != 0) [c_src/src/pcre2_compile.c:10645]` | [x] `set errorcode to ERR75` |
| 169 | `pcre2_compile` | `if (!utf && !ucp) [c_src/src/pcre2_compile.c:10655]` | [x] `set errorcode to ERR104` |
| 170 | `pcre2_compile` | `if (!utf) [c_src/src/pcre2_compile.c:10662]` | [x] `set errorcode to ERR105` |
| 171 | `pcre2_compile` | `if ((xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0) [c_src/src/pcre2_compile.c:10669]` | [x] `set errorcode to ERR106` |
| 172 | `pcre2_compile` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:10716]` | [x] `set errorcode to ERR56` |
| 173 | `pcre2_compile` | `if (cb.groupinfo == NULL) [c_src/src/pcre2_compile.c:10783]` | [x] `set errorcode to ERR21` |
| 174 | `pcre2_compile` | `if (length > MAX_PATTERN_SIZE \|\| MAX_PATTERN_SIZE - length < (cb.char_lists_size / sizeof(PCRE2_UCHAR))) [c_src/src/pcre2_compile.c:10846]` | [x] `set errorcode to ERR20` |
| 175 | `pcre2_compile` | `if (re_blocksize > ccontext->max_pattern_compiled_length) [c_src/src/pcre2_compile.c:10875]` | [x] `set errorcode to ERR101` |
| 176 | `pcre2_compile` | `if (re == NULL) [c_src/src/pcre2_compile.c:10885]` | [x] `set errorcode to ERR21` |
| 177 | `pcre2_compile` | `if (usedlength > length) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:10995]` | [x] `set errorcode to ERR23` |
| 178 | `pcre2_compile` | `if (rgroup == NULL) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:11053]` | [x] `set errorcode to ERR53` |
| 179 | `pcre2_compile` | `if (possessify_rc != 0) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:11092]` | [x] `set errorcode to ERR80` |
| 180 | `pcre2_compile` | `if (study_rc != 0) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile.c:11254]` | [x] `set errorcode to ERR31` |
| 181 | `PRIV(compile_find_dupname_details)` | `if (i >= cb->names_found) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_compile_cgroup.c:235]` | [x] `set *errorcodeptr to ERR53` |
| 182 | `PRIV(compile_process_capture_list)` | `if (ng == NULL) [c_src/src/pcre2_compile_cgroup.c:297]` | [x] `set *errorcodeptr to ERR15` |
| 183 | `PRIV(compile_process_capture_list)` | `if (i > cb->bracount) [c_src/src/pcre2_compile_cgroup.c:326]` | [x] `set *errorcodeptr to ERR15` |
| 184 | `PRIV(compile_parse_scan_substr_args)` | `if (captures == NULL) [c_src/src/pcre2_compile_cgroup.c:384]` | [x] `set *errorcodeptr to ERR21` |
| 185 | `PRIV(compile_parse_recurse_args)` | `if (args == NULL) [c_src/src/pcre2_compile_cgroup.c:531]` | [x] `set *errorcodeptr to ERR21` |
| 186 | `PRIV(compile_class_not_nested)` | `if (cranges == NULL) [c_src/src/pcre2_compile_class.c:1127]` | [x] `set *errorcodeptr to ERR21` |
| 187 | `PRIV(compile_class_not_nested)` | `if (*lengthptr > MAX_PATTERN_SIZE \|\| MAX_PATTERN_SIZE - *lengthptr < char_lists_size) [c_src/src/pcre2_compile_class.c:1771]` | [x] `set *errorcodeptr to ERR20` |
| 188 | `pcre2_config` | `default: [c_src/src/pcre2_config.c:78]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 189 | `pcre2_config` | `default: [c_src/src/pcre2_config.c:108]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 190 | `pcre2_config` | `case PCRE2_CONFIG_JITTARGET: [c_src/src/pcre2_config.c:160]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 191 | `PRIV(memctl_malloc)` | `if (yield == NULL) [c_src/src/pcre2_context.c:87]` | [x] `return NULL` |
| 192 | `pcre2_general_context_create` | `if (gcontext == NULL) [c_src/src/pcre2_context.c:118]` | [x] `return NULL` |
| 193 | `pcre2_compile_context_create` | `if (ccontext == NULL) [c_src/src/pcre2_context.c:152]` | [x] `return NULL` |
| 194 | `pcre2_match_context_create` | `if (mcontext == NULL) [c_src/src/pcre2_context.c:188]` | [x] `return NULL` |
| 195 | `pcre2_convert_context_create` | `if (ccontext == NULL) [c_src/src/pcre2_context.c:218]` | [x] `return NULL` |
| 196 | `pcre2_general_context_copy` | `if (newcontext == NULL) [c_src/src/pcre2_context.c:236]` | [x] `return NULL` |
| 197 | `pcre2_compile_context_copy` | `if (newcontext == NULL) [c_src/src/pcre2_context.c:248]` | [x] `return NULL` |
| 198 | `pcre2_match_context_copy` | `if (newcontext == NULL) [c_src/src/pcre2_context.c:260]` | [x] `return NULL` |
| 199 | `pcre2_convert_context_copy` | `if (newcontext == NULL) [c_src/src/pcre2_context.c:272]` | [x] `return NULL` |
| 200 | `pcre2_set_bsr` | `default: [c_src/src/pcre2_context.c:344]` | [x] `return PCRE2_ERROR_BADDATA` |
| 201 | `pcre2_set_newline` | `default: [c_src/src/pcre2_context.c:377]` | [x] `return PCRE2_ERROR_BADDATA` |
| 202 | `pcre2_set_optimize` | `if (ccontext == NULL) [c_src/src/pcre2_context.c:415]` | [x] `return PCRE2_ERROR_NULL` |
| 203 | `pcre2_set_optimize` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_context.c:438]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 204 | `pcre2_set_glob_separator` | `if (separator != CHAR_SLASH && separator != CHAR_BACKSLASH && [c_src/src/pcre2_context.c:532]` | [x] `return PCRE2_ERROR_BADDATA` |
| 205 | `pcre2_set_glob_escape` | `if (escape > 255 \|\| (escape != 0 && strchr(globpunct, escape) == NULL)) [c_src/src/pcre2_context.c:551]` | [x] `return PCRE2_ERROR_BADDATA` |
| 206 | `(file scope)` | `if (p >= endp) [c_src/src/pcre2_convert.c:73]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 207 | `convert_posix` | `if (p + clength > endp) [c_src/src/pcre2_convert.c:242]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 208 | `convert_posix` | `if (plength == 0) [c_src/src/pcre2_convert.c:303]` | [x] `return PCRE2_ERROR_END_BACKSLASH` |
| 209 | `convert_posix` | `if (p + 1 > endp) [c_src/src/pcre2_convert.c:309]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 210 | `convert_posix` | `if (p + 1 > endp) [c_src/src/pcre2_convert.c:339]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 211 | `convert_posix` | `if (p + clength > endp) [c_src/src/pcre2_convert.c:370]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 212 | `convert_posix` | `if (posix_state >= POSIX_CLASS_NOT_STARTED) [c_src/src/pcre2_convert.c:379]` | [x] `return PCRE2_ERROR_MISSING_SQUARE_BRACKET` |
| 213 | `convert_glob_parse_range` | `if (pattern >= pattern_end) [c_src/src/pcre2_convert.c:654]` | [x] `return PCRE2_ERROR_MISSING_SQUARE_BRACKET` |
| 214 | `convert_glob_parse_range` | `if (pattern >= pattern_end) [c_src/src/pcre2_convert.c:665]` | [x] `return PCRE2_ERROR_MISSING_SQUARE_BRACKET` |
| 215 | `convert_glob_parse_range` | `else if (c == CHAR_LEFT_SQUARE_BRACKET && *pattern == CHAR_COLON) [c_src/src/pcre2_convert.c:765]` | [x] `return PCRE2_ERROR_CONVERT_SYNTAX` |
| 216 | `convert_glob_parse_range` | `if (prev_c > c) [c_src/src/pcre2_convert.c:771]` | [x] `return PCRE2_ERROR_CONVERT_SYNTAX` |
| 217 | `convert_glob_parse_range` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_convert.c:803]` | [x] `return PCRE2_ERROR_MISSING_SQUARE_BRACKET` |
| 218 | `convert_glob` | `if (utf && (separator >= 128 \|\| escape >= 128)) [c_src/src/pcre2_convert.c:872]` | [x] `return PCRE2_ERROR_CONVERT_SYNTAX` |
| 219 | `pcre2_pattern_convert` | `if (bufflenptr != NULL) *bufflenptr = 0; /* Error offset */ [c_src/src/pcre2_convert.c:1135]` | [x] `return PCRE2_ERROR_NULL` |
| 220 | `pcre2_pattern_convert` | `if ((options & ~ALL_OPTIONS) != 0 \|\| /* Undefined bit set */ (pattype & (~pattype+1)) != pattype \|\| /* More than one type set */ pattype == 0) /* No type set */ [c_src/src/pcre2_convert.c:1143]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 221 | `pcre2_pattern_convert` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_convert.c:1206]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 222 | `pcre2_pattern_convert` | `if (allocated == NULL) [c_src/src/pcre2_convert.c:1223]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 223 | `pcre2_pattern_convert` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_convert.c:1235]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 224 | `more_workspace` | `if (newsize < RWS_RSIZE + ovecsize + RWS_ANCHOR_SIZE) [c_src/src/pcre2_dfa_match.c:445]` | [x] `return PCRE2_ERROR_HEAPLIMIT` |
| 225 | `more_workspace` | `if (new == NULL) [c_src/src/pcre2_dfa_match.c:447]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 226 | `internal_dfa_match` | `if (mb->match_call_count++ >= mb->match_limit) [c_src/src/pcre2_dfa_match.c:566]` | [x] `return PCRE2_ERROR_MATCHLIMIT` |
| 227 | `internal_dfa_match` | `if (rlevel++ > mb->match_limit_depth) [c_src/src/pcre2_dfa_match.c:567]` | [x] `return PCRE2_ERROR_DEPTHLIMIT` |
| 228 | `internal_dfa_match` | `case OP_ANYBYTE: [c_src/src/pcre2_dfa_match.c:825]` | [x] `return PCRE2_ERROR_DFA_UITEM` |
| 229 | `internal_dfa_match` | `if ((mb->moptions & PCRE2_PARTIAL_HARD) != 0) [c_src/src/pcre2_dfa_match.c:964]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 230 | `internal_dfa_match` | `if ((mb->moptions & PCRE2_PARTIAL_HARD) != 0) [c_src/src/pcre2_dfa_match.c:1016]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 231 | `internal_dfa_match` | `if (condcode == OP_CREF \|\| condcode == OP_DNCREF \|\| condcode == OP_DNRREF) [c_src/src/pcre2_dfa_match.c:2857]` | [x] `return PCRE2_ERROR_DFA_UCOND` |
| 232 | `internal_dfa_match` | `if (value != RREF_ANY) [c_src/src/pcre2_dfa_match.c:2877]` | [x] `return PCRE2_ERROR_DFA_UCOND` |
| 233 | `internal_dfa_match` | `if (code[1 + LINK_SIZE] == OP_CREF) [c_src/src/pcre2_dfa_match.c:2943]` | [x] `return PCRE2_ERROR_DFA_UITEM` |
| 234 | `internal_dfa_match` | `if (recno == ri->group_num && ptr == ri->subject_position && mb->last_used_ptr == ri->last_used_ptr) [c_src/src/pcre2_dfa_match.c:2966]` | [x] `return PCRE2_ERROR_RECURSELOOP` |
| 235 | `internal_dfa_match` | `if ( [c_src/src/pcre2_dfa_match.c:2995]` | [x] `return PCRE2_ERROR_DFA_RECURSE` |
| 236 | `internal_dfa_match` | `default: /* Unsupported opcode */ [c_src/src/pcre2_dfa_match.c:3259]` | [x] `return PCRE2_ERROR_DFA_UITEM` |
| 237 | `pcre2_dfa_match` | `if (match_data == NULL) [c_src/src/pcre2_dfa_match.c:3396]` | [x] `return PCRE2_ERROR_NULL` |
| 238 | `pcre2_dfa_match` | `if (re == NULL \|\| subject == NULL \|\| workspace == NULL) [c_src/src/pcre2_dfa_match.c:3398]` | [x] `set/return rc as PCRE2_ERROR_NULL` |
| 239 | `pcre2_dfa_match` | `if ((options & ~PUBLIC_DFA_MATCH_OPTIONS) != 0) [c_src/src/pcre2_dfa_match.c:3400]` | [x] `set/return rc as PCRE2_ERROR_BADOPTION` |
| 240 | `pcre2_dfa_match` | `if (wscount < 20) [c_src/src/pcre2_dfa_match.c:3407]` | [x] `set/return rc as PCRE2_ERROR_DFA_WSSIZE` |
| 241 | `pcre2_dfa_match` | `if (start_offset > length) [c_src/src/pcre2_dfa_match.c:3408]` | [x] `set/return rc as PCRE2_ERROR_BADOFFSET` |
| 242 | `pcre2_dfa_match` | `if ((options & (PCRE2_PARTIAL_HARD\|PCRE2_PARTIAL_SOFT)) != 0 && ((re->overall_options \| options) & PCRE2_ENDANCHORED) != 0) [c_src/src/pcre2_dfa_match.c:3415]` | [x] `set/return rc as PCRE2_ERROR_BADOPTION` |
| 243 | `pcre2_dfa_match` | `if ((re->overall_options & PCRE2_MATCH_INVALID_UTF) != 0) [c_src/src/pcre2_dfa_match.c:3420]` | [x] `set/return rc as PCRE2_ERROR_DFA_UINVALID_UTF` |
| 244 | `pcre2_dfa_match` | `if (re->magic_number != MAGIC_NUMBER) [c_src/src/pcre2_dfa_match.c:3426]` | [x] `set/return rc as PCRE2_ERROR_BADMAGIC` |
| 245 | `pcre2_dfa_match` | `if ((re->flags & PCRE2_MODE_MASK) != PCRE2_CODE_UNIT_WIDTH/8) [c_src/src/pcre2_dfa_match.c:3431]` | [x] `set/return rc as PCRE2_ERROR_BADMODE` |
| 246 | `pcre2_dfa_match` | `if ((workspace[0] & (-2)) != 0 \|\| workspace[1] < 1 \|\| workspace[1] > (int)((wscount - 2)/INTS_PER_STATEBLOCK)) [c_src/src/pcre2_dfa_match.c:3458]` | [x] `set/return rc as PCRE2_ERROR_DFA_BADRESTART` |
| 247 | `pcre2_dfa_match` | `if ((re->overall_options & PCRE2_USE_OFFSET_LIMIT) == 0) [c_src/src/pcre2_dfa_match.c:3506]` | [x] `set/return rc as PCRE2_ERROR_BADOFFSETLIMIT` |
| 248 | `pcre2_dfa_match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_dfa_match.c:3576]` | [x] `set/return rc as PCRE2_ERROR_INTERNAL` |
| 249 | `pcre2_dfa_match` | `if (start_match < end_subject && NOT_FIRSTCU(*start_match)) [c_src/src/pcre2_dfa_match.c:3599]` | [x] `set/return rc as PCRE2_ERROR_BADUTFOFFSET` |
| 250 | `pcre2_dfa_match` | `if (match_data->subject == NULL) [c_src/src/pcre2_dfa_match.c:4068]` | [x] `set/return rc as PCRE2_ERROR_NOMEMORY` |
| 251 | `pcre2_dfa_match` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_dfa_match.c:4114]` | [x] `set/return rc as PCRE2_ERROR_NOMATCH` |
| 252 | `pcre2_get_error_message` | `if (size == 0) [c_src/src/pcre2_error.c:339]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 253 | `pcre2_get_error_message` | `if (*message == CHAR_NUL) [c_src/src/pcre2_error.c:360]` | [x] `return PCRE2_ERROR_BADDATA` |
| 254 | `pcre2_get_error_message` | `if (i >= size - 1) [c_src/src/pcre2_error.c:367]` | [x] `set/return rc as PCRE2_ERROR_NOMEMORY` |
| 255 | `pcre2_jit_compile` | `if (options != PCRE2_JIT_TEST_ALLOC) [c_src/src/pcre2_jit_compile.c:14319]` | [x] `return PCRE2_ERROR_JIT_BADOPTION` |
| 256 | `pcre2_jit_compile` | `if (options != PCRE2_JIT_TEST_ALLOC) return PCRE2_ERROR_JIT_BADOPTION; [c_src/src/pcre2_jit_compile.c:14324]` | [x] `return PCRE2_ERROR_JIT_UNSUPPORTED` |
| 257 | `pcre2_jit_compile` | `if (code == NULL) [c_src/src/pcre2_jit_compile.c:14329]` | [x] `return PCRE2_ERROR_NULL` |
| 258 | `pcre2_jit_compile` | `if ((options & ~PUBLIC_JIT_COMPILE_OPTIONS) != 0) [c_src/src/pcre2_jit_compile.c:14332]` | [x] `return PCRE2_ERROR_JIT_BADOPTION` |
| 259 | `pcre2_jit_compile` | `interpreter support) even in the absence of JIT. But now, if there is no JIT support, give an error return. */ [c_src/src/pcre2_jit_compile.c:14381]` | [x] `return PCRE2_ERROR_JIT_BADOPTION` |
| 260 | `(file scope)` | `if (mb->partial != 0 && (Feptr > mb->start_used_ptr \|\| mb->allowemptypartial)) { mb->hitend = TRUE; if (mb->partial > 1) [c_src/src/pcre2_match.c:623]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 261 | `match` | `if (match_data->heapframes_size == PCRE2_SIZE_MAX - 1) [c_src/src/pcre2_match.c:768]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 262 | `match` | `if (mb->heap_limit <= old_size) [c_src/src/pcre2_match.c:778]` | [x] `return PCRE2_ERROR_HEAPLIMIT` |
| 263 | `match` | `if (newsize - usedsize < frame_size) [c_src/src/pcre2_match.c:791]` | [x] `return PCRE2_ERROR_HEAPLIMIT` |
| 264 | `match` | `if (new == NULL) [c_src/src/pcre2_match.c:793]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 265 | `match` | `if (mb->match_call_count++ >= mb->match_limit) [c_src/src/pcre2_match.c:873]` | [x] `return PCRE2_ERROR_MATCHLIMIT` |
| 266 | `match` | `if (Frdepth >= mb->match_limit_depth) [c_src/src/pcre2_match.c:874]` | [x] `return PCRE2_ERROR_DEPTHLIMIT` |
| 267 | `match` | `if (offset == PCRE2_UNSET) [c_src/src/pcre2_match.c:909]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 268 | `match` | `if (offset == PCRE2_UNSET) [c_src/src/pcre2_match.c:951]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 269 | `match` | `if (!mb->allowlookaroundbsk) [c_src/src/pcre2_match.c:1030]` | [x] `return PCRE2_ERROR_BAD_BACKSLASH_K` |
| 270 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:1070]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 271 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:2876]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 272 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:3229]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 273 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:3279]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 274 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:3507]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 275 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:3535]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 276 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:3762]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 277 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:4051]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 278 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:4110]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 279 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:4208]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 280 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:4241]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 281 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:4355]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 282 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:4626]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 283 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:4740]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 284 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:4947]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 285 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:4992]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 286 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:5207]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 287 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:5401]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 288 | `match` | `if (Feptr == P->eptr && mb->last_used_ptr == P->recurse_last_used && (mb->moptions & PCRE2_DISABLE_RECURSELOOP_CHECK) == 0) [c_src/src/pcre2_match.c:5729]` | [x] `return PCRE2_ERROR_RECURSELOOP` |
| 289 | `match` | `if (offset == PCRE2_UNSET) [c_src/src/pcre2_match.c:6377]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 290 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:6596]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 291 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:6615]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 292 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:6625]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 293 | `match` | `if (mb->partial > 1) [c_src/src/pcre2_match.c:6663]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 294 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:6889]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 295 | `match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:6941]` | [x] `return PCRE2_ERROR_INTERNAL` |
| 296 | `pcre2_match` | `if (match_data == NULL) [c_src/src/pcre2_match.c:7042]` | [x] `return PCRE2_ERROR_NULL` |
| 297 | `pcre2_match` | `if (code == NULL \|\| subject == NULL) [c_src/src/pcre2_match.c:7044]` | [x] `return match_data->rc = PCRE2_ERROR_NULL` |
| 298 | `pcre2_match` | `if ((options & ~PUBLIC_MATCH_OPTIONS) != 0) [c_src/src/pcre2_match.c:7046]` | [x] `return match_data->rc = PCRE2_ERROR_BADOPTION` |
| 299 | `pcre2_match` | `if (start_offset > length) [c_src/src/pcre2_match.c:7056]` | [x] `return match_data->rc = PCRE2_ERROR_BADOFFSET` |
| 300 | `pcre2_match` | `if (re->magic_number != MAGIC_NUMBER) [c_src/src/pcre2_match.c:7061]` | [x] `return match_data->rc = PCRE2_ERROR_BADMAGIC` |
| 301 | `pcre2_match` | `if ((re->flags & PCRE2_MODE_MASK) != PCRE2_CODE_UNIT_WIDTH/8) [c_src/src/pcre2_match.c:7066]` | [x] `return match_data->rc = PCRE2_ERROR_BADMODE` |
| 302 | `pcre2_match` | `if (mb->partial != 0 && ((re->overall_options \| options) & PCRE2_ENDANCHORED) != 0) [c_src/src/pcre2_match.c:7113]` | [x] `return match_data->rc = PCRE2_ERROR_BADOPTION` |
| 303 | `pcre2_match` | `if (mcontext != NULL && mcontext->offset_limit != PCRE2_UNSET && (re->overall_options & PCRE2_USE_OFFSET_LIMIT) == 0) [c_src/src/pcre2_match.c:7120]` | [x] `return match_data->rc = PCRE2_ERROR_BADOFFSETLIMIT` |
| 304 | `pcre2_match` | `if (start_offset > 0) [c_src/src/pcre2_match.c:7295]` | [x] `return match_data->rc = PCRE2_ERROR_BADUTFOFFSET` |
| 305 | `pcre2_match` | `if (start_offset > 0) return match_data->rc = PCRE2_ERROR_BADUTFOFFSET; [c_src/src/pcre2_match.c:7297]` | [x] `return match_data->rc = PCRE2_ERROR_UTF8_ERR20` |
| 306 | `pcre2_match` | `default: PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_match.c:7478]` | [x] `return match_data->rc = PCRE2_ERROR_INTERNAL` |
| 307 | `pcre2_match` | `if (max_size < frame_size) [c_src/src/pcre2_match.c:7521]` | [x] `return match_data->rc = PCRE2_ERROR_HEAPLIMIT` |
| 308 | `pcre2_match` | `if (match_data->heapframes == NULL) { match_data->heapframes_size = 0; [c_src/src/pcre2_match.c:7537]` | [x] `return match_data->rc = PCRE2_ERROR_NOMEMORY` |
| 309 | `pcre2_match` | `if (match_data->subject == NULL) [c_src/src/pcre2_match.c:8195]` | [x] `return match_data->rc = PCRE2_ERROR_NOMEMORY` |
| 310 | `pcre2_match` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_match.c:8232]` | [x] `set/return rc as PCRE2_ERROR_PARTIAL` |
| 311 | `pcre2_match` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_match.c:8242]` | [x] `set/return rc as PCRE2_ERROR_NOMATCH` |
| 312 | `pcre2_match_data_create` | `if (yield == NULL) [c_src/src/pcre2_match_data.c:62]` | [x] `return NULL` |
| 313 | `pcre2_match_data_create_from_pattern` | `if (code == NULL) [c_src/src/pcre2_match_data.c:84]` | [x] `return NULL` |
| 314 | `pcre2_pattern_info` | `if (re == NULL) [c_src/src/pcre2_pattern_info.c:107]` | [x] `return PCRE2_ERROR_NULL` |
| 315 | `pcre2_pattern_info` | `if (re->magic_number != MAGIC_NUMBER) [c_src/src/pcre2_pattern_info.c:112]` | [x] `return PCRE2_ERROR_BADMAGIC` |
| 316 | `pcre2_pattern_info` | `if ((re->flags & (PCRE2_CODE_UNIT_WIDTH/8)) == 0) [c_src/src/pcre2_pattern_info.c:116]` | [x] `return PCRE2_ERROR_BADMODE` |
| 317 | `pcre2_pattern_info` | `if (re->limit_depth == UINT32_MAX) [c_src/src/pcre2_pattern_info.c:142]` | [x] `return PCRE2_ERROR_UNSET` |
| 318 | `pcre2_pattern_info` | `if (re->limit_heap == UINT32_MAX) [c_src/src/pcre2_pattern_info.c:179]` | [x] `return PCRE2_ERROR_UNSET` |
| 319 | `pcre2_pattern_info` | `if (re->limit_match == UINT32_MAX) [c_src/src/pcre2_pattern_info.c:210]` | [x] `return PCRE2_ERROR_UNSET` |
| 320 | `pcre2_pattern_info` | `default: [c_src/src/pcre2_pattern_info.c:242]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 321 | `pcre2_callout_enumerate` | `if (re == NULL) [c_src/src/pcre2_pattern_info.c:276]` | [x] `return PCRE2_ERROR_NULL` |
| 322 | `pcre2_callout_enumerate` | `if (re->magic_number != MAGIC_NUMBER) [c_src/src/pcre2_pattern_info.c:285]` | [x] `return PCRE2_ERROR_BADMAGIC` |
| 323 | `pcre2_callout_enumerate` | `if ((re->flags & (PCRE2_CODE_UNIT_WIDTH/8)) == 0) [c_src/src/pcre2_pattern_info.c:289]` | [x] `return PCRE2_ERROR_BADMODE` |
| 324 | `pcre2_serialize_encode` | `if (codes == NULL \|\| serialized_bytes == NULL \|\| serialized_size == NULL) [c_src/src/pcre2_serialize.c:86]` | [x] `return PCRE2_ERROR_NULL` |
| 325 | `pcre2_serialize_encode` | `if (number_of_codes <= 0) [c_src/src/pcre2_serialize.c:88]` | [x] `return PCRE2_ERROR_BADDATA` |
| 326 | `pcre2_serialize_encode` | `if (codes[i] == NULL) [c_src/src/pcre2_serialize.c:96]` | [x] `return PCRE2_ERROR_NULL` |
| 327 | `pcre2_serialize_encode` | `if (re->magic_number != MAGIC_NUMBER) [c_src/src/pcre2_serialize.c:98]` | [x] `return PCRE2_ERROR_BADMAGIC` |
| 328 | `pcre2_serialize_encode` | `else if (tables != re->tables) [c_src/src/pcre2_serialize.c:102]` | [x] `return PCRE2_ERROR_MIXEDTABLES` |
| 329 | `pcre2_serialize_encode` | `if (bytes == NULL) [c_src/src/pcre2_serialize.c:108]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 330 | `pcre2_serialize_decode` | `if (data == NULL \|\| codes == NULL) [c_src/src/pcre2_serialize.c:176]` | [x] `return PCRE2_ERROR_NULL` |
| 331 | `pcre2_serialize_decode` | `if (number_of_codes <= 0) [c_src/src/pcre2_serialize.c:177]` | [x] `return PCRE2_ERROR_BADDATA` |
| 332 | `pcre2_serialize_decode` | `if (data->number_of_codes <= 0) [c_src/src/pcre2_serialize.c:178]` | [x] `return PCRE2_ERROR_BADSERIALIZEDDATA` |
| 333 | `pcre2_serialize_decode` | `if (data->magic != SERIALIZED_DATA_MAGIC) [c_src/src/pcre2_serialize.c:179]` | [x] `return PCRE2_ERROR_BADMAGIC` |
| 334 | `pcre2_serialize_decode` | `if (data->version != SERIALIZED_DATA_VERSION) [c_src/src/pcre2_serialize.c:180]` | [x] `return PCRE2_ERROR_BADMODE` |
| 335 | `pcre2_serialize_decode` | `if (data->config != SERIALIZED_DATA_CONFIG) [c_src/src/pcre2_serialize.c:181]` | [x] `return PCRE2_ERROR_BADMODE` |
| 336 | `pcre2_serialize_decode` | `if (tables == NULL) [c_src/src/pcre2_serialize.c:192]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 337 | `pcre2_serialize_get_number_of_codes` | `if (data == NULL) [c_src/src/pcre2_serialize.c:272]` | [x] `return PCRE2_ERROR_NULL` |
| 338 | `pcre2_serialize_get_number_of_codes` | `if (data->magic != SERIALIZED_DATA_MAGIC) [c_src/src/pcre2_serialize.c:273]` | [x] `return PCRE2_ERROR_BADMAGIC` |
| 339 | `pcre2_serialize_get_number_of_codes` | `if (data->version != SERIALIZED_DATA_VERSION) [c_src/src/pcre2_serialize.c:274]` | [x] `return PCRE2_ERROR_BADMODE` |
| 340 | `pcre2_serialize_get_number_of_codes` | `if (data->config != SERIALIZED_DATA_CONFIG) [c_src/src/pcre2_serialize.c:275]` | [x] `return PCRE2_ERROR_BADMODE` |
| 341 | `find_text_end` | `if (errorcode != 0) { pcre2_substitute(). */ [c_src/src/pcre2_substitute.c:137]` | [x] `set/return rc as PCRE2_ERROR_BADREPESCAPE` |
| 342 | `find_text_end` | `if (erc < 0) break; /* capture group reference */ ptr = esc_end_ptr; [c_src/src/pcre2_substitute.c:169]` | [x] `set/return rc as PCRE2_ERROR_BADREPESCAPE` |
| 343 | `find_text_end` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:175]` | [x] `set/return rc as PCRE2_ERROR_REPMISSINGBRACE` |
| 344 | `pcre2_substitute` | `if (partial && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) == 0) [c_src/src/pcre2_substitute.c:797]` | [x] `return PCRE2_ERROR_BADOPTION` |
| 345 | `pcre2_substitute` | `if (rlength != 0) [c_src/src/pcre2_substitute.c:804]` | [x] `return PCRE2_ERROR_NULL` |
| 346 | `pcre2_substitute` | `if (length != 0) [c_src/src/pcre2_substitute.c:815]` | [x] `return PCRE2_ERROR_NULL` |
| 347 | `pcre2_substitute` | `if (use_existing_match && match_data == NULL) [c_src/src/pcre2_substitute.c:827]` | [x] `return PCRE2_ERROR_NULL` |
| 348 | `pcre2_substitute` | `if (match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER) [c_src/src/pcre2_substitute.c:850]` | [x] `return PCRE2_ERROR_DFA_UFUNC` |
| 349 | `pcre2_substitute` | `if (code != match_data->code) [c_src/src/pcre2_substitute.c:853]` | [x] `return PCRE2_ERROR_DIFFSUBSPATTERN` |
| 350 | `pcre2_substitute` | `if (length != match_data->subject_length \|\| !(original_subject == match_data->subject \|\| ((match_data->flags & PCRE2_MD_COPIED_SUBJECT) != 0 && (length == 0 \|\| memcmp(subject, match_data->subject, CU2BYTES(length)) == 0)))) [c_src/src/pcre2_substitute.c:864]` | [x] `return PCRE2_ERROR_DIFFSUBSSUBJECT` |
| 351 | `pcre2_substitute` | `if (start_offset != match_data->start_offset) [c_src/src/pcre2_substitute.c:867]` | [x] `return PCRE2_ERROR_DIFFSUBSOFFSET` |
| 352 | `pcre2_substitute` | `if ((options & ~(SUBSTITUTE_OPTIONS\|PCRE2_NO_UTF_CHECK)) != (match_data->options & ~PCRE2_NO_UTF_CHECK)) [c_src/src/pcre2_substitute.c:871]` | [x] `return PCRE2_ERROR_DIFFSUBSOPTIONS` |
| 353 | `pcre2_substitute` | `if (internal_match_data == NULL) [c_src/src/pcre2_substitute.c:895]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 354 | `pcre2_substitute` | `if (internal_match_data == NULL) [c_src/src/pcre2_substitute.c:909]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 355 | `pcre2_substitute` | `if (start_offset > length) { match_data->leftchar = 0; [c_src/src/pcre2_substitute.c:959]` | [x] `set/return rc as PCRE2_ERROR_BADOFFSET` |
| 356 | `pcre2_substitute` | `if (ovector[1] < ovector[0] \|\| ovector[0] < start_offset) [c_src/src/pcre2_substitute.c:1003]` | [x] `set/return rc as PCRE2_ERROR_BADSUBSPATTERN` |
| 357 | `pcre2_substitute` | `if (subs > 0 && !(ovector[1] > ovecsave[1] \|\| (ovector[1] == ovector[0] && ovecsave[1] > ovecsave[0] && ovector[1] == ovecsave[1]))) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_substitute.c:1021]` | [x] `set/return rc as PCRE2_ERROR_INTERNAL_DUPMATCH` |
| 358 | `pcre2_substitute` | `if (subs == INT_MAX) [c_src/src/pcre2_substitute.c:1034]` | [x] `set/return rc as PCRE2_ERROR_TOOMANYREPLACE` |
| 359 | `pcre2_substitute` | `if (partial) [c_src/src/pcre2_substitute.c:1155]` | [x] `set/return rc as PCRE2_ERROR_PARTIALSUBS` |
| 360 | `pcre2_substitute` | `if (partial) [c_src/src/pcre2_substitute.c:1172]` | [x] `set/return rc as PCRE2_ERROR_PARTIALSUBS` |
| 361 | `pcre2_substitute` | `if ((suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) == 0) [c_src/src/pcre2_substitute.c:1193]` | [x] `set/return rc as PCRE2_ERROR_NOSUBSTRING` |
| 362 | `pcre2_substitute` | `if (match_data->oveccount < code->top_bracket + 1) [c_src/src/pcre2_substitute.c:1204]` | [x] `set/return rc as PCRE2_ERROR_UNAVAILABLE` |
| 363 | `pcre2_substitute` | `if ((suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0) continue; [c_src/src/pcre2_substitute.c:1213]` | [x] `set/return rc as PCRE2_ERROR_UNSET` |
| 364 | `pcre2_substitute` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:1265]` | [x] `set/return rc as PCRE2_ERROR_NOSUBSTRING` |
| 365 | `pcre2_substitute` | `if (special != CHAR_PLUS && special != CHAR_MINUS) [c_src/src/pcre2_substitute.c:1296]` | [x] `set/return rc as PCRE2_ERROR_BADSUBSTITUTION` |
| 366 | `pcre2_substitute` | `if (ptr >= repend \|\| *ptr != CHAR_RIGHT_CURLY_BRACKET) [c_src/src/pcre2_substitute.c:1318]` | [x] `set/return rc as PCRE2_ERROR_REPMISSINGBRACE` |
| 367 | `pcre2_substitute` | `if (rc == PCRE2_ERROR_NOSUBSTRING && (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) != 0) [c_src/src/pcre2_substitute.c:1410]` | [x] `set/return rc as PCRE2_ERROR_UNSET` |
| 368 | `pcre2_substitute` | `if (overflowed) [c_src/src/pcre2_substitute.c:1750]` | [x] `set/return rc as PCRE2_ERROR_NOMEMORY` |
| 369 | `pcre2_substitute` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:1772]` | [x] `set/return rc as PCRE2_ERROR_NOMEMORY` |
| 370 | `pcre2_substitute` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:1776]` | [x] `set/return rc as PCRE2_ERROR_REPLACECASE` |
| 371 | `pcre2_substitute` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:1780]` | [x] `set/return rc as PCRE2_ERROR_TOOLARGEREPLACE` |
| 372 | `pcre2_substitute` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:1784]` | [x] `set/return rc as PCRE2_ERROR_BADREPLACEMENT` |
| 373 | `pcre2_substitute` | `unconditional rejection reached by the enclosing C control flow [c_src/src/pcre2_substitute.c:1788]` | [x] `set/return rc as PCRE2_ERROR_BADREPESCAPE` |
| 374 | `pcre2_substring_copy_byname` | `if (match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER) [c_src/src/pcre2_substring.c:75]` | [x] `return PCRE2_ERROR_DFA_UFUNC` |
| 375 | `pcre2_substring_copy_bynumber` | `if (size + 1 > *sizeptr) [c_src/src/pcre2_substring.c:124]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 376 | `pcre2_substring_get_byname` | `if (match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER) [c_src/src/pcre2_substring.c:163]` | [x] `return PCRE2_ERROR_DFA_UFUNC` |
| 377 | `pcre2_substring_get_bynumber` | `if (yield == NULL) [c_src/src/pcre2_substring.c:215]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 378 | `pcre2_substring_length_byname` | `if (match_data->matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER) [c_src/src/pcre2_substring.c:270]` | [x] `return PCRE2_ERROR_DFA_UFUNC` |
| 379 | `pcre2_substring_length_bynumber` | `if (stringnumber > 0) [c_src/src/pcre2_substring.c:319]` | [x] `return PCRE2_ERROR_PARTIAL` |
| 380 | `pcre2_substring_length_bynumber` | `if (stringnumber > match_data->code->top_bracket) [c_src/src/pcre2_substring.c:327]` | [x] `return PCRE2_ERROR_NOSUBSTRING` |
| 381 | `pcre2_substring_length_bynumber` | `if (stringnumber >= match_data->oveccount) [c_src/src/pcre2_substring.c:329]` | [x] `return PCRE2_ERROR_UNAVAILABLE` |
| 382 | `pcre2_substring_length_bynumber` | `if (match_data->ovector[stringnumber*2] == PCRE2_UNSET) [c_src/src/pcre2_substring.c:331]` | [x] `return PCRE2_ERROR_UNSET` |
| 383 | `pcre2_substring_length_bynumber` | `if (stringnumber >= match_data->oveccount) [c_src/src/pcre2_substring.c:335]` | [x] `return PCRE2_ERROR_UNAVAILABLE` |
| 384 | `pcre2_substring_length_bynumber` | `if (count != 0 && stringnumber >= (uint32_t)count) [c_src/src/pcre2_substring.c:336]` | [x] `return PCRE2_ERROR_UNSET` |
| 385 | `pcre2_substring_length_bynumber` | `if (left > match_data->subject_length \|\| right > match_data->subject_length) { PCRE2_DEBUG_UNREACHABLE(); [c_src/src/pcre2_substring.c:347]` | [x] `return PCRE2_ERROR_INVALIDOFFSET` |
| 386 | `pcre2_substring_list_get` | `if (memp == NULL) [c_src/src/pcre2_substring.c:404]` | [x] `return PCRE2_ERROR_NOMEMORY` |
| 387 | `pcre2_substring_nametable_scan` | `if (c > 0) bot = mid + 1; else top = mid; } [c_src/src/pcre2_substring.c:525]` | [x] `return PCRE2_ERROR_NOSUBSTRING` |
| 388 | `PRIV(valid_utf)` | `if (c < 0xc0) /* Isolated 10xx xxxx byte */ [c_src/src/pcre2_valid_utf.c:145]` | [x] `return PCRE2_ERROR_UTF8_ERR20` |
| 389 | `PRIV(valid_utf)` | `if (c >= 0xfe) /* Invalid 0xfe or 0xff bytes */ [c_src/src/pcre2_valid_utf.c:151]` | [x] `return PCRE2_ERROR_UTF8_ERR21` |
| 390 | `PRIV(valid_utf)` | `case 1: [c_src/src/pcre2_valid_utf.c:160]` | [x] `return PCRE2_ERROR_UTF8_ERR1` |
| 391 | `PRIV(valid_utf)` | `case 2: [c_src/src/pcre2_valid_utf.c:161]` | [x] `return PCRE2_ERROR_UTF8_ERR2` |
| 392 | `PRIV(valid_utf)` | `case 3: [c_src/src/pcre2_valid_utf.c:162]` | [x] `return PCRE2_ERROR_UTF8_ERR3` |
| 393 | `PRIV(valid_utf)` | `case 4: [c_src/src/pcre2_valid_utf.c:163]` | [x] `return PCRE2_ERROR_UTF8_ERR4` |
| 394 | `PRIV(valid_utf)` | `case 5: [c_src/src/pcre2_valid_utf.c:164]` | [x] `return PCRE2_ERROR_UTF8_ERR5` |
| 395 | `PRIV(valid_utf)` | `if (((d = *(++p)) & 0xc0) != 0x80) [c_src/src/pcre2_valid_utf.c:174]` | [x] `return PCRE2_ERROR_UTF8_ERR6` |
| 396 | `PRIV(valid_utf)` | `case 1: if ((c & 0x3e) == 0) [c_src/src/pcre2_valid_utf.c:189]` | [x] `return PCRE2_ERROR_UTF8_ERR15` |
| 397 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Third byte */ [c_src/src/pcre2_valid_utf.c:201]` | [x] `return PCRE2_ERROR_UTF8_ERR7` |
| 398 | `PRIV(valid_utf)` | `if (c == 0xe0 && (d & 0x20) == 0) [c_src/src/pcre2_valid_utf.c:206]` | [x] `return PCRE2_ERROR_UTF8_ERR16` |
| 399 | `PRIV(valid_utf)` | `if (c == 0xed && d >= 0xa0) [c_src/src/pcre2_valid_utf.c:211]` | [x] `return PCRE2_ERROR_UTF8_ERR14` |
| 400 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Third byte */ [c_src/src/pcre2_valid_utf.c:223]` | [x] `return PCRE2_ERROR_UTF8_ERR7` |
| 401 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Fourth byte */ [c_src/src/pcre2_valid_utf.c:228]` | [x] `return PCRE2_ERROR_UTF8_ERR8` |
| 402 | `PRIV(valid_utf)` | `if (c == 0xf0 && (d & 0x30) == 0) [c_src/src/pcre2_valid_utf.c:233]` | [x] `return PCRE2_ERROR_UTF8_ERR17` |
| 403 | `PRIV(valid_utf)` | `if (c > 0xf4 \|\| (c == 0xf4 && d > 0x8f)) [c_src/src/pcre2_valid_utf.c:238]` | [x] `return PCRE2_ERROR_UTF8_ERR13` |
| 404 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Third byte */ [c_src/src/pcre2_valid_utf.c:254]` | [x] `return PCRE2_ERROR_UTF8_ERR7` |
| 405 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Fourth byte */ [c_src/src/pcre2_valid_utf.c:259]` | [x] `return PCRE2_ERROR_UTF8_ERR8` |
| 406 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Fifth byte */ [c_src/src/pcre2_valid_utf.c:264]` | [x] `return PCRE2_ERROR_UTF8_ERR9` |
| 407 | `PRIV(valid_utf)` | `if (c == 0xf8 && (d & 0x38) == 0) [c_src/src/pcre2_valid_utf.c:269]` | [x] `return PCRE2_ERROR_UTF8_ERR18` |
| 408 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Third byte */ [c_src/src/pcre2_valid_utf.c:280]` | [x] `return PCRE2_ERROR_UTF8_ERR7` |
| 409 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Fourth byte */ [c_src/src/pcre2_valid_utf.c:285]` | [x] `return PCRE2_ERROR_UTF8_ERR8` |
| 410 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Fifth byte */ [c_src/src/pcre2_valid_utf.c:290]` | [x] `return PCRE2_ERROR_UTF8_ERR9` |
| 411 | `PRIV(valid_utf)` | `if ((*(++p) & 0xc0) != 0x80) /* Sixth byte */ [c_src/src/pcre2_valid_utf.c:295]` | [x] `return PCRE2_ERROR_UTF8_ERR10` |
| 412 | `PRIV(valid_utf)` | `if (c == 0xfc && (d & 0x3c) == 0) [c_src/src/pcre2_valid_utf.c:300]` | [x] `return PCRE2_ERROR_UTF8_ERR19` |
| 413 | `PRIV(valid_utf)` | `if (ab > 3) [c_src/src/pcre2_valid_utf.c:312]` | [x] `return (ab == 4)? PCRE2_ERROR_UTF8_ERR11 : PCRE2_ERROR_UTF8_ERR12` |
