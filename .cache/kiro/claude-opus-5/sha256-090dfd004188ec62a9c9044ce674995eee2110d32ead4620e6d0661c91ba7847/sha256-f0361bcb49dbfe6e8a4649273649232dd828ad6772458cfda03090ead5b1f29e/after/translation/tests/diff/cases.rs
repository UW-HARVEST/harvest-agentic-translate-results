//! Case registry and dispatcher.
//!
//! Each case is identified by a stable name. `rows` records which `CONFIGS.md`
//! (`B<n>`) and `ERRORS.md` (`C<n>`) rows the case discharges, so the driver can
//! report coverage against the Phase A artifacts.

use crate::api::Api;

#[path = "cases/lowlevel.rs"]
pub mod lowlevel;
#[path = "cases/errpath.rs"]
pub mod errpath;
#[path = "cases/errpath2.rs"]
pub mod errpath2;
#[path = "cases/prim.rs"]
pub mod prim;
#[path = "cases/rdpath.rs"]
pub mod rdpath;
#[path = "cases/wrpath.rs"]
pub mod wrpath;

pub struct Spec {
    pub name: &'static str,
    /// How many independent randomized inputs to run through this case.
    pub seeds: u32,
    /// How many independent variants the case has. Each variant runs in its own
    /// pair of worker processes, which is what makes it possible to probe more
    /// than one fatal `png_error` condition per case.
    pub variants: u32,
    pub rows: &'static [&'static str],
}

macro_rules! specs {
    ( $( $name:literal , $seeds:literal , $variants:literal , [ $($row:literal),* $(,)? ] ; )* ) => {
        pub fn registry() -> Vec<Spec> {
            vec![ $( Spec {
                name: $name, seeds: $seeds, variants: $variants, rows: &[ $($row),* ]
            } ),* ]
        }
    }
}

