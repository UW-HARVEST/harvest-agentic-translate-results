//! Generated layout cross-check against the C structures.
use crate::*;
use core::mem::{size_of, offset_of};

#[test]
fn dump_layout() {
    println!("STRUCT png_struct {}", size_of::<png_struct>());
    println!("S jmp_buf_local {}", offset_of!(png_struct, jmp_buf_local));
    println!("S longjmp_fn {}", offset_of!(png_struct, longjmp_fn));
    println!("S jmp_buf_ptr {}", offset_of!(png_struct, jmp_buf_ptr));
    println!("S jmp_buf_size {}", offset_of!(png_struct, jmp_buf_size));
    println!("S error_fn {}", offset_of!(png_struct, error_fn));
    println!("S warning_fn {}", offset_of!(png_struct, warning_fn));
    println!("S error_ptr {}", offset_of!(png_struct, error_ptr));
    println!("S write_data_fn {}", offset_of!(png_struct, write_data_fn));
    println!("S read_data_fn {}", offset_of!(png_struct, read_data_fn));
    println!("S io_ptr {}", offset_of!(png_struct, io_ptr));
    println!("S read_user_transform_fn {}", offset_of!(png_struct, read_user_transform_fn));
    println!("S write_user_transform_fn {}", offset_of!(png_struct, write_user_transform_fn));
    println!("S user_transform_ptr {}", offset_of!(png_struct, user_transform_ptr));
    println!("S user_transform_depth {}", offset_of!(png_struct, user_transform_depth));
    println!("S user_transform_channels {}", offset_of!(png_struct, user_transform_channels));
    println!("S mode {}", offset_of!(png_struct, mode));
    println!("S flags {}", offset_of!(png_struct, flags));
    println!("S transformations {}", offset_of!(png_struct, transformations));
    println!("S zowner {}", offset_of!(png_struct, zowner));
    println!("S zstream {}", offset_of!(png_struct, zstream));
    println!("S zbuffer_list {}", offset_of!(png_struct, zbuffer_list));
    println!("S zbuffer_size {}", offset_of!(png_struct, zbuffer_size));
    println!("S zlib_level {}", offset_of!(png_struct, zlib_level));
    println!("S zlib_method {}", offset_of!(png_struct, zlib_method));
    println!("S zlib_window_bits {}", offset_of!(png_struct, zlib_window_bits));
    println!("S zlib_mem_level {}", offset_of!(png_struct, zlib_mem_level));
    println!("S zlib_strategy {}", offset_of!(png_struct, zlib_strategy));
    println!("S zlib_text_level {}", offset_of!(png_struct, zlib_text_level));
    println!("S zlib_text_method {}", offset_of!(png_struct, zlib_text_method));
    println!("S zlib_text_window_bits {}", offset_of!(png_struct, zlib_text_window_bits));
    println!("S zlib_text_mem_level {}", offset_of!(png_struct, zlib_text_mem_level));
    println!("S zlib_text_strategy {}", offset_of!(png_struct, zlib_text_strategy));
    println!("S zlib_set_level {}", offset_of!(png_struct, zlib_set_level));
    println!("S zlib_set_method {}", offset_of!(png_struct, zlib_set_method));
    println!("S zlib_set_window_bits {}", offset_of!(png_struct, zlib_set_window_bits));
    println!("S zlib_set_mem_level {}", offset_of!(png_struct, zlib_set_mem_level));
    println!("S zlib_set_strategy {}", offset_of!(png_struct, zlib_set_strategy));
    println!("S chunks {}", offset_of!(png_struct, chunks));
    println!("S width {}", offset_of!(png_struct, width));
    println!("S height {}", offset_of!(png_struct, height));
    println!("S num_rows {}", offset_of!(png_struct, num_rows));
    println!("S usr_width {}", offset_of!(png_struct, usr_width));
    println!("S rowbytes {}", offset_of!(png_struct, rowbytes));
    println!("S iwidth {}", offset_of!(png_struct, iwidth));
    println!("S row_number {}", offset_of!(png_struct, row_number));
    println!("S chunk_name {}", offset_of!(png_struct, chunk_name));
    println!("S prev_row {}", offset_of!(png_struct, prev_row));
    println!("S row_buf {}", offset_of!(png_struct, row_buf));
    println!("S try_row {}", offset_of!(png_struct, try_row));
    println!("S tst_row {}", offset_of!(png_struct, tst_row));
    println!("S info_rowbytes {}", offset_of!(png_struct, info_rowbytes));
    println!("S idat_size {}", offset_of!(png_struct, idat_size));
    println!("S crc {}", offset_of!(png_struct, crc));
    println!("S palette {}", offset_of!(png_struct, palette));
    println!("S num_palette {}", offset_of!(png_struct, num_palette));
    println!("S num_palette_max {}", offset_of!(png_struct, num_palette_max));
    println!("S num_trans {}", offset_of!(png_struct, num_trans));
    println!("S compression {}", offset_of!(png_struct, compression));
    println!("S filter {}", offset_of!(png_struct, filter));
    println!("S interlaced {}", offset_of!(png_struct, interlaced));
    println!("S pass {}", offset_of!(png_struct, pass));
    println!("S do_filter {}", offset_of!(png_struct, do_filter));
    println!("S color_type {}", offset_of!(png_struct, color_type));
    println!("S bit_depth {}", offset_of!(png_struct, bit_depth));
    println!("S usr_bit_depth {}", offset_of!(png_struct, usr_bit_depth));
    println!("S pixel_depth {}", offset_of!(png_struct, pixel_depth));
    println!("S channels {}", offset_of!(png_struct, channels));
    println!("S usr_channels {}", offset_of!(png_struct, usr_channels));
    println!("S sig_bytes {}", offset_of!(png_struct, sig_bytes));
    println!("S maximum_pixel_depth {}", offset_of!(png_struct, maximum_pixel_depth));
    println!("S transformed_pixel_depth {}", offset_of!(png_struct, transformed_pixel_depth));
    println!("S zstream_start {}", offset_of!(png_struct, zstream_start));
    println!("S filler {}", offset_of!(png_struct, filler));
    println!("S background_gamma_type {}", offset_of!(png_struct, background_gamma_type));
    println!("S background_gamma {}", offset_of!(png_struct, background_gamma));
    println!("S background {}", offset_of!(png_struct, background));
    println!("S background_1 {}", offset_of!(png_struct, background_1));
    println!("S output_flush_fn {}", offset_of!(png_struct, output_flush_fn));
    println!("S flush_dist {}", offset_of!(png_struct, flush_dist));
    println!("S flush_rows {}", offset_of!(png_struct, flush_rows));
    println!("S chromaticities {}", offset_of!(png_struct, chromaticities));
    println!("S gamma_shift {}", offset_of!(png_struct, gamma_shift));
    println!("S screen_gamma {}", offset_of!(png_struct, screen_gamma));
    println!("S file_gamma {}", offset_of!(png_struct, file_gamma));
    println!("S chunk_gamma {}", offset_of!(png_struct, chunk_gamma));
    println!("S default_gamma {}", offset_of!(png_struct, default_gamma));
    println!("S gamma_table {}", offset_of!(png_struct, gamma_table));
    println!("S gamma_16_table {}", offset_of!(png_struct, gamma_16_table));
    println!("S gamma_from_1 {}", offset_of!(png_struct, gamma_from_1));
    println!("S gamma_to_1 {}", offset_of!(png_struct, gamma_to_1));
    println!("S gamma_16_from_1 {}", offset_of!(png_struct, gamma_16_from_1));
    println!("S gamma_16_to_1 {}", offset_of!(png_struct, gamma_16_to_1));
    println!("S sig_bit {}", offset_of!(png_struct, sig_bit));
    println!("S shift {}", offset_of!(png_struct, shift));
    println!("S trans_alpha {}", offset_of!(png_struct, trans_alpha));
    println!("S trans_color {}", offset_of!(png_struct, trans_color));
    println!("S read_row_fn {}", offset_of!(png_struct, read_row_fn));
    println!("S write_row_fn {}", offset_of!(png_struct, write_row_fn));
    println!("S info_fn {}", offset_of!(png_struct, info_fn));
    println!("S row_fn {}", offset_of!(png_struct, row_fn));
    println!("S end_fn {}", offset_of!(png_struct, end_fn));
    println!("S save_buffer_ptr {}", offset_of!(png_struct, save_buffer_ptr));
    println!("S save_buffer {}", offset_of!(png_struct, save_buffer));
    println!("S current_buffer_ptr {}", offset_of!(png_struct, current_buffer_ptr));
    println!("S current_buffer {}", offset_of!(png_struct, current_buffer));
    println!("S push_length {}", offset_of!(png_struct, push_length));
    println!("S skip_length {}", offset_of!(png_struct, skip_length));
    println!("S save_buffer_size {}", offset_of!(png_struct, save_buffer_size));
    println!("S save_buffer_max {}", offset_of!(png_struct, save_buffer_max));
    println!("S buffer_size {}", offset_of!(png_struct, buffer_size));
    println!("S current_buffer_size {}", offset_of!(png_struct, current_buffer_size));
    println!("S process_mode {}", offset_of!(png_struct, process_mode));
    println!("S cur_palette {}", offset_of!(png_struct, cur_palette));
    println!("S palette_lookup {}", offset_of!(png_struct, palette_lookup));
    println!("S quantize_index {}", offset_of!(png_struct, quantize_index));
    println!("S options {}", offset_of!(png_struct, options));
    println!("S time_buffer {}", offset_of!(png_struct, time_buffer));
    println!("S free_me {}", offset_of!(png_struct, free_me));
    println!("S user_chunk_ptr {}", offset_of!(png_struct, user_chunk_ptr));
    println!("S read_user_chunk_fn {}", offset_of!(png_struct, read_user_chunk_fn));
    println!("S unknown_default {}", offset_of!(png_struct, unknown_default));
    println!("S num_chunk_list {}", offset_of!(png_struct, num_chunk_list));
    println!("S chunk_list {}", offset_of!(png_struct, chunk_list));
    println!("S rgb_to_gray_status {}", offset_of!(png_struct, rgb_to_gray_status));
    println!("S rgb_to_gray_coefficients_set {}", offset_of!(png_struct, rgb_to_gray_coefficients_set));
    println!("S rgb_to_gray_red_coeff {}", offset_of!(png_struct, rgb_to_gray_red_coeff));
    println!("S rgb_to_gray_green_coeff {}", offset_of!(png_struct, rgb_to_gray_green_coeff));
    println!("S riffled_palette {}", offset_of!(png_struct, riffled_palette));
    println!("S mng_features_permitted {}", offset_of!(png_struct, mng_features_permitted));
    println!("S filter_type {}", offset_of!(png_struct, filter_type));
    println!("S mem_ptr {}", offset_of!(png_struct, mem_ptr));
    println!("S malloc_fn {}", offset_of!(png_struct, malloc_fn));
    println!("S free_fn {}", offset_of!(png_struct, free_fn));
    println!("S big_row_buf {}", offset_of!(png_struct, big_row_buf));
    println!("S index_to_palette {}", offset_of!(png_struct, index_to_palette));
    println!("S palette_to_index {}", offset_of!(png_struct, palette_to_index));
    println!("S compression_type {}", offset_of!(png_struct, compression_type));
    println!("S user_width_max {}", offset_of!(png_struct, user_width_max));
    println!("S user_height_max {}", offset_of!(png_struct, user_height_max));
    println!("S user_chunk_cache_max {}", offset_of!(png_struct, user_chunk_cache_max));
    println!("S user_chunk_malloc_max {}", offset_of!(png_struct, user_chunk_malloc_max));
    println!("S unknown_chunk {}", offset_of!(png_struct, unknown_chunk));
    println!("S old_big_row_buf_size {}", offset_of!(png_struct, old_big_row_buf_size));
    println!("S read_buffer {}", offset_of!(png_struct, read_buffer));
    println!("S read_buffer_size {}", offset_of!(png_struct, read_buffer_size));
    println!("S IDAT_read_size {}", offset_of!(png_struct, IDAT_read_size));
    println!("S io_state {}", offset_of!(png_struct, io_state));
    println!("S big_prev_row {}", offset_of!(png_struct, big_prev_row));
    println!("S read_filter {}", offset_of!(png_struct, read_filter));
    println!("STRUCT png_info {}", size_of::<png_info>());
    println!("I width {}", offset_of!(png_info, width));
    println!("I height {}", offset_of!(png_info, height));
    println!("I valid {}", offset_of!(png_info, valid));
    println!("I rowbytes {}", offset_of!(png_info, rowbytes));
    println!("I palette {}", offset_of!(png_info, palette));
    println!("I num_palette {}", offset_of!(png_info, num_palette));
    println!("I num_trans {}", offset_of!(png_info, num_trans));
    println!("I bit_depth {}", offset_of!(png_info, bit_depth));
    println!("I color_type {}", offset_of!(png_info, color_type));
    println!("I compression_type {}", offset_of!(png_info, compression_type));
    println!("I filter_type {}", offset_of!(png_info, filter_type));
    println!("I interlace_type {}", offset_of!(png_info, interlace_type));
    println!("I channels {}", offset_of!(png_info, channels));
    println!("I pixel_depth {}", offset_of!(png_info, pixel_depth));
    println!("I spare_byte {}", offset_of!(png_info, spare_byte));
    println!("I signature {}", offset_of!(png_info, signature));
    println!("I cicp_colour_primaries {}", offset_of!(png_info, cicp_colour_primaries));
    println!("I cicp_transfer_function {}", offset_of!(png_info, cicp_transfer_function));
    println!("I cicp_matrix_coefficients {}", offset_of!(png_info, cicp_matrix_coefficients));
    println!("I cicp_video_full_range_flag {}", offset_of!(png_info, cicp_video_full_range_flag));
    println!("I iccp_name {}", offset_of!(png_info, iccp_name));
    println!("I iccp_profile {}", offset_of!(png_info, iccp_profile));
    println!("I iccp_proflen {}", offset_of!(png_info, iccp_proflen));
    println!("I maxCLL {}", offset_of!(png_info, maxCLL));
    println!("I maxFALL {}", offset_of!(png_info, maxFALL));
    println!("I mastering_red_x {}", offset_of!(png_info, mastering_red_x));
    println!("I mastering_red_y {}", offset_of!(png_info, mastering_red_y));
    println!("I mastering_green_x {}", offset_of!(png_info, mastering_green_x));
    println!("I mastering_green_y {}", offset_of!(png_info, mastering_green_y));
    println!("I mastering_blue_x {}", offset_of!(png_info, mastering_blue_x));
    println!("I mastering_blue_y {}", offset_of!(png_info, mastering_blue_y));
    println!("I mastering_white_x {}", offset_of!(png_info, mastering_white_x));
    println!("I mastering_white_y {}", offset_of!(png_info, mastering_white_y));
    println!("I mastering_maxDL {}", offset_of!(png_info, mastering_maxDL));
    println!("I mastering_minDL {}", offset_of!(png_info, mastering_minDL));
    println!("I num_text {}", offset_of!(png_info, num_text));
    println!("I max_text {}", offset_of!(png_info, max_text));
    println!("I text {}", offset_of!(png_info, text));
    println!("I mod_time {}", offset_of!(png_info, mod_time));
    println!("I sig_bit {}", offset_of!(png_info, sig_bit));
    println!("I trans_alpha {}", offset_of!(png_info, trans_alpha));
    println!("I trans_color {}", offset_of!(png_info, trans_color));
    println!("I background {}", offset_of!(png_info, background));
    println!("I x_offset {}", offset_of!(png_info, x_offset));
    println!("I y_offset {}", offset_of!(png_info, y_offset));
    println!("I offset_unit_type {}", offset_of!(png_info, offset_unit_type));
    println!("I x_pixels_per_unit {}", offset_of!(png_info, x_pixels_per_unit));
    println!("I y_pixels_per_unit {}", offset_of!(png_info, y_pixels_per_unit));
    println!("I phys_unit_type {}", offset_of!(png_info, phys_unit_type));
    println!("I num_exif {}", offset_of!(png_info, num_exif));
    println!("I exif {}", offset_of!(png_info, exif));
    println!("I hist {}", offset_of!(png_info, hist));
    println!("I pcal_purpose {}", offset_of!(png_info, pcal_purpose));
    println!("I pcal_X0 {}", offset_of!(png_info, pcal_X0));
    println!("I pcal_X1 {}", offset_of!(png_info, pcal_X1));
    println!("I pcal_units {}", offset_of!(png_info, pcal_units));
    println!("I pcal_params {}", offset_of!(png_info, pcal_params));
    println!("I pcal_type {}", offset_of!(png_info, pcal_type));
    println!("I pcal_nparams {}", offset_of!(png_info, pcal_nparams));
    println!("I free_me {}", offset_of!(png_info, free_me));
    println!("I unknown_chunks {}", offset_of!(png_info, unknown_chunks));
    println!("I unknown_chunks_num {}", offset_of!(png_info, unknown_chunks_num));
    println!("I splt_palettes {}", offset_of!(png_info, splt_palettes));
    println!("I splt_palettes_num {}", offset_of!(png_info, splt_palettes_num));
    println!("I scal_unit {}", offset_of!(png_info, scal_unit));
    println!("I scal_s_width {}", offset_of!(png_info, scal_s_width));
    println!("I scal_s_height {}", offset_of!(png_info, scal_s_height));
    println!("I row_pointers {}", offset_of!(png_info, row_pointers));
    println!("I cHRM {}", offset_of!(png_info, cHRM));
    println!("I gamma {}", offset_of!(png_info, gamma));
    println!("I rendering_intent {}", offset_of!(png_info, rendering_intent));
    println!("STRUCT png_control {}", size_of::<png_control>());
    println!("C png_ptr {}", offset_of!(png_control, png_ptr));
    println!("C info_ptr {}", offset_of!(png_control, info_ptr));
    println!("C error_buf {}", offset_of!(png_control, error_buf));
    println!("C memory {}", offset_of!(png_control, memory));
    println!("C size {}", offset_of!(png_control, size));
    println!("STRUCT png_image {}", size_of::<png_image>());
    println!("STRUCT png_text {}", size_of::<png_text>());
    println!("STRUCT png_unknown_chunk {}", size_of::<png_unknown_chunk>());
    println!("STRUCT png_row_info {}", size_of::<png_row_info>());
    println!("STRUCT png_sPLT_t {}", size_of::<png_sPLT_t>());
    println!("STRUCT z_stream {}", size_of::<z_stream>());
    println!("STRUCT jmp_buf {}", size_of::<jmp_buf>());
    println!("STRUCT png_compression_buffer {}", size_of::<png_compression_buffer>());
    println!("STRUCT png_color_16 {}", size_of::<png_color_16>());
    println!("STRUCT png_color_8 {}", size_of::<png_color_8>());
    println!("STRUCT png_xy {}", size_of::<png_xy>());
    println!("STRUCT png_XYZ {}", size_of::<png_XYZ>());
    println!("STRUCT png_time {}", size_of::<png_time>());
}

