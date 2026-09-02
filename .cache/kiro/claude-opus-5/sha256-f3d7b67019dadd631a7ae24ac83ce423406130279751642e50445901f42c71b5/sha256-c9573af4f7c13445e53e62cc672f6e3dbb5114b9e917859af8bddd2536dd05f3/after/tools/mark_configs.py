#!/usr/bin/env python3
"""Fill in the CONFIGS.md check column with the test that covers each row.

The mapping is explicit (row id -> `tests/<file>.rs::<fn>`) so that the table can
be audited against `cargo test -- --list`.
"""
import re
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CFG = os.path.join(ROOT, 'translation/CONFIGS.md')

M = {}


def add(ids, test):
    for i in ids:
        M[i] = test


# ---- group L: tests/b_lowlevel.rs -----------------------------------------
add(['L1'], 'b_lowlevel.rs::version_strings_match')
add(['L2'], 'b_lowlevel.rs::l2_png_sig_cmp')
add(['L3', 'L4', 'L5', 'L7'], 'b_lowlevel.rs::l3_l7_int_functions')
add(['L6'], 'b_lowlevel.rs::l6_png_get_uint_31')
add(['L8'], 'b_lowlevel.rs::l8_build_grayscale_palette')
add(['L9', 'L10'], 'b_lowlevel.rs::l9_l11_time_conversion')
add(['L11'], 'b_lowlevel.rs::l11_convert_to_rfc1123_deprecated')
add(['L12'], 'b_lowlevel.rs::l12_png_muldiv')
add(['L13', 'L14', 'L15', 'L16', 'L17'], 'b_lowlevel.rs::l13_l17_reciprocal_and_gamma')
add(['L18'], 'b_lowlevel.rs::l18_png_gamma_correct')
add(['L19', 'L20', 'L21'], 'b_lowlevel.rs::l19_l21_xyz_conversion')
add(['L22', 'L23'], 'b_lowlevel.rs::l22_l23_fp_number_checks')
add(['L24', 'L25'], 'b_lowlevel.rs::l24_l25_safecat_and_format_number')
add(['L26', 'L27', 'L28', 'L29'], 'b_lowlevel.rs::l26_l29_ascii_and_fixed')
add(['L30'], 'b_lowlevel.rs::l30_check_keyword')
add(['L31'], 'b_lowlevel.rs::l31_crc')
add(['L32', 'L33', 'L34', 'L35', 'L36'], 'b_lowlevel.rs::l32_l35_row_transforms')
add(['L37', 'L38'], 'b_lowlevel.rs::l37_l38_interlace')
add(['L39'], 'b_lowlevel.rs::l39_read_filter_row')
add(['L40'], 'b_lowlevel.rs::l40_check_ihdr + l_errors3.rs::e1_check_ihdr_architecture_limits')
add(['L41'], 'b_lowlevel.rs::l41_icc_check_length')
add(['L42', 'L43'], 'b_lowlevel.rs::l42_l43_icc_check_header_and_tags')
add(['L44'], 'b_lowlevel.rs::l44_srgb_tables')
add(['L45'], 'b_lowlevel.rs::l45_zstream_error')
add(['L46'], 'b_lowlevel.rs::l46_l47_allocation + l_errors3.rs::e10_malloc_default')
add(['L47'], 'b_lowlevel.rs::l46_l47_allocation + k_errors2.rs::d7_size_limits_and_transform_combinations')
add(['L48', 'L49'], 'b_lowlevel.rs::l48_l49_create_struct_and_version_check')
add(['L50'], 'b_lowlevel.rs::l50_chunk_unknown_handling')

# ---- group W: tests/c_write.rs --------------------------------------------
add(['W%d' % i for i in range(1, 17)], 'c_write.rs::w1_w16_all_legal_shapes')
add(['W17', 'W18'], 'c_write.rs::w17_w18_write_rows_and_image')
add(['W19'], 'c_write.rs::w19_filters')
add(['W20', 'W21', 'W22', 'W23', 'W24', 'W25'], 'c_write.rs::w20_w25_zlib_parameters')
add(['W26'], 'c_write.rs::w26_text_compression_parameters')
add(['W27'], 'c_write.rs::w27_flush')
add(['W28', 'W29', 'W30', 'W31', 'W32', 'W33', 'W34', 'W35', 'W36', 'W37'],
    'c_write.rs::w28_w37_write_transforms')
