import sys
MUTS = {
 "M1_hash_string_const":      ("hash = hash.wrapping_mul(21);", "hash = hash.wrapping_mul(22);"),
 "M2_used_count_threshold":   ("(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2);",
                               "(*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 1);"),
 "M3_siphash_tail_signext":   ("data |= (((*d.wrapping_add(3) as u32) << 24) as i32) as usize;",
                               "data |= (*d.wrapping_add(3) as usize) << 24;"),
 "M4_wrap_loop_tempkey":      ("                            // NOTE: unlike the loop above, the original C does\n                            // *not* update stbds_temp_key here.\n                            set_stbds_temp(a, (*bucket).index[i]);",
                               "                            set_stbds_temp(a, (*bucket).index[i]);\n                            if mode >= STBDS_HM_STRING {\n                                let k = *(elem_at(raw_a, elemsize, (*bucket).index[i] as usize, keyoffset) as *mut *mut c_char);\n                                set_stbds_temp_key(a, k);\n                            }"),
 "M5_arrgrowf_min4":          ("} else if min_cap < 4 {", "} else if min_cap < 5 {"),
 "M6_final_index":            ("let final_index: isize = stbds_arrlen(raw_a) - 1 - 1;",
                               "let final_index: isize = stbds_arrlen(raw_a) - 1;"),
 "M7_strkey_prefix":          ('for &c in b"test_" {', 'for &c in b"tesT_" {'),
 "M8_stralloc_blocksize":     ("blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);",
                               "blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << blocksize;"),
 "M9_probe_position":         ("hash & (slot_count.wrapping_sub(1))", "(hash >> 1) & (slot_count.wrapping_sub(1))"),
 "M10_hash_string_rot":       ("hash = rotate_left(hash, 9).wrapping_add(*s as usize);",
                               "hash = rotate_left(hash, 8).wrapping_add(*s as usize);"),
 "M11_tombstone_thresh":      ("(*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 4);",
                               "(*t).tombstone_count_threshold = (slot_count >> 3).wrapping_add(slot_count >> 5);"),
 "M12_hash_seed_mult":        ("let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);",
                               "let a = stbds_load_32_or_64(2147001325, 0x27bb2ee7, 0x87b0b0fd);"),
 "M13_shmode_cast":           ("(*h).string.mode = mode as u8;", "(*h).string.mode = (mode & 0x7f) as u8;"),
 "M15_sipround_rot":          ("$v3 = rotate_left($v3, 21);", "$v3 = rotate_left($v3, 20);"),
 "M16_shrink_threshold":      ("(*t).used_count_shrink_threshold = slot_count >> 2;",
                               "(*t).used_count_shrink_threshold = slot_count >> 3;"),
 "M17_hash_lt2":              ("if hash < 2 {\n            hash = hash.wrapping_add(2);\n        }\n\n        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);\n\n        loop {",
                               "if hash < 3 {\n            hash = hash.wrapping_add(2);\n        }\n\n        pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);\n\n        loop {"),
 "M18_step_growth":           ("step = step.wrapping_add(STBDS_BUCKET_LENGTH);\n            pos &= (*table).slot_count.wrapping_sub(1);",
                               "step = step.wrapping_add(2 * STBDS_BUCKET_LENGTH);\n            pos &= (*table).slot_count.wrapping_sub(1);"),
 "M19_strdup_len":            ("let len = strlen(str_).wrapping_add(1);\n        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;",
                               "let len = strlen(str_).wrapping_add(2);\n        let p = stbds_realloc(ptr::null_mut(), len) as *mut c_char;"),
 "M20_stralloc_ptr":          ("p = ((&raw mut (*(*a).storage).storage) as *mut c_char)\n            .wrapping_add((*a).remaining as isize as usize)\n            .wrapping_sub(len);",
                               "p = ((&raw mut (*(*a).storage).storage) as *mut c_char)\n            .wrapping_add((*a).remaining as isize as usize)\n            .wrapping_sub(len)\n            .wrapping_sub(0);\n        let _ = &p;"),
 "M21_align_fwd":             ("n.wrapping_add(a).wrapping_sub(1) & !(a - 1)", "n.wrapping_add(a) & !(a - 1)"),
 "M22_hmput_default_cond":    ("if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {",
                               "if a.is_null() {"),
 "M23_hmdel_temp":            ("set_stbds_temp(raw_a, 1);", "set_stbds_temp(raw_a, 2);"),
 "M24_siphash_c_rounds":      ("const STBDS_SIPHASH_C_ROUNDS: usize = 2;", "const STBDS_SIPHASH_C_ROUNDS: usize = 3;"),
 "M25_hmput_key_memcpy":      ("                        memcpy(\n                            elem_at(a, elemsize, i as usize, 0) as *mut c_void,\n                            key as *const c_void,\n                            keysize,\n                        );",
                               "                        memcpy(\n                            elem_at(a, elemsize, i as usize, 0) as *mut c_void,\n                            key as *const c_void,\n                            keysize.saturating_sub(1),\n                        );"),
}

MUTS["M18_step_growth"] = ("            step = step.wrapping_add(STBDS_BUCKET_LENGTH);\n            pos &= (*table).slot_count.wrapping_sub(1);",
                           "            step = step.wrapping_add(2 * STBDS_BUCKET_LENGTH);\n            pos &= (*table).slot_count.wrapping_sub(1);")
MUTS["M26_hmput_step_growth"] = ("                step = step.wrapping_add(STBDS_BUCKET_LENGTH);\n                pos &= (*table).slot_count.wrapping_sub(1);",
                                 "                step = step.wrapping_add(2 * STBDS_BUCKET_LENGTH);\n                pos &= (*table).slot_count.wrapping_sub(1);")
MUTS["M27_rehash_step_growth"] = ("                            step = step.wrapping_add(STBDS_BUCKET_LENGTH);\n                            pos &= (*t).slot_count.wrapping_sub(1);",
                                  "                            step = step.wrapping_add(2 * STBDS_BUCKET_LENGTH);\n                            pos &= (*t).slot_count.wrapping_sub(1);")
MUTS["M28_find_slot_wrap_limit"] = ("            let limit = pos & STBDS_BUCKET_MASK;\n            let mut i: usize = 0;\n            while i < limit {\n                if (*bucket).hash[i] == hash {",
                                    "            let limit = pos & STBDS_BUCKET_MASK;\n            let mut i: usize = 0;\n            while i + 1 < limit {\n                if (*bucket).hash[i] == hash {")
MUTS["M29_tombstone_pos"] = ("            if tombstone >= 0 {\n                pos = tombstone as usize;",
                             "            if tombstone > 0 {\n                pos = tombstone as usize;")
MUTS["M30_rehash_index"] = ("                                    (*bucket).index[z] = (*ob).index[j];\n                                    break 'done;\n                                }\n                                z += 1;\n                            }\n\n                            let limit = pos & STBDS_BUCKET_MASK;",
                            "                                    (*bucket).index[z] = (*ob).index[j];\n                                    break 'done;\n                                }\n                                z += 1;\n                            }\n\n                            let limit = (pos & STBDS_BUCKET_MASK).saturating_sub(1);")

name = sys.argv[1]
old, new = MUTS[name]
s = open("src/lib.rs").read()
if old not in s:
    print("MUT-NOT-FOUND"); sys.exit(2)
open("src/lib.rs","w").write(s.replace(old, new, 1))
print("MUT-APPLIED")