specs! {
    /* ---- Phase B: exported primitives ---- */
    "prim/int_get", 4, 1, ["B1"];
    "prim/int_save", 4, 1, ["B2"];
    "prim/sig_cmp", 2, 1, ["B3", "C1", "C2", "C3"];
    "prim/version", 1, 1, ["B4"];
    "prim/grayscale_palette", 1, 1, ["B164"];
    "prim/rfc1123", 4, 1, ["B161", "C19", "C20", "C21", "C22"];
    "prim/info_lifecycle", 2, 1, ["B160", "C9", "C14", "C312", "C313"];
    "prim/null_guards", 1, 1, ["C10", "C11", "C12", "C15", "C23", "C24", "C25",
                                     "C26", "C27", "C28", "C29", "C30", "C35", "C323", "C324"];
    "prim/create_struct_bad_ver", 1, 1, ["C6", "C7", "C8"];
    "prim/malloc", 1, 1, ["C16"];
    "prim/mng_features", 1, 1, ["B162"];
    "prim/option", 1, 1, ["C315"];
    "prim/get_overflow", 3, 1, ["C31", "C32", "C33", "C34"];
    "prim/simple_zero_dims", 2, 8, ["C295", "C302"];

    /* ---- Phase B: write path ---- */
    "wr/chunk_raw", 4, 1, ["B5"];
    "wr/low_all_shapes", 6, 1, ["B6","B7","B8","B9","B10","B11","B12","B13","B14",
                                     "B15","B16","B17","B18","B19","B20"];
    "wr/low_interlaced", 4, 1, ["B21"];
    "wr/rows_api", 4, 1, ["B22"];
    "wr/image_api", 4, 1, ["B23"];
    "wr/image_api_interlaced", 4, 1, ["B24"];
    "wr/filters_single", 4, 1, ["B25","B26","B27","B28","B29"];
    "wr/filters_all", 4, 1, ["B30"];
    "wr/filters_pairs", 4, 1, ["B31"];
    "wr/compression_level", 3, 1, ["B32"];
    "wr/compression_strategy", 3, 1, ["B33"];
    "wr/compression_memlevel", 3, 1, ["B34"];
    "wr/compression_winbits", 3, 1, ["B35"];
    "wr/compression_bufsize", 3, 1, ["B36", "C227", "C228"];
    "wr/flush", 3, 1, ["B37"];
    "wr/status_fn", 3, 1, ["B38"];
    "wr/tr_bgr", 4, 1, ["B39"];
    "wr/tr_swap", 4, 1, ["B40"];
    "wr/tr_packing", 4, 1, ["B41"];
    "wr/tr_packswap", 4, 1, ["B42"];
    "wr/tr_shift", 4, 1, ["B43"];
    "wr/tr_invert_mono", 4, 1, ["B44"];
    "wr/tr_invert_alpha", 4, 1, ["B45"];
    "wr/tr_swap_alpha", 4, 1, ["B46"];
    "wr/tr_filler_after", 4, 1, ["B47"];
    "wr/tr_filler_before", 4, 1, ["B48"];
    "wr/tr_user", 4, 1, ["B49"];
    "wr/tr_pairs", 4, 1, ["B50"];
    "wr/png_identity", 4, 1, ["B51"];
    "wr/png_transforms", 4, 1, ["B52"];
    "wr/png_transform_pairs", 4, 1, ["B53"];
    "wr/anc_gama", 3, 1, ["B54"];
    "wr/anc_chrm", 3, 1, ["B55"];
    "wr/anc_srgb", 3, 1, ["B56"];
    "wr/anc_iccp", 3, 1, ["B57"];
    "wr/anc_sbit", 3, 1, ["B58"];
    "wr/anc_trns", 3, 1, ["B59"];
    "wr/anc_bkgd", 3, 1, ["B60"];
    "wr/anc_hist", 3, 1, ["B61"];
    "wr/anc_phys", 3, 1, ["B62"];
    "wr/anc_offs", 3, 1, ["B63"];
    "wr/anc_pcal", 3, 1, ["B64"];
    "wr/anc_scal", 3, 1, ["B65"];
    "wr/anc_time", 3, 1, ["B66"];
    "wr/anc_splt", 3, 1, ["B67"];
    "wr/anc_text", 3, 1, ["B68"];
    "wr/anc_ztxt", 3, 1, ["B69"];
    "wr/anc_itxt", 3, 1, ["B70"];
    "wr/anc_exif", 3, 1, ["B71"];
    "wr/anc_cicp", 3, 1, ["B72"];
    "wr/anc_clli", 3, 1, ["B73"];
    "wr/anc_mdcv", 3, 1, ["B74"];
    "wr/anc_unknown", 3, 1, ["B75"];
    "wr/anc_everything", 4, 1, ["B76", "B152"];
    "wr/simple_formats", 4, 1, ["B77"];
    "wr/simple_colormap", 4, 1, ["B78"];
    "wr/simple_convert8", 3, 1, ["B79"];
    "wr/simple_stride", 3, 1, ["B80"];
    "wr/simple_sizing", 3, 1, ["B81"];
    "wr/simple_file", 2, 1, ["B82"];

    /* ---- Phase B: read path ---- */
    "rd/low_all", 6, 1, ["B83"];
    "rd/low_interlaced", 4, 1, ["B84"];
    "rd/low_interlaced_manual", 4, 1, ["B85"];
    "rd/rows_api", 4, 1, ["B86"];
    "rd/image_api", 4, 1, ["B87"];
    "rd/png_identity", 4, 1, ["B88"];
    "rd/png_transforms", 4, 1, ["B89"];
    "rd/png_transform_combos", 4, 1, ["B90"];
    "rd/prog_chunked", 4, 1, ["B91"];
    "rd/prog_interlaced", 4, 1, ["B92"];
    "rd/prog_pause", 3, 1, ["B93"];
    "rd/prog_partial_cbs", 3, 1, ["B94"];
    "rd/simple_formats", 4, 1, ["B95"];
    "rd/simple_colormap", 4, 1, ["B96"];
    "rd/simple_background", 3, 1, ["B97"];
    "rd/simple_stride", 3, 1, ["B98"];
    "rd/simple_file", 2, 1, ["B99"];
    "rd/tr_palette_to_rgb", 4, 1, ["B100"];
    "rd/tr_expand", 4, 1, ["B101"];
    "rd/tr_expand_gray", 4, 1, ["B102"];
    "rd/tr_expand_16", 4, 1, ["B103"];
    "rd/tr_gray_to_rgb", 4, 1, ["B104"];
    "rd/tr_rgb_to_gray", 4, 1, ["B105", "C185"];
    "rd/tr_strip_16", 4, 1, ["B106"];
    "rd/tr_scale_16", 4, 1, ["B107"];
    "rd/tr_strip_alpha", 4, 1, ["B108"];
    "rd/tr_swap_alpha", 4, 1, ["B109"];
    "rd/tr_invert_alpha", 4, 1, ["B110"];
    "rd/tr_filler", 4, 1, ["B111"];
    "rd/tr_add_alpha", 4, 1, ["B112"];
    "rd/tr_packing", 4, 1, ["B113"];
    "rd/tr_packswap", 4, 1, ["B114"];
    "rd/tr_shift", 4, 1, ["B115"];
    "rd/tr_swap", 4, 1, ["B116"];
    "rd/tr_bgr", 4, 1, ["B117"];
    "rd/tr_invert_mono", 4, 1, ["B118"];
    "rd/tr_gamma_fixed", 4, 1, ["B119"];
    "rd/tr_gamma_float", 4, 1, ["B120"];
    "rd/tr_gamma_gama_chunk", 4, 1, ["B121"];
    "rd/tr_gamma_srgb_chunk", 4, 1, ["B122"];
    "rd/tr_alpha_mode_png", 3, 1, ["B123"];
    "rd/tr_alpha_mode_std", 3, 1, ["B124"];
    "rd/tr_alpha_mode_assoc", 3, 1, ["B125"];
    "rd/tr_alpha_mode_opt", 3, 1, ["B126"];
    "rd/tr_alpha_mode_broken", 3, 1, ["B127"];
    "rd/tr_bg_screen", 3, 1, ["B128"];
    "rd/tr_bg_file", 3, 1, ["B129"];
    "rd/tr_bg_unique", 3, 1, ["B130"];
    "rd/tr_bg_palette", 3, 1, ["B131"];
    "rd/tr_bg_trns", 3, 1, ["B132"];
    "rd/tr_quantize_rgb", 3, 1, ["B133", "C193"];
    "rd/tr_quantize_palette", 3, 1, ["B134"];
    "rd/tr_user", 3, 1, ["B135"];
    "rd/status_fn", 3, 1, ["B136"];
    "rd/tr_pipelines", 5, 1, ["B137"];
    "rd/benign_errors", 2, 1, ["B138"];
    "rd/crc_action_grid", 2, 1, ["B139", "C41", "C42", "C186", "C309"];
    "rd/check_invalid_index", 2, 1, ["B140", "C56", "B165"];
    "rd/user_limits", 2, 1, ["B141", "C62", "C65", "C330"];
    "rd/chunk_cache_max", 2, 1, ["B142", "C54", "C109", "C110", "C132", "C133"];
    "rd/chunk_malloc_max", 2, 1, ["B143", "C48", "C55"];
    "rd/opt_inflate_window", 2, 1, ["B144"];
    "rd/opt_skip_srgb", 2, 1, ["B145"];
    "rd/opt_ignore_adler", 2, 1, ["B146"];
    "rd/keep_unknown", 2, 1, ["B147", "B153"];
    "rd/user_chunk_fn", 2, 1, ["B148", "C53"];
    // png_set_strip_error_numbers is not exported by this build (PNG_ERROR_NUMBERS_SUPPORTED
    // is undefined in pnglibconf.h); CONFIGS.md row B149 is marked N/A there.
    "rd/mem_fn", 2, 1, ["B150"];
    "rd/sig_bytes", 2, 1, ["B151", "C4", "C5"];
    "rd/anc_after_idat", 2, 1, ["B154"];
    "rd/idat_split", 3, 1, ["B155"];
    "rd/update_info_report", 3, 1, ["B159"];
    "rd/roundtrip_low", 5, 1, ["B156"];
    "rd/roundtrip_png", 4, 1, ["B157"];
    "rd/roundtrip_simple", 4, 1, ["B158"];
    "rd/io_state", 2, 1, ["B163"];


    /* ---- Phase B: internal (PNG_INTERNAL_*) exported entry points ----
     * This build exports libpng's private helpers too, so they are part of the
     * ABI under test and are driven DIRECTLY here rather than only through the
     * convenience wrappers. CONFIGS.md axis A4 ("the FULL set of public entry
     * points, INCLUDING the lowest-level ones") and A11. */
    "ll/math_fixed", 4, 1, ["B1"];
    "ll/gamma_correct", 4, 1, ["B119"];
    "ll/srgb_tables", 1, 1, ["B4"];
    "ll/colorspace_xy", 4, 1, ["B55"];
    "ll/fp_strings", 4, 1, ["B65"];
    "ll/ascii_from", 4, 1, ["B65"];
    "ll/safecat_format", 4, 1, ["B4"];
    "ll/check_keyword", 4, 1, ["C218", "C219"];
    "ll/row_bgr_invert_swap", 4, 1, ["B39", "B40", "B42", "B44"];
    "ll/row_strip_channel", 4, 1, ["B47", "B48"];
    "ll/row_read_interlace", 4, 1, ["B84"];
    "ll/row_write_interlace", 4, 1, ["B21"];
    "ll/read_filter_row", 6, 1, ["B25", "B26", "B27", "B28", "B29", "C145"];
    "ll/check_ihdr_direct", 1, 14, ["C60", "C66", "C67", "C68"];
    "ll/chunk_unknown_handling", 2, 1, ["B147"];
    "ll/user_version_check", 1, 1, ["C6"];
    "ll/zalloc", 1, 1, ["C15"];
    "ll/malloc_internals", 1, 1, ["C16"];
    "ll/set_text_2", 2, 1, ["C216", "C217"];
    "ll/warning_params", 2, 1, ["C219"];
    "ll/app_error", 1, 6, ["C171"];
    "ll/filter_heuristics", 2, 1, ["B30"];
    "ll/create_png_struct", 1, 1, ["C6", "C9"];
    "ll/io_direct", 2, 1, ["B163"];
    "ll/crc_direct", 3, 1, ["B5"];
    "ll/write_chunks_direct", 4, 1, ["B76"];
    "ll/error_dispatch", 1, 8, ["C323"];

    /* ---- Phase C: error paths ---- */
    "err/sig_bad", 1, 4, ["C37", "C38", "C159", "C160"];
    "err/chunk_header", 1, 3, ["C39", "C40", "C158"];
    "err/missing_ihdr", 1, 3, ["C43", "C49", "C162"];
    "err/out_of_place", 1, 10, ["C44", "C75", "C77", "C113", "C116"];
    "err/duplicate", 1, 4, ["C45", "C74"];
    "err/len_too_short", 1, 2, ["C46"];
    "err/len_too_long", 1, 2, ["C47"];
    "err/missing_plte", 1, 2, ["C50", "C163"];
    "err/too_many_idat", 1, 10, ["C51", "C57", "C58", "C164", "C149"];
    "err/critical_unknown", 1, 1, ["C52"];
    "err/ihdr_length", 1, 4, ["C59", "C161"];
    "err/ihdr_fields", 1, 12, ["C60","C61","C63","C64","C66","C67","C68","C69",
                                     "C70","C71","C72","C314"];
    "err/plte", 1, 4, ["C73", "C76", "C194", "C195", "C196"];
    "err/iend", 1, 2, ["C78"];
    "err/gama", 1, 2, ["C79"];
    "err/sbit", 1, 4, ["C80", "C81"];
    "err/chrm", 1, 2, ["C82"];
    "err/srgb", 1, 2, ["C83"];
    "err/iccp_frame", 1, 10, ["C84", "C85", "C86", "C87", "C88"];
    "err/iccp_profile", 1, 36, ["C89","C90","C91","C92","C93","C94","C95","C96",
                                     "C97","C98","C99","C100","C101","C102","C103",
                                     "C104","C105","C106"];
    "err/splt", 1, 2, ["C107", "C108"];
    "err/trns", 1, 8, ["C111", "C112", "C114", "C115"];
    "err/bkgd", 1, 8, ["C117", "C118", "C119", "C120"];
    "err/exif", 1, 2, ["C121"];
    "err/hist", 1, 2, ["C122"];
    "err/pcal", 1, 8, ["C123", "C124", "C125", "C126"];
    "err/scal", 1, 10, ["C127", "C128", "C129", "C130", "C131"];
    "err/ztxt", 1, 8, ["C134", "C135", "C136", "C137"];
    "err/itxt", 1, 10, ["C138", "C139", "C140", "C141", "C142"];
    "err/zstream_reuse", 1, 1, ["C143"];
    "err/row_before_idat", 1, 1, ["C144"];
    "err/bad_filter_value", 1, 2, ["C145", "C170"];
    "err/not_enough_data", 1, 4, ["C146", "C147", "C165", "C168"];
    "err/too_much_data", 1, 4, ["C148", "C169"];
    "err/idat_corrupt", 1, 4, ["C150", "C151", "C166", "C167"];
    "err/row_too_big", 1, 1, ["C152"];
    "err/duplicate_update", 1, 2, ["C153", "C154"];
    "err/read_png_limits", 1, 2, ["C155", "C156"];
    "err/int32_overflow", 1, 1, ["C157", "C331"];
    "err/rtran_ordering", 1, 3, ["C171", "C172", "C173", "C190", "C322"];
    "err/alpha_mode_bad", 1, 3, ["C174", "C175", "C176", "C306"];
    "err/gamma_bad", 1, 3, ["C177", "C178", "C179"];
    "err/background_bad", 1, 3, ["C180", "C181", "C308"];
    "err/rgb_to_gray_bad", 1, 3, ["C182", "C183", "C184", "C307"];
    "err/shift_bad", 1, 1, ["C187"];
    "err/filler_bad", 1, 2, ["C188", "C189", "C321"];
    "err/interlace_handling", 1, 1, ["C191"];
    "err/user_transform_depth", 1, 1, ["C192"];
    "err/set_scal_bad", 1, 3, ["C197", "C198", "C199", "C200", "C201", "C316"];
    "err/set_iccp_bad", 1, 1, ["C202", "C203"];
    "err/set_pcal_bad", 1, 3, ["C204", "C205", "C206", "C317"];
    "err/set_hist_bad", 1, 1, ["C207"];
    "err/set_time_bad", 1, 1, ["C208"];
    "err/set_trns_bad", 1, 1, ["C209"];
    "err/set_cicp_bad", 1, 1, ["C210"];
    "err/set_clli_bad", 1, 1, ["C211"];
    "err/set_mdcv_bad", 1, 1, ["C212"];
    "err/set_chrm_xyz_bad", 1, 1, ["C213"];
    "err/set_exif_bad", 1, 1, ["C214", "C215"];
    "err/set_text_bad", 1, 2, ["C216", "C217", "C218", "C219", "C318"];
    "err/set_unknown_bad", 1, 4, ["C220", "C221", "C222", "C223", "C224", "C225",
                                     "C310", "C311"];
    "err/set_bufsize_bad", 1, 1, ["C226"];
    "err/write_ihdr_bad", 1, 7, ["C229","C230","C231","C232","C233","C234",
                                     "C235","C236","C237"];
    "err/write_info_no_plte", 1, 1, ["C238"];
    "err/write_plte_bad", 1, 2, ["C239", "C240", "C241"];
    "err/write_row_early", 1, 1, ["C242"];
    "err/write_end_no_idat", 1, 2, ["C243", "C244"];
    "err/write_chunk_toolong", 1, 1, ["C245"];
    "err/set_filter_bad", 1, 3, ["C246", "C247", "C248", "C304", "C305"];
    "err/compression_coerce", 1, 1, ["C249", "C250", "C251", "C252", "C320"];
    "err/write_iccp_bad", 1, 4, ["C253", "C254", "C255", "C256"];
    "err/write_text_bad", 1, 6, ["C257","C258","C259","C260","C261","C262"];
    "err/write_pcal_bad", 1, 2, ["C263", "C264"];
    "err/write_range_warns", 1, 1, ["C265","C266","C267","C268","C269","C270",
                                     "C271","C272","C273","C274"];
    "err/write_png_bad", 1, 2, ["C275", "C276", "C277"];
    "err/simple_read_args", 1, 1, ["C278","C279","C280","C281","C282","C283",
                                     "C284","C285","C286","C287"];
    "err/simple_finish_args", 1, 1, ["C288","C289","C290","C291","C292","C293",
                                     "C294","C295"];
    "err/simple_write_args", 1, 1, ["C296","C297","C298","C299","C300","C301",
                                     "C302","C303"];
    "err/image_free", 1, 1, ["C303"];
    "err/gama_srgb_enum", 1, 1, ["C319"];
    "err/row_null", 1, 3, ["C325", "C326", "C327"];
    "err/truncated_stream", 1, 2, ["C328", "C329"];
    "err/sig_bytes_toomany", 1, 1, ["C4"];
    "err/data_freer_bad", 1, 1, ["C14", "C312"];
    "err/free_data_masks", 1, 1, ["C313"];
    "err/longjmp_fn", 1, 1, ["C13"];
    "err/read_write_fn_mix", 1, 1, ["C17", "C18"];
    "err/get_eXIf_deprecated", 1, 1, ["C27"];
    "err/process_data_skip", 1, 1, ["C36"];
    "err/uint31_range", 1, 1, ["C158"];
    "err/enum_fuzz", 2, 8, ["C304","C306","C307","C310","C311","C312",
                                     "C315","C316","C317"];
}

pub fn run(api: &Api, case: &str, seed: u64) {
    if prim::run(api, case, seed) {
        return;
    }
    if lowlevel::run(api, case, seed) {
        return;
    }
    if wrpath::run(api, case, seed) {
        return;
    }
    if rdpath::run(api, case, seed) {
        return;
    }
    if errpath::run(api, case, seed) {
        return;
    }
    if errpath2::run(api, case, seed) {
        return;
    }
    crate::support::emit(&format!("UNKNOWN-CASE {}", case));
    crate::support::finish(3);
}