add(['W38', 'W39'], 'c_write.rs::w38_w39_user_transform_and_status')
add(['W40'], 'c_write.rs::w40_mng_features')
add(['W41'], 'c_write.rs::w41_raw_chunk_api + l_errors3.rs::e3_chunk_length_maximum')
add(['W42'], 'c_write.rs::w42_invalid_index_check')
add(['W43'], 'c_write.rs::w43_set_option')
add(['W44'], 'c_write.rs::w44_write_info_before_plte')
add(['W45'], 'c_write.rs::w45_write_png_transforms')

# ---- group WC: tests/d_chunks.rs ------------------------------------------
add(['WC1'], 'd_chunks.rs::wc1_gama')
add(['WC2', 'WC3'], 'd_chunks.rs::wc2_wc3_chrm')
add(['WC4', 'WC5'], 'd_chunks.rs::wc4_wc5_srgb')
add(['WC6'], 'd_chunks.rs::wc6_iccp + l_errors3.rs::e4_write_iccp_lengths')
add(['WC7'], 'd_chunks.rs::wc7_sbit')
add(['WC8'], 'd_chunks.rs::wc8_bkgd')
add(['WC9'], 'd_chunks.rs::wc9_hist')
add(['WC10'], 'd_chunks.rs::wc10_trns')
add(['WC11', 'WC12'], 'd_chunks.rs::wc11_wc12_phys_offs')
add(['WC13'], 'd_chunks.rs::wc13_pcal')
add(['WC14'], 'd_chunks.rs::wc14_scal')
add(['WC15'], 'd_chunks.rs::wc15_time')
add(['WC16'], 'd_chunks.rs::wc16_splt')
add(['WC17', 'WC18', 'WC19', 'WC20'], 'd_chunks.rs::wc17_wc20_text')
add(['WC21'], 'd_chunks.rs::wc21_exif')
add(['WC22', 'WC23', 'WC24'], 'd_chunks.rs::wc22_wc24_pngv3_chunks')
add(['WC25', 'WC26'], 'd_chunks.rs::wc25_wc26_unknown_chunks')
add(['WC27'], 'd_chunks.rs::wc27_invalid_free_data')
add(['WC28'], 'd_chunks.rs::wc28_ihdr_accessors')

# ---- group R: tests/e_read.rs ---------------------------------------------
add(['R1', 'R2'], 'e_read.rs::r1_r2_all_legal_shapes')
add(['R3'], 'e_read.rs::r3_manual_interlace')
add(['R4', 'R5'], 'e_read.rs::r4_r5_read_image_and_rows')
add(['R6'], 'e_read.rs::r11_r29_read_transforms (read_rows_session always calls png_read_update_info after the transforms)')
add(['R7', 'R8'], 'e_read.rs::r7_r8_start_read_and_sig_bytes')
add(['R9'], 'e_read.rs::r9_crc_actions')
add(['R10'], 'e_read.rs::r10_user_limits')
add(['R11', 'R12', 'R13', 'R14', 'R15', 'R16', 'R18', 'R19', 'R20', 'R21', 'R22',
     'R23', 'R24', 'R25', 'R26', 'R27', 'R28', 'R29'],
    'e_read.rs::r11_r29_read_transforms')
add(['R17'], 'e_read.rs::r17_rgb_to_gray')
add(['R30'], 'e_read.rs::r30_gamma')
add(['R31'], 'e_read.rs::r31_background')
add(['R32'], 'e_read.rs::r32_alpha_mode')
add(['R33'], 'e_read.rs::r33_quantize')
add(['R34', 'R35'], 'e_read.rs::r34_r35_read_callbacks + l_errors3.rs::e16_user_transform_pixel_depth')
add(['R36'], 'e_read.rs::r36_user_chunk_callback + k_errors2.rs::d5_crafted_chunk_handlers')
add(['R37'], 'e_read.rs::r37_keep_unknown_on_read')
add(['R38', 'R39'], 'e_read.rs::r38_r39_benign_and_options')
add(['R40'], 'e_read.rs::r40_read_png_transforms')
add(['R41'], 'e_read.rs::r41_io_state')
add(['R42'], 'e_read.rs::r41_io_state + r43_all_ancillary_chunks + d_chunks.rs::wc11_wc12_phys_offs')
add(['R43'], 'e_read.rs::r43_all_ancillary_chunks')
add(['R44'], 'e_read.rs::r43_all_ancillary_chunks (calls png_reset_zstream after the read)')
add(['R45', 'R46', 'R47'], 'e_read.rs::r45_r47_stream_shapes')

