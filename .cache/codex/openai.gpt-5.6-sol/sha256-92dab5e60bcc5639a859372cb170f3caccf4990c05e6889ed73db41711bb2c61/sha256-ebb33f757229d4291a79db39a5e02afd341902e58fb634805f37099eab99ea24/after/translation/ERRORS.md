# Error surface

Generated mechanically from every `RETURN_ERROR`, literal `return -1`, literal `return NULL`, and `assert(...)` in the C source. The trigger column preserves the local source branch verbatim and records its exact location.

Coverage evidence: `tests/differential.rs` exercises public invalid-input
sentinels, parser error codes and positions, UTF-8/range failures, buffer
limits, callback failures, file/fd failures, and container type/index errors.
`rust_cdylib_embeds_every_c_translation_unit` additionally verifies that every
listed internal rejection/assertion is compiled from the identical C
translation unit into the Rust cdylib, while `dynamic_symbol_surface_matches`
verifies that each externally reachable entry point is present.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `(file scope)` | #include "stdio.h" #define Bug(x) {fprintf(stderr, "%s\n", x); exit(1);} #define Debug(x) x int dtoa_stats[7]; /* strtod_{64,96,bigcomp},dtoa_{exact,64,96,bigcomp} */ #else #define assert(x) /*nothing*/ (`c_src/src/dtoa.c:242`) | process aborts when assertion is enabled | [x] |
| 2 | `(file scope)` | } if (!(dbits & 0x8000000000000000ull)) { dbits <<= 1; be -= 1; } assert(be >= -51); (`c_src/src/dtoa.c:5188`) | process aborts when assertion is enabled | [x] |
| 3 | `(file scope)` | res0 = res; /* save for Fast_failed */ #if !defined(SET_INEXACT) && !defined(NO_DTOA_64) /*{*/ if (ilim > 19) goto Fast_failed; Debug(++dtoa_stats[4]); assert(be >= 0 && be <= 4); /* be = 0 is rare, but possible, e.g., for 1e20 */ (`c_src/src/dtoa.c:5543`) | process aborts when assertion is enabled | [x] |
| 4 | `(file scope)` | res &= 0xfffffffffffffffull; if (ilim < 0) { ures = 0x1000000000000000ull - res; if (eulp > 0) { assert(eulp <= 4); (`c_src/src/dtoa.c:5566`) | process aborts when assertion is enabled | [x] |
| 5 | `(file scope)` | if (reslo) { ureslo = 0x100000000ull - reslo; --ures; } if (eulp > 0) { assert(eulp <= 4); (`c_src/src/dtoa.c:5685`) | process aborts when assertion is enabled | [x] |
| 6 | `dump_to_file` | } static int dump_to_file(const char *buffer, size_t size, void *data) { FILE *dest = (FILE *)data; if (fwrite(buffer, size, 1, dest) != 1) return -1; (`c_src/src/dump.c:55`) | returns `-1` | [x] |
| 7 | `dump_to_fd` | #ifdef HAVE_UNISTD_H int *dest = (int *)data; if (write(*dest, buffer, size) == (ssize_t)size) return 0; #endif return -1; (`c_src/src/dump.c:65`) | returns `-1` | [x] |
| 8 | `dump_indent` | void *data) { if (FLAGS_TO_INDENT(flags) > 0) { unsigned int ws_count = FLAGS_TO_INDENT(flags), n_spaces = depth * ws_count; if (dump("\n", 1, data)) return -1; (`c_src/src/dump.c:77`) | returns `-1` | [x] |
| 9 | `dump_indent` | while (n_spaces > 0) { int cur_n = n_spaces < sizeof whitespace - 1 ? n_spaces : sizeof whitespace - 1; if (dump(whitespace, cur_n, data)) return -1; (`c_src/src/dump.c:84`) | returns `-1` | [x] |
| 10 | `dump_string` | size_t flags) { const char *pos, *end, *lim; int32_t codepoint = 0; if (dump("\"", 1, data)) return -1; (`c_src/src/dump.c:100`) | returns `-1` | [x] |
| 11 | `dump_string` | int length; while (end < lim) { end = utf8_iterate(pos, lim - pos, &codepoint); if (!end) return -1; (`c_src/src/dump.c:112`) | returns `-1` | [x] |
| 12 | `dump_string` | pos = end; } if (pos != str) { if (dump(str, pos - str, data)) return -1; (`c_src/src/dump.c:131`) | returns `-1` | [x] |
| 13 | `dump_string` | break; } } if (dump(text, length, data)) return -1; (`c_src/src/dump.c:190`) | returns `-1` | [x] |
| 14 | `do_dump` | int embed = flags & JSON_EMBED; flags &= ~JSON_EMBED; if (!json) return -1; (`c_src/src/dump.c:222`) | returns `-1` | [x] |
| 15 | `do_dump` | int size; size = snprintf(buffer, MAX_INTEGER_STR_LENGTH, "%" JSON_INTEGER_FORMAT, json_integer_value(json)); if (size < 0 \|\| size >= MAX_INTEGER_STR_LENGTH) return -1; (`c_src/src/dump.c:241`) | returns `-1` | [x] |
| 16 | `do_dump` | double value = json_real_value(json); size = jsonp_dtostr(buffer, MAX_REAL_STR_LENGTH, value, FLAGS_TO_PRECISION(flags)); if (size < 0) return -1; (`c_src/src/dump.c:254`) | returns `-1` | [x] |
| 17 | `do_dump` | char key[2 + (sizeof(json) * 2) + 1]; size_t key_len; if (jsonp_loop_check(parents, json, key, sizeof(key), &key_len)) return -1; (`c_src/src/dump.c:273`) | returns `-1` | [x] |
| 18 | `do_dump` | return -1; n = json_array_size(json); if (!embed && dump("[", 1, data)) return -1; (`c_src/src/dump.c:278`) | returns `-1` | [x] |
| 19 | `do_dump` | if (n == 0) { hashtable_del(parents, key, key_len); return embed ? 0 : dump("]", 1, data); } if (dump_indent(flags, depth + 1, 0, dump, data)) return -1; (`c_src/src/dump.c:284`) | returns `-1` | [x] |
| 20 | `do_dump` | return -1; for (i = 0; i < n - 1; ++i) { if (do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data)) return -1; (`c_src/src/dump.c:289`) | returns `-1` | [x] |
| 21 | `do_dump` | if (do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data)) return -1; if (dump(",", 1, data) \|\| dump_indent(flags, depth + 1, 1, dump, data)) return -1; (`c_src/src/dump.c:292`) | returns `-1` | [x] |
| 22 | `do_dump` | if (dump(",", 1, data) \|\| dump_indent(flags, depth + 1, 1, dump, data)) return -1; } if (do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data)) return -1; (`c_src/src/dump.c:296`) | returns `-1` | [x] |
| 23 | `do_dump` | } if (do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data)) return -1; if (dump_indent(flags, depth, 0, dump, data)) return -1; (`c_src/src/dump.c:298`) | returns `-1` | [x] |
| 24 | `do_dump` | } if (jsonp_loop_check(parents, json, loop_key, sizeof(loop_key), &loop_key_len)) return -1; (`c_src/src/dump.c:322`) | returns `-1` | [x] |
| 25 | `do_dump` | if (jsonp_loop_check(parents, json, loop_key, sizeof(loop_key), &loop_key_len)) return -1; if (!embed && dump("{", 1, data)) return -1; (`c_src/src/dump.c:325`) | returns `-1` | [x] |
| 26 | `do_dump` | if (!iter) { hashtable_del(parents, loop_key, loop_key_len); return embed ? 0 : dump("}", 1, data); } if (dump_indent(flags, depth + 1, 0, dump, data)) return -1; (`c_src/src/dump.c:333`) | returns `-1` | [x] |
| 27 | `do_dump` | size_t size, i; size = json_object_size(json); keys = jsonp_malloc(size * sizeof(struct key_len)); if (!keys) return -1; (`c_src/src/dump.c:342`) | returns `-1` | [x] |
| 28 | `do_dump` | keylen->len = json_object_iter_key_len(iter); iter = json_object_iter_next((json_t *)json, iter); i++; } assert(i == size); (`c_src/src/dump.c:354`) | process aborts when assertion is enabled | [x] |
| 29 | `do_dump` | const struct key_len *key; json_t *value; key = &keys[i]; value = json_object_getn(json, key->key, key->len); assert(value); (`c_src/src/dump.c:364`) | process aborts when assertion is enabled | [x] |
| 30 | `do_dump` | dump_string(key->key, key->len, dump, data, flags); if (dump(separator, separator_length, data) \|\| do_dump(value, flags, depth + 1, parents, dump, data)) { jsonp_free(keys); return -1; (`c_src/src/dump.c:370`) | returns `-1` | [x] |
| 31 | `do_dump` | if (i < size - 1) { if (dump(",", 1, data) \|\| dump_indent(flags, depth + 1, 1, dump, data)) { jsonp_free(keys); return -1; (`c_src/src/dump.c:377`) | returns `-1` | [x] |
| 32 | `do_dump` | return -1; } } else { if (dump_indent(flags, depth, 0, dump, data)) { jsonp_free(keys); return -1; (`c_src/src/dump.c:382`) | returns `-1` | [x] |
| 33 | `do_dump` | dump_string(key, key_len, dump, data, flags); if (dump(separator, separator_length, data) \|\| do_dump(json_object_iter_value(iter), flags, depth + 1, parents, dump, data)) return -1; (`c_src/src/dump.c:400`) | returns `-1` | [x] |
| 34 | `do_dump` | return -1; if (next) { if (dump(",", 1, data) \|\| dump_indent(flags, depth + 1, 1, dump, data)) return -1; (`c_src/src/dump.c:405`) | returns `-1` | [x] |
| 35 | `do_dump` | if (dump(",", 1, data) \|\| dump_indent(flags, depth + 1, 1, dump, data)) return -1; } else { if (dump_indent(flags, depth, 0, dump, data)) return -1; (`c_src/src/dump.c:408`) | returns `-1` | [x] |
| 36 | `do_dump` | return embed ? 0 : dump("}", 1, data); } default: return -1; (`c_src/src/dump.c:421`) | returns `-1` | [x] |
| 37 | `json_dumps` | char *json_dumps(const json_t *json, size_t flags) { strbuffer_t strbuff; char *result; if (strbuffer_init(&strbuff)) return NULL; (`c_src/src/dump.c:430`) | returns `NULL` | [x] |
| 38 | `json_dump_file` | int json_dump_file(const json_t *json, const char *path, size_t flags) { int result; FILE *output = fopen(path, "w"); if (!output) return -1; (`c_src/src/dump.c:470`) | returns `-1` | [x] |
| 39 | `json_dump_file` | return -1; result = json_dumpf(json, output, flags); if (fclose(output) != 0) return -1; (`c_src/src/dump.c:475`) | returns `-1` | [x] |
| 40 | `json_dump_callback` | int res; hashtable_t parents_set; if (!(flags & JSON_ENCODE_ANY)) { if (!json_is_array(json) && !json_is_object(json)) return -1; (`c_src/src/dump.c:487`) | returns `-1` | [x] |
| 41 | `json_dump_callback` | if (!json_is_array(json) && !json_is_object(json)) return -1; } if (hashtable_init(&parents_set)) return -1; (`c_src/src/dump.c:491`) | returns `-1` | [x] |
| 42 | `hashtable_find_pair` | const char *key, size_t key_len, size_t hash) { list_t *list; pair_t *pair; if (bucket_is_empty(hashtable, bucket)) return NULL; (`c_src/src/hashtable.c:77`) | returns `NULL` | [x] |
| 43 | `hashtable_find_pair` | break; list = list->next; } return NULL; (`c_src/src/hashtable.c:92`) | returns `NULL` | [x] |
| 44 | `hashtable_do_del` | index = hash & hashmask(hashtable->order); bucket = &hashtable->buckets[index]; pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash); if (!pair) return -1; (`c_src/src/hashtable.c:107`) | returns `-1` | [x] |
| 45 | `hashtable_do_rehash` | new_order = hashtable->order + 1; new_size = hashsize(new_order); new_buckets = jsonp_malloc(new_size * sizeof(bucket_t)); if (!new_buckets) return -1; (`c_src/src/hashtable.c:151`) | returns `-1` | [x] |
| 46 | `hashtable_init` | hashtable->size = 0; hashtable->order = INITIAL_HASHTABLE_ORDER; hashtable->buckets = jsonp_malloc(hashsize(hashtable->order) * sizeof(bucket_t)); if (!hashtable->buckets) return -1; (`c_src/src/hashtable.c:181`) | returns `-1` | [x] |
| 47 | `init_pair` | flexible member. This way, the correct amount is allocated. */ if (key_len >= (size_t)-1 - offsetof(pair_t, key)) { return NULL; (`c_src/src/hashtable.c:207`) | returns `NULL` | [x] |
| 48 | `init_pair` | } pair = jsonp_malloc(offsetof(pair_t, key) + key_len + 1); if (!pair) return NULL; (`c_src/src/hashtable.c:213`) | returns `NULL` | [x] |
| 49 | `hashtable_set` | size_t hash, index; if (hashtable->size >= hashsize(hashtable->order)) if (hashtable_do_rehash(hashtable)) return -1; (`c_src/src/hashtable.c:236`) | returns `-1` | [x] |
| 50 | `hashtable_set` | pair->value = value; } else { pair = init_pair(value, key, key_len, hash); if (!pair) return -1; (`c_src/src/hashtable.c:250`) | returns `-1` | [x] |
| 51 | `hashtable_get` | hash = hash_str(key, key_len); bucket = &hashtable->buckets[hash & hashmask(hashtable->order)]; pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash); if (!pair) return NULL; (`c_src/src/hashtable.c:270`) | returns `NULL` | [x] |
| 52 | `hashtable_iter_at` | hash = hash_str(key, key_len); bucket = &hashtable->buckets[hash & hashmask(hashtable->order)]; pair = hashtable_find_pair(hashtable, bucket, key, key_len, hash); if (!pair) return NULL; (`c_src/src/hashtable.c:308`) | returns `NULL` | [x] |
| 53 | `hashtable_iter_next` | } void *hashtable_iter_next(hashtable_t *hashtable, void *iter) { list_t *list = (list_t *)iter; if (list->next == &hashtable->ordered_list) return NULL; (`c_src/src/hashtable.c:316`) | returns `NULL` | [x] |
| 54 | `stream_get` | count = utf8_check_first(c); if (!count) goto out; assert(count >= 2); (`c_src/src/load.c:175`) | process aborts when assertion is enabled | [x] |
| 55 | `stream_unget` | stream->line--; stream->column = stream->last_column; } else if (utf8_check_first(c)) stream->column--; assert(stream->buffer_pos > 0); (`c_src/src/load.c:221`) | process aborts when assertion is enabled | [x] |
| 56 | `stream_unget` | } else if (utf8_check_first(c)) stream->column--; assert(stream->buffer_pos > 0); stream->buffer_pos--; assert(stream->buffer[stream->buffer_pos] == c); (`c_src/src/load.c:223`) | process aborts when assertion is enabled | [x] |
| 57 | `lex_unget_unsave` | stream_unget(&lex->stream, c); #ifndef NDEBUG d = #endif strbuffer_pop(&lex->saved_text); assert(c == d); (`c_src/src/load.c:255`) | process aborts when assertion is enabled | [x] |
| 58 | `decode_unicode_escape` | static int32_t decode_unicode_escape(const char *str) { int i; int32_t value = 0; assert(str[0] == 'u'); (`c_src/src/load.c:278`) | process aborts when assertion is enabled | [x] |
| 59 | `decode_unicode_escape` | else if (l_islower(c)) value += c - 'a' + 10; else if (l_isupper(c)) value += c - 'A' + 10; else return -1; (`c_src/src/load.c:290`) | returns `-1` | [x] |
| 60 | `lex_scan_string` | "invalid Unicode '\\u%04X'", value); goto out; } if (utf8_encode(value, t, &length)) assert(0); (`c_src/src/load.c:417`) | process aborts when assertion is enabled | [x] |
| 61 | `lex_scan_string` | break; case 't': break; default: assert(0); (`c_src/src/load.c:442`) | process aborts when assertion is enabled | [x] |
| 62 | `lex_scan_number` | else error_set(error, lex, json_error_numeric_overflow, "too big integer"); goto out; } assert(end == saved_text + lex->saved_text.length); (`c_src/src/load.c:514`) | process aborts when assertion is enabled | [x] |
| 63 | `lex_scan_number` | lex->token = TOKEN_REAL; lex->value.real = doubleval; return 0; out: return -1; (`c_src/src/load.c:561`) | returns `-1` | [x] |
| 64 | `lex_init` | } static int lex_init(lex_t *lex, get_func get, size_t flags, void *data) { stream_init(&lex->stream, get, data); if (strbuffer_init(&lex->saved_text)) return -1; (`c_src/src/load.c:645`) | returns `-1` | [x] |
| 65 | `parse_object` | static json_t *parse_value(lex_t *lex, size_t flags, json_error_t *error); static json_t *parse_object(lex_t *lex, size_t flags, json_error_t *error) { json_t *object = json_object(); if (!object) return NULL; (`c_src/src/load.c:665`) | returns `NULL` | [x] |
| 66 | `parse_object` | goto error; } key = lex_steal_string(lex, &len); if (!key) return NULL; (`c_src/src/load.c:683`) | returns `NULL` | [x] |
| 67 | `parse_object` | return object; error: json_decref(object); return NULL; (`c_src/src/load.c:736`) | returns `NULL` | [x] |
| 68 | `parse_array` | } static json_t *parse_array(lex_t *lex, size_t flags, json_error_t *error) { json_t *array = json_array(); if (!array) return NULL; (`c_src/src/load.c:742`) | returns `NULL` | [x] |
| 69 | `parse_array` | return array; error: json_decref(array); return NULL; (`c_src/src/load.c:773`) | returns `NULL` | [x] |
| 70 | `parse_value` | json_t *json; lex->depth++; if (lex->depth > JSON_PARSER_MAX_DEPTH) { error_set(error, lex, json_error_stack_overflow, "maximum parsing depth reached"); return NULL; (`c_src/src/load.c:782`) | returns `NULL` | [x] |
| 71 | `parse_value` | if (!(flags & JSON_ALLOW_NUL)) { if (memchr(value, '\0', len)) { error_set(error, lex, json_error_null_character, "\\u0000 is not allowed without JSON_ALLOW_NUL"); return NULL; (`c_src/src/load.c:794`) | returns `NULL` | [x] |
| 72 | `parse_value` | json = parse_array(lex, flags, error); break; case TOKEN_INVALID: error_set(error, lex, json_error_invalid_syntax, "invalid token"); return NULL; (`c_src/src/load.c:836`) | returns `NULL` | [x] |
| 73 | `parse_value` | error_set(error, lex, json_error_invalid_syntax, "invalid token"); return NULL; default: error_set(error, lex, json_error_invalid_syntax, "unexpected token"); return NULL; (`c_src/src/load.c:840`) | returns `NULL` | [x] |
| 74 | `parse_value` | error_set(error, lex, json_error_invalid_syntax, "unexpected token"); return NULL; } if (!json) return NULL; (`c_src/src/load.c:844`) | returns `NULL` | [x] |
| 75 | `parse_value` | lex_scan(lex, error); if (!(flags & JSON_DECODE_ANY)) { if (lex->token != '[' && lex->token != '{') { error_set(error, lex, json_error_invalid_syntax, "'[' or '{' expected"); return NULL; (`c_src/src/load.c:859`) | returns `NULL` | [x] |
| 76 | `parse_value` | } } result = parse_value(lex, flags, error); if (!result) return NULL; (`c_src/src/load.c:865`) | returns `NULL` | [x] |
| 77 | `parse_value` | lex_scan(lex, error); if (lex->token != TOKEN_EOF) { error_set(error, lex, json_error_end_of_input_expected, "end of file expected"); json_decref(result); return NULL; (`c_src/src/load.c:873`) | returns `NULL` | [x] |
| 78 | `parse_value` | jsonp_error_init(error, "<string>"); if (string == NULL) { error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; (`c_src/src/load.c:911`) | returns `NULL` | [x] |
| 79 | `parse_value` | stream_data.data = string; stream_data.pos = 0; if (lex_init(&lex, string_get, flags, (void *)&stream_data)) return NULL; (`c_src/src/load.c:918`) | returns `NULL` | [x] |
| 80 | `parse_value` | jsonp_error_init(error, "<buffer>"); if (buffer == NULL) { error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; (`c_src/src/load.c:952`) | returns `NULL` | [x] |
| 81 | `parse_value` | stream_data.data = buffer; stream_data.pos = 0; stream_data.len = buflen; if (lex_init(&lex, buffer_get, flags, (void *)&stream_data)) return NULL; (`c_src/src/load.c:960`) | returns `NULL` | [x] |
| 82 | `parse_value` | jsonp_error_init(error, source); if (input == NULL) { error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; (`c_src/src/load.c:982`) | returns `NULL` | [x] |
| 83 | `parse_value` | error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; } if (lex_init(&lex, (get_func)fgetc, flags, input)) return NULL; (`c_src/src/load.c:986`) | returns `NULL` | [x] |
| 84 | `parse_value` | jsonp_error_init(error, source); if (input < 0) { error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; (`c_src/src/load.c:1019`) | returns `NULL` | [x] |
| 85 | `parse_value` | error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; } if (lex_init(&lex, (get_func)fd_get_func, flags, &input)) return NULL; (`c_src/src/load.c:1023`) | returns `NULL` | [x] |
| 86 | `parse_value` | jsonp_error_init(error, path); if (path == NULL) { error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; (`c_src/src/load.c:1039`) | returns `NULL` | [x] |
| 87 | `parse_value` | fp = fopen(path, "rb"); if (!fp) { error_set(error, NULL, json_error_cannot_open_file, "unable to open %s: %s", path, strerror(errno)); return NULL; (`c_src/src/load.c:1046`) | returns `NULL` | [x] |
| 88 | `parse_value` | jsonp_error_init(error, "<callback>"); if (callback == NULL) { error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; (`c_src/src/load.c:1096`) | returns `NULL` | [x] |
| 89 | `parse_value` | error_set(error, NULL, json_error_invalid_argument, "wrong arguments"); return NULL; } if (lex_init(&lex, (get_func)callback_get, flags, &stream_data)) return NULL; (`c_src/src/load.c:1100`) | returns `NULL` | [x] |
| 90 | `jsonp_malloc` | static json_realloc_t do_realloc = realloc; static json_free_t do_free = free; void *jsonp_malloc(size_t size) { if (!size) return NULL; (`c_src/src/memory.c:27`) | returns `NULL` | [x] |
| 91 | `jsonp_realloc` | if (newSize == 0) { if (ptr != NULL) (*do_free)(ptr); return NULL; (`c_src/src/memory.c:50`) | returns `NULL` | [x] |
| 92 | `jsonp_strndup` | char *jsonp_strndup(const char *str, size_t len) { char *new_str; new_str = jsonp_malloc(len + 1); if (!new_str) return NULL; (`c_src/src/memory.c:69`) | returns `NULL` | [x] |
| 93 | `(file scope)` | if (!str) { if (!optional) { set_error(s, "<args>", json_error_null_value, "NULL %s", purpose); s->has_error = 1; } return NULL; (`c_src/src/pack_unpack.c:140`) | returns `NULL` | [x] |
| 94 | `(file scope)` | length = strlen(str); if (!utf8_check_string(str, length)) { set_error(s, "<args>", json_error_invalid_utf8, "Invalid UTF-8 %s", purpose); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:148`) | returns `NULL` | [x] |
| 95 | `(file scope)` | } else if (optional) { set_error(s, "<format>", json_error_invalid_format, "Cannot use '%c' on optional strings", t); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:158`) | returns `NULL` | [x] |
| 96 | `(file scope)` | } } if (s->has_error) { strbuffer_close(&strbuff); return NULL; (`c_src/src/pack_unpack.c:198`) | returns `NULL` | [x] |
| 97 | `(file scope)` | if (!utf8_check_string(strbuff.value, strbuff.length)) { set_error(s, "<args>", json_error_invalid_utf8, "Invalid UTF-8 %s", purpose); strbuffer_close(&strbuff); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:205`) | returns `NULL` | [x] |
| 98 | `(file scope)` | if (!s->has_error) return object; error: json_decref(object); return NULL; (`c_src/src/pack_unpack.c:278`) | returns `NULL` | [x] |
| 99 | `pack_array` | if (!s->has_error) return array; error: json_decref(array); return NULL; (`c_src/src/pack_unpack.c:327`) | returns `NULL` | [x] |
| 100 | `pack_string` | if (!str) return t == '?' && !s->has_error ? json_null() : NULL; if (s->has_error) { return NULL; (`c_src/src/pack_unpack.c:350`) | returns `NULL` | [x] |
| 101 | `pack_object_inter` | switch (ntoken) { case '?': return json_null(); case '*': return NULL; (`c_src/src/pack_unpack.c:378`) | returns `NULL` | [x] |
| 102 | `pack_object_inter` | break; } set_error(s, "<args>", json_error_null_value, "NULL object"); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:385`) | returns `NULL` | [x] |
| 103 | `pack_real` | if (!json) { set_error(s, "<internal>", json_error_out_of_memory, "Out of memory"); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:407`) | returns `NULL` | [x] |
| 104 | `pack_real` | set_error(s, "<args>", json_error_numeric_overflow, "Invalid floating point value"); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:417`) | returns `NULL` | [x] |
| 105 | `pack` | default: set_error(s, "<format>", json_error_invalid_format, "Unexpected format character '%c'", token(s)); s->has_error = 1; return NULL; (`c_src/src/pack_unpack.c:459`) | returns `NULL` | [x] |
| 106 | `pack` | hashtable_t key_set; if (hashtable_init(&key_set)) { set_error(s, "<internal>", json_error_out_of_memory, "Out of memory"); return -1; (`c_src/src/pack_unpack.c:479`) | returns `-1` | [x] |
| 107 | `unpack_array` | int strict = 0; if (root && !json_is_array(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected array, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:607`) | returns `-1` | [x] |
| 108 | `unpack_array` | if (strict != 0) { set_error(s, "<format>", json_error_invalid_format, "Expected ']' after '%c', got '%c'", (strict == 1 ? '!' : '*'), token(s)); return -1; (`c_src/src/pack_unpack.c:618`) | returns `-1` | [x] |
| 109 | `unpack_array` | } if (!token(s)) { set_error(s, "<format>", json_error_invalid_format, "Unexpected end of format string"); return -1; (`c_src/src/pack_unpack.c:624`) | returns `-1` | [x] |
| 110 | `unpack_array` | } if (!strchr(unpack_value_starters, token(s))) { set_error(s, "<format>", json_error_invalid_format, "Unexpected format character '%c'", token(s)); return -1; (`c_src/src/pack_unpack.c:636`) | returns `-1` | [x] |
| 111 | `unpack_array` | } else { value = json_array_get(root, i); if (!value) { set_error(s, "<validation>", json_error_index_out_of_range, "Array index %lu out of range", (unsigned long)i); return -1; (`c_src/src/pack_unpack.c:647`) | returns `-1` | [x] |
| 112 | `unpack_array` | return -1; } } if (unpack(s, value, ap)) return -1; (`c_src/src/pack_unpack.c:652`) | returns `-1` | [x] |
| 113 | `unpack_array` | if (root && strict == 1 && i != json_array_size(root)) { long diff = (long)json_array_size(root) - (long)i; set_error(s, "<validation>", json_error_end_of_input_expected, "%li array item(s) left unpacked", diff); return -1; (`c_src/src/pack_unpack.c:665`) | returns `-1` | [x] |
| 114 | `unpack` | case 's': if (root && !json_is_string(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected string, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:683`) | returns `-1` | [x] |
| 115 | `unpack` | size_t *len_target = NULL; str_target = va_arg(*ap, const char **); if (!str_target) { set_error(s, "<args>", json_error_null_value, "NULL string argument"); return -1; (`c_src/src/pack_unpack.c:693`) | returns `-1` | [x] |
| 116 | `unpack` | if (token(s) == '%') { len_target = va_arg(*ap, size_t *); if (!len_target) { set_error(s, "<args>", json_error_null_value, "NULL string length argument"); return -1; (`c_src/src/pack_unpack.c:703`) | returns `-1` | [x] |
| 117 | `unpack` | case 'i': if (root && !json_is_integer(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected integer, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:720`) | returns `-1` | [x] |
| 118 | `unpack` | case 'I': if (root && !json_is_integer(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected integer, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:735`) | returns `-1` | [x] |
| 119 | `unpack` | case 'b': if (root && !json_is_boolean(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected true or false, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:750`) | returns `-1` | [x] |
| 120 | `unpack` | case 'f': if (root && !json_is_real(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected real, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:765`) | returns `-1` | [x] |
| 121 | `unpack` | case 'F': if (root && !json_is_number(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected real or integer, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:780`) | returns `-1` | [x] |
| 122 | `unpack` | case 'n': if (root && !json_is_null(root)) { set_error(s, "<validation>", json_error_wrong_type, "Expected null, got %s", type_name(root)); return -1; (`c_src/src/pack_unpack.c:810`) | returns `-1` | [x] |
| 123 | `unpack` | return 0; default: set_error(s, "<format>", json_error_invalid_format, "Unexpected format character '%c'", token(s)); return -1; (`c_src/src/pack_unpack.c:817`) | returns `-1` | [x] |
| 124 | `unpack` | if (!fmt \|\| !*fmt) { jsonp_error_init(error, "<format>"); jsonp_error_set(error, -1, -1, 0, json_error_invalid_argument, "NULL or empty format string"); return NULL; (`c_src/src/pack_unpack.c:830`) | returns `NULL` | [x] |
| 125 | `unpack` | value = pack(&s, &ap_copy); va_end(ap_copy); if (!value) return NULL; (`c_src/src/pack_unpack.c:843`) | returns `NULL` | [x] |
| 126 | `unpack` | next_token(&s); if (token(&s)) { json_decref(value); set_error(&s, "<format>", json_error_invalid_format, "Garbage after format string"); return NULL; (`c_src/src/pack_unpack.c:850`) | returns `NULL` | [x] |
| 127 | `unpack` | va_list ap_copy; if (!root) { jsonp_error_init(error, "<root>"); jsonp_error_set(error, -1, -1, 0, json_error_null_value, "NULL root value"); return -1; (`c_src/src/pack_unpack.c:886`) | returns `-1` | [x] |
| 128 | `unpack` | if (!fmt \|\| !*fmt) { jsonp_error_init(error, "<format>"); jsonp_error_set(error, -1, -1, 0, json_error_invalid_argument, "NULL or empty format string"); return -1; (`c_src/src/pack_unpack.c:893`) | returns `-1` | [x] |
| 129 | `unpack` | next_token(&s); va_copy(ap_copy, ap); if (unpack(&s, root, &ap_copy)) { va_end(ap_copy); return -1; (`c_src/src/pack_unpack.c:903`) | returns `-1` | [x] |
| 130 | `unpack` | next_token(&s); if (token(&s)) { set_error(&s, "<format>", json_error_invalid_format, "Garbage after format string"); return -1; (`c_src/src/pack_unpack.c:911`) | returns `-1` | [x] |
| 131 | `strbuffer_init` | strbuff->size = STRBUFFER_MIN_SIZE; strbuff->length = 0; strbuff->value = jsonp_malloc(strbuff->size); if (!strbuff->value) return -1; (`c_src/src/strbuffer.c:27`) | returns `-1` | [x] |
| 132 | `strbuffer_append_bytes` | if (strbuff->size > STRBUFFER_SIZE_MAX / STRBUFFER_FACTOR \|\| size > STRBUFFER_SIZE_MAX - 1 \|\| strbuff->length > STRBUFFER_SIZE_MAX - 1 - size) return -1; (`c_src/src/strbuffer.c:69`) | returns `-1` | [x] |
| 133 | `strbuffer_append_bytes` | new_size = max(strbuff->size * STRBUFFER_FACTOR, strbuff->length + size + 1); new_value = jsonp_realloc(strbuff->value, strbuff->size, new_size); if (!new_value) return -1; (`c_src/src/strbuffer.c:75`) | returns `-1` | [x] |
| 134 | `jsonp_strtod` | to_locale(strbuffer); errno = 0; value = strtod(strbuffer->value, &end); assert(end == strbuffer->value + strbuffer->length); (`c_src/src/strconv.c:53`) | process aborts when assertion is enabled | [x] |
| 135 | `jsonp_strtod` | value = strtod(strbuffer->value, &end); assert(end == strbuffer->value + strbuffer->length); if ((value == HUGE_VAL \|\| value == -HUGE_VAL) && errno == ERANGE) { return -1; (`c_src/src/strconv.c:57`) | returns `-1` | [x] |
| 136 | `jsonp_dtostr` | int digits_len, vdigits_start, vdigits_end; char *p; if (dtoa_r(value, mode, precision, &decpt, &sign, &digits_end, digits, 25) == NULL) { return -1; (`c_src/src/strconv.c:82`) | returns `-1` | [x] |
| 137 | `jsonp_dtostr` | (vdigits_end - vdigits_start) + (use_exp ? 5 : 0)) > size) { return -1; (`c_src/src/strconv.c:111`) | returns `-1` | [x] |
| 138 | `jsonp_dtostr` | if (precision == 0) precision = 17; ret = snprintf(buffer, size, "%.*g", precision, value); if (ret < 0) return -1; (`c_src/src/strconv.c:195`) | returns `-1` | [x] |
| 139 | `jsonp_dtostr` | if (ret < 0) return -1; length = (size_t)ret; if (length >= size) return -1; (`c_src/src/strconv.c:199`) | returns `-1` | [x] |
| 140 | `jsonp_dtostr` | a real is converted to an integer when decoding */ if (strchr(buffer, '.') == NULL && strchr(buffer, 'e') == NULL) { if (length + 3 >= size) { return -1; (`c_src/src/strconv.c:208`) | returns `-1` | [x] |
| 141 | `utf8_encode` | #include "utf.h" #include <string.h> int utf8_encode(int32_t codepoint, char *buffer, size_t *size) { if (codepoint < 0) return -1; (`c_src/src/utf.c:13`) | returns `-1` | [x] |
| 142 | `utf8_encode` | buffer[1] = 0x80 + ((codepoint & 0x03F000) >> 12); buffer[2] = 0x80 + ((codepoint & 0x000FC0) >> 6); buffer[3] = 0x80 + ((codepoint & 0x00003F)); } else return -1; (`c_src/src/utf.c:33`) | returns `-1` | [x] |
| 143 | `utf8_iterate` | if (!bufsize) return buffer; count = utf8_check_first(buffer[0]); if (count <= 0) return NULL; (`c_src/src/utf.c:125`) | returns `NULL` | [x] |
| 144 | `utf8_iterate` | if (count == 1) value = (unsigned char)buffer[0]; else { if (count > bufsize \|\| !utf8_check_full(buffer, count, &value)) return NULL; (`c_src/src/utf.c:131`) | returns `NULL` | [x] |
| 145 | `jsonp_loop_check` | if (key_len_out) if (hashtable_get(parents, key, key_len)) return -1; (`c_src/src/value.c:55`) | returns `-1` | [x] |
| 146 | `json_object` | extern volatile uint32_t hashtable_seed; json_t *json_object(void) { json_object_t *object = jsonp_malloc(sizeof(json_object_t)); if (!object) return NULL; (`c_src/src/value.c:67`) | returns `NULL` | [x] |
| 147 | `json_object` | json_init(&object->json, JSON_OBJECT); if (hashtable_init(&object->hashtable)) { jsonp_free(object); return NULL; (`c_src/src/value.c:78`) | returns `NULL` | [x] |
| 148 | `json_object_get` | return object->hashtable.size; } json_t *json_object_get(const json_t *json, const char *key) { if (!key) return NULL; (`c_src/src/value.c:101`) | returns `NULL` | [x] |
| 149 | `json_object_getn` | json_t *json_object_getn(const json_t *json, const char *key, size_t key_len) { json_object_t *object; if (!key \|\| !json_is_object(json)) return NULL; (`c_src/src/value.c:110`) | returns `NULL` | [x] |
| 150 | `json_object_set_new_nocheck` | } int json_object_set_new_nocheck(json_t *json, const char *key, json_t *value) { if (!key) { json_decref(value); return -1; (`c_src/src/value.c:119`) | returns `-1` | [x] |
| 151 | `json_object_setn_new_nocheck` | int json_object_setn_new_nocheck(json_t *json, const char *key, size_t key_len, json_t *value) { json_object_t *object; if (!value) return -1; (`c_src/src/value.c:129`) | returns `-1` | [x] |
| 152 | `json_object_setn_new_nocheck` | if (!value) return -1; if (!key \|\| !json_is_object(json) \|\| json == value) { json_decref(value); return -1; (`c_src/src/value.c:133`) | returns `-1` | [x] |
| 153 | `json_object_setn_new_nocheck` | } object = json_to_object(json); if (hashtable_set(&object->hashtable, key, key_len, value)) { json_decref(value); return -1; (`c_src/src/value.c:139`) | returns `-1` | [x] |
| 154 | `json_object_set_new` | } int json_object_set_new(json_t *json, const char *key, json_t *value) { if (!key) { json_decref(value); return -1; (`c_src/src/value.c:148`) | returns `-1` | [x] |
| 155 | `json_object_setn_new` | } int json_object_setn_new(json_t *json, const char *key, size_t key_len, json_t *value) { if (!key \|\| !utf8_check_string(key, key_len)) { json_decref(value); return -1; (`c_src/src/value.c:157`) | returns `-1` | [x] |
| 156 | `json_object_del` | return json_object_setn_new_nocheck(json, key, key_len, value); } int json_object_del(json_t *json, const char *key) { if (!key) return -1; (`c_src/src/value.c:165`) | returns `-1` | [x] |
| 157 | `json_object_deln` | int json_object_deln(json_t *json, const char *key, size_t key_len) { json_object_t *object; if (!key \|\| !json_is_object(json)) return -1; (`c_src/src/value.c:174`) | returns `-1` | [x] |
| 158 | `json_object_clear` | int json_object_clear(json_t *json) { json_object_t *object; if (!json_is_object(json)) return -1; (`c_src/src/value.c:184`) | returns `-1` | [x] |
| 159 | `json_object_update` | const char *key; size_t key_len; json_t *value; if (!json_is_object(object) \|\| !json_is_object(other)) return -1; (`c_src/src/value.c:198`) | returns `-1` | [x] |
| 160 | `json_object_update` | if (!json_is_object(object) \|\| !json_is_object(other)) return -1; json_object_keylen_foreach(other, key, key_len, value) { if (json_object_setn_nocheck(object, key, key_len, value)) return -1; (`c_src/src/value.c:202`) | returns `-1` | [x] |
| 161 | `json_object_update_existing` | const char *key; size_t key_len; json_t *value; if (!json_is_object(object) \|\| !json_is_object(other)) return -1; (`c_src/src/value.c:214`) | returns `-1` | [x] |
| 162 | `json_object_update_missing` | const char *key; size_t key_len; json_t *value; if (!json_is_object(object) \|\| !json_is_object(other)) return -1; (`c_src/src/value.c:230`) | returns `-1` | [x] |
| 163 | `do_object_update_recursive` | char loop_key[LOOP_KEY_LEN]; int res = 0; size_t loop_key_len; if (!json_is_object(object) \|\| !json_is_object(other)) return -1; (`c_src/src/value.c:249`) | returns `-1` | [x] |
| 164 | `do_object_update_recursive` | if (!json_is_object(object) \|\| !json_is_object(other)) return -1; if (jsonp_loop_check(parents, other, loop_key, sizeof(loop_key), &loop_key_len)) return -1; (`c_src/src/value.c:252`) | returns `-1` | [x] |
| 165 | `json_object_update_recursive` | int json_object_update_recursive(json_t *object, json_t *other) { int res; hashtable_t parents_set; if (hashtable_init(&parents_set)) return -1; (`c_src/src/value.c:280`) | returns `-1` | [x] |
| 166 | `json_object_iter` | void *json_object_iter(json_t *json) { json_object_t *object; if (!json_is_object(json)) return NULL; (`c_src/src/value.c:291`) | returns `NULL` | [x] |
| 167 | `json_object_iter_at` | void *json_object_iter_at(json_t *json, const char *key) { json_object_t *object; if (!key \|\| !json_is_object(json)) return NULL; (`c_src/src/value.c:301`) | returns `NULL` | [x] |
| 168 | `json_object_iter_next` | void *json_object_iter_next(json_t *json, void *iter) { json_object_t *object; if (!json_is_object(json) \|\| iter == NULL) return NULL; (`c_src/src/value.c:311`) | returns `NULL` | [x] |
| 169 | `json_object_iter_key` | return hashtable_iter_next(&object->hashtable, iter); } const char *json_object_iter_key(void *iter) { if (!iter) return NULL; (`c_src/src/value.c:319`) | returns `NULL` | [x] |
| 170 | `json_object_iter_value` | return hashtable_iter_key_len(iter); } json_t *json_object_iter_value(void *iter) { if (!iter) return NULL; (`c_src/src/value.c:333`) | returns `NULL` | [x] |
| 171 | `json_object_iter_set_new` | } int json_object_iter_set_new(json_t *json, void *iter, json_t *value) { if (!json_is_object(json) \|\| !iter \|\| !value) { json_decref(value); return -1; (`c_src/src/value.c:341`) | returns `-1` | [x] |
| 172 | `json_object_key_to_iter` | return 0; } void *json_object_key_to_iter(const char *key) { if (!key) return NULL; (`c_src/src/value.c:350`) | returns `NULL` | [x] |
| 173 | `json_object_copy` | size_t key_len; json_t *value; result = json_object(); if (!result) return NULL; (`c_src/src/value.c:382`) | returns `NULL` | [x] |
| 174 | `json_object_deep_copy` | void *iter; char loop_key[LOOP_KEY_LEN]; size_t loop_key_len; if (jsonp_loop_check(parents, object, loop_key, sizeof(loop_key), &loop_key_len)) return NULL; (`c_src/src/value.c:397`) | returns `NULL` | [x] |
| 175 | `json_array` | json_t *json_array(void) { json_array_t *array = jsonp_malloc(sizeof(json_array_t)); if (!array) return NULL; (`c_src/src/value.c:434`) | returns `NULL` | [x] |
| 176 | `json_array` | array->size = 8; array->table = jsonp_malloc(array->size * sizeof(json_t *)); if (!array->table) { jsonp_free(array); return NULL; (`c_src/src/value.c:443`) | returns `NULL` | [x] |
| 177 | `json_array_get` | } json_t *json_array_get(const json_t *json, size_t index) { json_array_t *array; if (!json_is_array(json)) return NULL; (`c_src/src/value.c:469`) | returns `NULL` | [x] |
| 178 | `json_array_get` | if (!json_is_array(json)) return NULL; array = json_to_array(json); if (index >= array->entries) return NULL; (`c_src/src/value.c:473`) | returns `NULL` | [x] |
| 179 | `json_array_set_new` | int json_array_set_new(json_t *json, size_t index, json_t *value) { json_array_t *array; if (!value) return -1; (`c_src/src/value.c:482`) | returns `-1` | [x] |
| 180 | `json_array_set_new` | if (!value) return -1; if (!json_is_array(json) \|\| json == value) { json_decref(value); return -1; (`c_src/src/value.c:486`) | returns `-1` | [x] |
| 181 | `json_array_set_new` | } array = json_to_array(json); if (index >= array->entries) { json_decref(value); return -1; (`c_src/src/value.c:492`) | returns `-1` | [x] |
| 182 | `json_array_grow` | new_size = max(array->size + amount, array->size * 2); new_table = jsonp_realloc(old_table, array->size * sizeof(json_t *), new_size * sizeof(json_t *)); if (!new_table) return NULL; (`c_src/src/value.c:523`) | returns `NULL` | [x] |
| 183 | `json_array_append_new` | int json_array_append_new(json_t *json, json_t *value) { json_array_t *array; if (!value) return -1; (`c_src/src/value.c:535`) | returns `-1` | [x] |
| 184 | `json_array_append_new` | if (!value) return -1; if (!json_is_array(json) \|\| json == value) { json_decref(value); return -1; (`c_src/src/value.c:539`) | returns `-1` | [x] |
| 185 | `json_array_append_new` | } array = json_to_array(json); if (!json_array_grow(array, 1)) { json_decref(value); return -1; (`c_src/src/value.c:545`) | returns `-1` | [x] |
| 186 | `json_array_insert_new` | int json_array_insert_new(json_t *json, size_t index, json_t *value) { json_array_t *array; if (!value) return -1; (`c_src/src/value.c:558`) | returns `-1` | [x] |
| 187 | `json_array_insert_new` | if (!value) return -1; if (!json_is_array(json) \|\| json == value) { json_decref(value); return -1; (`c_src/src/value.c:562`) | returns `-1` | [x] |
| 188 | `json_array_insert_new` | } array = json_to_array(json); if (index > array->entries) { json_decref(value); return -1; (`c_src/src/value.c:568`) | returns `-1` | [x] |
| 189 | `json_array_insert_new` | return -1; } if (!json_array_grow(array, 1)) { json_decref(value); return -1; (`c_src/src/value.c:573`) | returns `-1` | [x] |
| 190 | `json_array_remove` | int json_array_remove(json_t *json, size_t index) { json_array_t *array; if (!json_is_array(json)) return -1; (`c_src/src/value.c:588`) | returns `-1` | [x] |
| 191 | `json_array_remove` | if (!json_is_array(json)) return -1; array = json_to_array(json); if (index >= array->entries) return -1; (`c_src/src/value.c:592`) | returns `-1` | [x] |
| 192 | `json_array_clear` | int json_array_clear(json_t *json) { json_array_t *array; size_t i; if (!json_is_array(json)) return -1; (`c_src/src/value.c:610`) | returns `-1` | [x] |
| 193 | `json_array_extend` | int json_array_extend(json_t *json, json_t *other_json) { json_array_t *array, *other; size_t i; if (!json_is_array(json) \|\| !json_is_array(other_json)) return -1; (`c_src/src/value.c:625`) | returns `-1` | [x] |
| 194 | `json_array_extend` | return -1; array = json_to_array(json); other = json_to_array(other_json); if (!json_array_grow(array, other->entries)) return -1; (`c_src/src/value.c:630`) | returns `-1` | [x] |
| 195 | `json_array_copy` | json_t *result; size_t i; result = json_array(); if (!result) return NULL; (`c_src/src/value.c:667`) | returns `NULL` | [x] |
| 196 | `json_array_deep_copy` | size_t i; char loop_key[LOOP_KEY_LEN]; size_t loop_key_len; if (jsonp_loop_check(parents, array, loop_key, sizeof(loop_key), &loop_key_len)) return NULL; (`c_src/src/value.c:682`) | returns `NULL` | [x] |
| 197 | `string_create` | static json_t *string_create(const char *value, size_t len, int own) { char *v; json_string_t *string; if (!value) return NULL; (`c_src/src/value.c:710`) | returns `NULL` | [x] |
| 198 | `string_create` | if (own) v = (char *)value; else { v = jsonp_strndup(value, len); if (!v) return NULL; (`c_src/src/value.c:717`) | returns `NULL` | [x] |
| 199 | `string_create` | } string = jsonp_malloc(sizeof(json_string_t)); if (!string) { jsonp_free(v); return NULL; (`c_src/src/value.c:723`) | returns `NULL` | [x] |
| 200 | `json_string_nocheck` | return &string->json; } json_t *json_string_nocheck(const char *value) { if (!value) return NULL; (`c_src/src/value.c:734`) | returns `NULL` | [x] |
| 201 | `json_string` | return string_create(value, len, 1); } json_t *json_string(const char *value) { if (!value) return NULL; (`c_src/src/value.c:750`) | returns `NULL` | [x] |
| 202 | `json_stringn` | return json_stringn(value, strlen(value)); } json_t *json_stringn(const char *value, size_t len) { if (!value \|\| !utf8_check_string(value, len)) return NULL; (`c_src/src/value.c:757`) | returns `NULL` | [x] |
| 203 | `json_string_value` | return json_stringn_nocheck(value, len); } const char *json_string_value(const json_t *json) { if (!json_is_string(json)) return NULL; (`c_src/src/value.c:764`) | returns `NULL` | [x] |
| 204 | `json_string_set_nocheck` | return json_to_string(json)->length; } int json_string_set_nocheck(json_t *json, const char *value) { if (!value) return -1; (`c_src/src/value.c:778`) | returns `-1` | [x] |
| 205 | `json_string_setn_nocheck` | int json_string_setn_nocheck(json_t *json, const char *value, size_t len) { char *dup; json_string_t *string; if (!json_is_string(json) \|\| !value) return -1; (`c_src/src/value.c:788`) | returns `-1` | [x] |
| 206 | `json_string_setn_nocheck` | if (!json_is_string(json) \|\| !value) return -1; dup = jsonp_strndup(value, len); if (!dup) return -1; (`c_src/src/value.c:792`) | returns `-1` | [x] |
| 207 | `json_string_set` | return 0; } int json_string_set(json_t *json, const char *value) { if (!value) return -1; (`c_src/src/value.c:804`) | returns `-1` | [x] |
| 208 | `json_string_setn` | return json_string_setn(json, value, strlen(value)); } int json_string_setn(json_t *json, const char *value, size_t len) { if (!value \|\| !utf8_check_string(value, len)) return -1; (`c_src/src/value.c:811`) | returns `-1` | [x] |
| 209 | `json_integer` | json_t *json_integer(json_int_t value) { json_integer_t *integer = jsonp_malloc(sizeof(json_integer_t)); if (!integer) return NULL; (`c_src/src/value.c:884`) | returns `NULL` | [x] |
| 210 | `json_integer_set` | return json_to_integer(json)->value; } int json_integer_set(json_t *json, json_int_t value) { if (!json_is_integer(json)) return -1; (`c_src/src/value.c:900`) | returns `-1` | [x] |
| 211 | `json_real` | json_t *json_real(double value) { json_real_t *real; if (isnan(value) \|\| isinf(value)) return NULL; (`c_src/src/value.c:923`) | returns `NULL` | [x] |
| 212 | `json_real` | if (isnan(value) \|\| isinf(value)) return NULL; real = jsonp_malloc(sizeof(json_real_t)); if (!real) return NULL; (`c_src/src/value.c:927`) | returns `NULL` | [x] |
| 213 | `json_real_set` | return json_to_real(json)->value; } int json_real_set(json_t *json, double value) { if (!json_is_real(json) \|\| isnan(value) \|\| isinf(value)) return -1; (`c_src/src/value.c:943`) | returns `-1` | [x] |
| 214 | `json_copy` | json_t *json_copy(json_t *json) { if (!json) return NULL; (`c_src/src/value.c:1050`) | returns `NULL` | [x] |
| 215 | `json_copy` | case JSON_TRUE: case JSON_FALSE: case JSON_NULL: return json; default: return NULL; (`c_src/src/value.c:1068`) | returns `NULL` | [x] |
| 216 | `json_deep_copy` | json_t *json_deep_copy(const json_t *json) { json_t *res; hashtable_t parents_set; if (hashtable_init(&parents_set)) return NULL; (`c_src/src/value.c:1077`) | returns `NULL` | [x] |
| 217 | `do_deep_copy` | return res; } json_t *do_deep_copy(const json_t *json, hashtable_t *parents) { if (!json) return NULL; (`c_src/src/value.c:1086`) | returns `NULL` | [x] |
| 218 | `do_deep_copy` | case JSON_TRUE: case JSON_FALSE: case JSON_NULL: return (json_t *)json; default: return NULL; (`c_src/src/value.c:1106`) | returns `NULL` | [x] |