/* ------------------------------------------------------------------------- */
/* The private setjmp/longjmp pair used for libpng's internal jmp_bufs.      */

static mut SJ_DEPTH: c_int = 0;

unsafe fn nested_jump(jb: *mut __jmp_buf_tag, depth: c_int) {
    SJ_DEPTH = depth;
    if depth > 0 {
        nested_jump(jb, depth - 1);
    }
    png_private_longjmp(jb, 7);
}

#[test]
fn setjmp_longjmp_roundtrip() {
    unsafe {
        let mut jb: jmp_buf = core::mem::zeroed();
        let mut visited = 0;
        let r = png_private_setjmp(jb.as_mut_ptr());
        if r == 0 {
            visited += 1;
            png_private_longjmp(jb.as_mut_ptr(), 42);
        }
        assert_eq!(r, 42);
        /* 'visited' lives in the restored stack frame, so (exactly as with the C
         * library's setjmp) its updated value survives the jump.
         */
        assert_eq!(visited, 1);
    }
}

#[test]
fn setjmp_longjmp_zero_becomes_one() {
    unsafe {
        let mut jb: jmp_buf = core::mem::zeroed();
        let r = png_private_setjmp(jb.as_mut_ptr());
        if r == 0 {
            png_private_longjmp(jb.as_mut_ptr(), 0);
        }
        assert_eq!(r, 1);
    }
}

#[test]
fn setjmp_longjmp_from_deep_frames() {
    unsafe {
        let mut jb: jmp_buf = core::mem::zeroed();
        let r = png_private_setjmp(jb.as_mut_ptr());
        if r == 0 {
            nested_jump(jb.as_mut_ptr(), 20);
        }
        assert_eq!(r, 7);
        assert_eq!(SJ_DEPTH, 0);
    }
}