# ---- group P: tests/f_progressive.rs -------------------------------------
add(['P1', 'P2'], 'f_progressive.rs::p1_p2_progressive_all_shapes')
add(['P3'], 'f_progressive.rs::p3_pause_and_resume')
add(['P4'], 'f_progressive.rs::p4_process_data_skip')
add(['P5'], 'f_progressive.rs::p1_p2_progressive_all_shapes (logs png_get_progressive_ptr)')
add(['P6'], 'f_progressive.rs::p6_progressive_transforms')
add(['P7'], 'f_progressive.rs::p7_progressive_with_chunks + l_errors3.rs::e7_progressive_idat_damage')

# ---- group S: tests/g_simplified.rs --------------------------------------
add(['S1', 'S2', 'S3', 'S4', 'S6'], 'g_simplified.rs::s1_s6_write_to_memory_all_formats')
add(['S5'], 'g_simplified.rs::s5_row_stride_variants')
add(['S7'], 'g_simplified.rs::s7_output_buffer_sizes + k_errors2.rs::d6_simplified_format_rejections')
add(['S8'], 'g_simplified.rs::s8_read_native_format')
add(['S9'], 'g_simplified.rs::s9_s12_read_with_format_override')
add(['S10'], 'g_simplified.rs::s9_s12_read_with_format_override + l_errors3.rs::e13_simplified_colormap_matrix')
add(['S11'], 'g_simplified.rs::s11_16bit_srgb_flag')
add(['S12'], 'g_simplified.rs::s9_s12_read_with_format_override')
add(['S13'], 'g_simplified.rs::s8_read_native_format (frees twice) + i_errors.rs::c11_simplified_api_errors')

# ---- group RT: tests/h_roundtrip.rs -------------------------------------
add(['RT1'], 'h_roundtrip.rs::rt1_rt4_write_read_roundtrip')
add(['RT2'], 'h_roundtrip.rs::rt2_random_write_configs')
add(['RT3'], 'e_read.rs::r43_all_ancillary_chunks')
add(['RT4'], 'h_roundtrip.rs::rt1_rt4_write_read_roundtrip (cross-implementation assertion)')
add(['RT5'], 'h_roundtrip.rs::rt5_write_png_read_png')
add(['RT6', 'RT7'], 'h_roundtrip.rs::rt6_rt7_simplified_roundtrip')
add(['RT8'], 'h_roundtrip.rs::rt8_write_then_progressive_read')

lines = open(CFG).read().split('\n')
out = []
seen = set()
row_re = re.compile(r'^\|\s*((?:L|WC|W|RT|R|P|S)\d+)\s*\|')
for ln in lines:
    m = row_re.match(ln)
    if m:
        rid = m.group(1)
        seen.add(rid)
        test = M.get(rid)
        if test is None:
            sys.stderr.write('NO TEST MAPPED FOR %s\n' % rid)
            out.append(ln)
            continue
        ln = re.sub(r'\|\s*\[ \]\s*\|\s*$', '| [x] `tests/%s` |' % test, ln)
        out.append(ln)
    else:
        # widen the header of each table
        ln = ln.replace('| configuration (options set + input shape) | [ ] |',
                        '| configuration (options set + input shape) | [x] covered by |')
        ln = ln.replace('|--------------------------------------------|-----|',
                        '|--------------------------------------------|----------------|')
        out.append(ln)

open(CFG, 'w').write('\n'.join(out))
missing = sorted(set(M) - seen)
if missing:
    sys.stderr.write('mapped but not present in CONFIGS.md: %s\n' % missing)
sys.stderr.write('rows checked off: %d\n' % len(seen))
