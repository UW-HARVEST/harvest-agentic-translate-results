| | `png_get_valid` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:22`, `:36`) | `return 0` |
| | `png_get_valid` | `flag == PNG_INFO_tRNS && png_ptr->num_trans == 0` (tRNS canceled by `png_handle_PLTE`) (`pngget.c:29-30`) | `return 0` |
| | `png_get_valid` | requested `flag` bit clear in `info_ptr->valid` (`pngget.c:33`) | `return info_ptr->valid & flag` == `0` |
| | `png_get_rowbytes` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:42-45`) | `return 0` |
| | `png_get_rows` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:52-55`) | `return 0` (NULL row-pointer array) |
| | `png_get_image_width` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:64-67`) | `return 0` |
| | `png_get_image_height` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:73-76`) | `return 0` |
| | `png_get_bit_depth` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:82-85`) | `return 0` |
| | `png_get_color_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:91-94`) | `return 0` |
| | `png_get_filter_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:100-103`) | `return 0` |
| | `png_get_interlace_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:109-112`) | `return 0` |
| | `png_get_compression_type` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:118-121`) | `return 0` |
| | `png_get_x_pixels_per_meter` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:131-132`) | `return 0` |
| | `png_get_x_pixels_per_meter` | `info_ptr->phys_unit_type != PNG_RESOLUTION_METER` (`pngget.c:134`) | `return 0` |
| | `png_get_y_pixels_per_meter` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:152-153`) | `return 0` |
| | `png_get_y_pixels_per_meter` | `info_ptr->phys_unit_type != PNG_RESOLUTION_METER` (`pngget.c:155`) | `return 0` |
| | `png_get_pixels_per_meter` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:172-173`) | `return 0` |
| | `png_get_pixels_per_meter` | `info_ptr->phys_unit_type != PNG_RESOLUTION_METER` (`pngget.c:175`) | `return 0` |
| | `png_get_pixels_per_meter` | `info_ptr->x_pixels_per_unit != info_ptr->y_pixels_per_unit` (non-square pixels) (`pngget.c:176`) | `return 0` |
| | `png_get_pixel_aspect_ratio` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:195-196`) | `return (float)0.0` |
| | `png_get_pixel_aspect_ratio` | `info_ptr->x_pixels_per_unit == 0` (divide-by-zero guard) (`pngget.c:198`) | `return (float)0.0` |
| | `png_get_pixel_aspect_ratio_fixed` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:219-220`) | `return 0` |
| | `png_get_pixel_aspect_ratio_fixed` | `info_ptr->x_pixels_per_unit <= 0 \|\| info_ptr->y_pixels_per_unit <= 0` (`pngget.c:221`) | `return 0` |
| | `png_get_pixel_aspect_ratio_fixed` | `info_ptr->x_pixels_per_unit > PNG_UINT_31_MAX \|\| info_ptr->y_pixels_per_unit > PNG_UINT_31_MAX` (cast-overflow guard) (`pngget.c:222-223`) | `return 0` |
| | `png_get_pixel_aspect_ratio_fixed` | `png_muldiv(&res, y_pixels_per_unit, PNG_FP_1, x_pixels_per_unit) == 0` (fixed-point overflow) (`pngget.c:230-231`) | `return 0` |
| | `png_get_x_offset_microns` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:249-250`) | `return 0` |
| | `png_get_x_offset_microns` | `info_ptr->offset_unit_type != PNG_OFFSET_MICROMETER` (`pngget.c:252`) | `return 0` |
| | `png_get_y_offset_microns` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:269-270`) | `return 0` |
| | `png_get_y_offset_microns` | `info_ptr->offset_unit_type != PNG_OFFSET_MICROMETER` (`pngget.c:272`) | `return 0` |
| | `png_get_x_offset_pixels` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:289-290`) | `return 0` |
| | `png_get_x_offset_pixels` | `info_ptr->offset_unit_type != PNG_OFFSET_PIXEL` (`pngget.c:292`) | `return 0` |
| | `png_get_y_offset_pixels` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:309-310`) | `return 0` |
| | `png_get_y_offset_pixels` | `info_ptr->offset_unit_type != PNG_OFFSET_PIXEL` (`pngget.c:312`) | `return 0` |
| | `ppi_from_ppm` (static; used by `png_get_pixels_per_inch`, `png_get_x_pixels_per_inch`, `png_get_y_pixels_per_inch`) | `ppm > PNG_UINT_31_MAX` (`pngget.c:347`) | `return 0` (overflow) |
| | `ppi_from_ppm` | `png_muldiv(&result, (png_int_32)ppm, 127, 5000) == 0` (overflow) (`pngget.c:347-352`) | `return 0` |
| | `png_fixed_inches_from_microns` (static; used by `png_get_x_offset_inches_fixed`, `png_get_y_offset_inches_fixed`) | `png_muldiv(&result, microns, 500, 127) == 0` (fixed-point overflow) (`pngget.c:385-389`) | `png_warning(png_ptr, "fixed point overflow ignored")`, `return 0` |
| | `png_get_pHYs_dpi` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:442-443`) | `return 0` (retval untouched) |
| | `png_get_pHYs_dpi` | all of `res_x`, `res_y`, `unit_type` are `NULL` (`pngget.c:445`, `:451`, `:457`) | `return 0` (no `PNG_INFO_pHYs` bit ever OR'ed in) |
| | `png_get_channels` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:483-486`) | `return 0` |
| | `png_get_signature` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:493-496`) | `return NULL` |
| | `png_get_bKGD` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:507`) | `return 0` |
| | `png_get_bKGD` | `(info_ptr->valid & PNG_INFO_bKGD) == 0` (no bKGD chunk) (`pngget.c:508`) | `return 0` |
| | `png_get_bKGD` | `background == NULL` (`pngget.c:509`) | `return 0` |
| | `png_get_cHRM` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:533`) | `return 0` |
| | `png_get_cHRM` | `(info_ptr->valid & PNG_INFO_cHRM) == 0` (`pngget.c:534`) | `return 0` |
| | `png_get_cHRM_XYZ` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:567`) | `return 0` |
| | `png_get_cHRM_XYZ` | `(info_ptr->valid & PNG_INFO_cHRM) == 0` (`pngget.c:568`) | `return 0` |
| | `png_get_cHRM_XYZ` | `png_XYZ_from_xy(&XYZ, &info_ptr->cHRM) != 0` (degenerate/unrepresentable chromaticities) (`pngget.c:569`) | `return 0` |
| | `png_get_cHRM_XYZ_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:608`) | `return 0` |
| | `png_get_cHRM_XYZ_fixed` | `(info_ptr->valid & PNG_INFO_cHRM) == 0U` (`pngget.c:609`) | `return 0` |
| | `png_get_cHRM_XYZ_fixed` | `png_XYZ_from_xy(&XYZ, &info_ptr->cHRM) != 0` (`pngget.c:610`) | `return 0` |
| | `png_get_cHRM_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:636`) | `return 0` |
| | `png_get_cHRM_fixed` | `(info_ptr->valid & PNG_INFO_cHRM) == 0` (`pngget.c:637`) | `return 0` |
| | `png_get_gAMA_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:664`) | `return 0` |
| | `png_get_gAMA_fixed` | `(info_ptr->valid & PNG_INFO_gAMA) == 0` (`pngget.c:665`) | `return 0` |
| | `png_get_gAMA` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:683`) | `return 0` |
| | `png_get_gAMA` | `(info_ptr->valid & PNG_INFO_gAMA) == 0` (`pngget.c:684`) | `return 0` |
| | `png_get_sRGB` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:704`) | `return 0` |
| | `png_get_sRGB` | `(info_ptr->valid & PNG_INFO_sRGB) == 0` (`pngget.c:705`) | `return 0` |
| | `png_get_iCCP` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:724`) | `return 0` |
| | `png_get_iCCP` | `(info_ptr->valid & PNG_INFO_iCCP) == 0` (`pngget.c:725`) | `return 0` |
| | `png_get_iCCP` | `name == NULL \|\| profile == NULL \|\| proflen == NULL` (`pngget.c:726`) | `return 0` |
| | `png_get_sPLT` | `png_ptr == NULL \|\| info_ptr == NULL \|\| spalettes == NULL` (`pngget.c:750`) | `return 0` |
| | `png_get_sPLT` | no sPLT stored, i.e. `info_ptr->splt_palettes_num == 0` (`pngget.c:753`) | `return 0` |
| | `png_get_cICP` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:769`) | `return 0` |
| | `png_get_cICP` | `(info_ptr->valid & PNG_INFO_cICP) == 0` (`pngget.c:770`) | `return 0` |
| | `png_get_cICP` | `colour_primaries == NULL \|\| transfer_function == NULL \|\| matrix_coefficients == NULL \|\| video_full_range_flag == NULL` (`pngget.c:771-772`) | `return 0` |
| | `png_get_cLLI_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:794`) | `return 0` |
| | `png_get_cLLI_fixed` | `(info_ptr->valid & PNG_INFO_cLLI) == 0` (`pngget.c:795`) | `return 0` |
| | `png_get_cLLI` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:813`) | `return 0` |
| | `png_get_cLLI` | `(info_ptr->valid & PNG_INFO_cLLI) == 0` (`pngget.c:814`) | `return 0` |
| | `png_get_mDCV_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:838`) | `return 0` |
| | `png_get_mDCV_fixed` | `(info_ptr->valid & PNG_INFO_mDCV) == 0` (`pngget.c:839`) | `return 0` |
| | `png_get_mDCV` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:867`) | `return 0` |
| | `png_get_mDCV` | `(info_ptr->valid & PNG_INFO_mDCV) == 0` (`pngget.c:868`) | `return 0` |
| | `png_get_eXIf` | any call (API permanently disabled) (`pngget.c:895-898`) | `png_warning(png_ptr, "png_get_eXIf does not work; use png_get_eXIf_1")`, `return 0` |
| | `png_get_eXIf_1` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:907`) | `return 0` |
| | `png_get_eXIf_1` | `(info_ptr->valid & PNG_INFO_eXIf) == 0` (`pngget.c:908`) | `return 0` |
| | `png_get_eXIf_1` | `exif == NULL` (`pngget.c:908`) | `return 0` |
| | `png_get_hIST` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:926`) | `return 0` |
| | `png_get_hIST` | `(info_ptr->valid & PNG_INFO_hIST) == 0` (`pngget.c:927`) | `return 0` |
| | `png_get_hIST` | `hist == NULL` (`pngget.c:927`) | `return 0` |
| | `png_get_IHDR` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:945-946`) | `return 0` |
| | `png_get_IHDR` | stored IHDR fields invalid (app tampered with `info_ptr` directly): re-validated via `png_check_IHDR(...)` (`pngget.c:974-976`) | `png_error` from `png_check_IHDR` (e.g. `"Invalid image width"`/`"Invalid bit depth"`) |
| | `png_get_oFFs` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:988`) | `return 0` |
| | `png_get_oFFs` | `(info_ptr->valid & PNG_INFO_oFFs) == 0` (`pngget.c:989`) | `return 0` |
| | `png_get_oFFs` | `offset_x == NULL \|\| offset_y == NULL \|\| unit_type == NULL` (`pngget.c:990`) | `return 0` |
| | `png_get_pCAL` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1010`) | `return 0` |
| | `png_get_pCAL` | `(info_ptr->valid & PNG_INFO_pCAL) == 0` (`pngget.c:1011`) | `return 0` |
| | `png_get_pCAL` | any of `purpose`, `X0`, `X1`, `type`, `nparams`, `units`, `params` is `NULL` (`pngget.c:1012-1013`) | `return 0` |
| | `png_get_sCAL_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1039`) | `return 0` |
| | `png_get_sCAL_fixed` | `(info_ptr->valid & PNG_INFO_sCAL) == 0` (`pngget.c:1040`) | `return 0` |
| | `png_get_sCAL_fixed` | stored `scal_s_width`/`scal_s_height` not representable as fixed point (`png_fixed(png_ptr, atof(...), "sCAL width"/"sCAL height")`) (`pngget.c:1047-1049`) | `png_fixed_error` → `png_error(png_ptr, "fixed point overflow in sCAL width")` |
| | `png_get_sCAL` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1064`) | `return 0` |
| | `png_get_sCAL` | `(info_ptr->valid & PNG_INFO_sCAL) == 0` (`pngget.c:1065`) | `return 0` |
| | `png_get_sCAL_s` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1082`) | `return 0` |
| | `png_get_sCAL_s` | `(info_ptr->valid & PNG_INFO_sCAL) == 0` (`pngget.c:1083`) | `return 0` |
| | `png_get_pHYs` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_pHYs) == 0` (`pngget.c:1104-1105`) | `return 0` |
| | `png_get_pHYs` | all of `res_x`, `res_y`, `unit_type` are `NULL` (`pngget.c:1107`, `:1113`, `:1119`) | `return 0` (retval stays 0) |
| | `png_get_PLTE` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1136`) | `return 0` |
| | `png_get_PLTE` | `(info_ptr->valid & PNG_INFO_PLTE) == 0` (`pngget.c:1137`) | `return 0` |
| | `png_get_PLTE` | `palette == NULL` (`pngget.c:1137`) | `return 0` |
| | `png_get_sBIT` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1155`) | `return 0` |
| | `png_get_sBIT` | `(info_ptr->valid & PNG_INFO_sBIT) == 0` (`pngget.c:1156`) | `return 0` |
| | `png_get_sBIT` | `sig_bit == NULL` (`pngget.c:1156`) | `return 0` |
| | `png_get_text` | `png_ptr == NULL \|\| info_ptr == NULL \|\| info_ptr->num_text <= 0` (`pngget.c:1171`) | `*num_text = 0` if non-NULL (`:1185-1186`), `return 0` |
| | `png_get_tIME` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1199`) | `return 0` |
| | `png_get_tIME` | `(info_ptr->valid & PNG_INFO_tIME) == 0` (`pngget.c:1200`) | `return 0` |
| | `png_get_tIME` | `mod_time == NULL` (`pngget.c:1200`) | `return 0` |
| | `png_get_tRNS` | `png_ptr == NULL \|\| info_ptr == NULL \|\| (info_ptr->valid & PNG_INFO_tRNS) == 0` (`pngget.c:1219-1220`) | `return 0` |
| | `png_get_tRNS` | `info_ptr->color_type == PNG_COLOR_TYPE_PALETTE && trans_alpha == NULL && num_trans == NULL` (`pngget.c:1222-1230`, `:1246`) | `return 0` (`PNG_INFO_tRNS` never OR'ed in) |
| | `png_get_tRNS` | `info_ptr->color_type != PNG_COLOR_TYPE_PALETTE && trans_color == NULL && num_trans == NULL` (`pngget.c:1234-1244`, `:1246`) | `return 0` |
| | `png_get_tRNS` | `info_ptr->color_type != PNG_COLOR_TYPE_PALETTE` with non-NULL `trans_alpha` (no per-palette alpha exists) (`pngget.c:1242-1243`) | `*trans_alpha = NULL` |
| | `png_get_unknown_chunks` | `png_ptr == NULL \|\| info_ptr == NULL \|\| unknowns == NULL` (`pngget.c:1262`) | `return 0` |
| | `png_get_unknown_chunks` | no stored unknown chunks, `info_ptr->unknown_chunks_num == 0` (`pngget.c:1265`) | `return 0` |
| | `png_get_rgb_to_gray_status` | `png_ptr == NULL` (`pngget.c:1276`) | `return 0` |
| | `png_get_user_chunk_ptr` | `png_ptr == NULL` (`pngget.c:1284`) | `return NULL` |
| | `png_get_compression_buffer_size` | `png_ptr == NULL` (`pngget.c:1291-1292`) | `return 0` |
| | `png_get_user_width_max` | `png_ptr == NULL` (`pngget.c:1317`) | `return 0` |
| | `png_get_user_height_max` | `png_ptr == NULL` (`pngget.c:1323`) | `return 0` |
| | `png_get_chunk_cache_max` | `png_ptr == NULL` (`pngget.c:1330`) | `return 0` |
| | `png_get_chunk_malloc_max` | `png_ptr == NULL` (`pngget.c:1337`) | `return 0` |
| | `png_get_palette_max` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngget.c:1361-1364`) | `return -1` |
| | `png_set_bKGD` | `png_ptr == NULL \|\| info_ptr == NULL \|\| background == NULL` (`pngset.c:29-30`) | silent `return`; `PNG_INFO_bKGD` not set |
| | `png_set_cHRM_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:46-47`) | silent `return` |
| | `png_set_cHRM_XYZ_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:74-75`) | silent `return` |
| | `png_set_cHRM_XYZ_fixed` | `png_xy_from_XYZ(&xy, &XYZ) != 0` (XYZ values do not convert to valid xy chromaticities) (`pngset.c:87`, `:94`) | `png_app_error(png_ptr, "invalid cHRM XYZ")`; `PNG_INFO_cHRM` not set |
| | `png_set_cHRM` | any of `white_x..blue_y` outside `±21474.83647` so `floor(100000*fp+.5)` exceeds `png_fixed_point` range (`pngset.c:104-111`) | `png_fixed_error` → `png_error(png_ptr, "fixed point overflow in cHRM White X")` (etc. per argument name) |
| | `png_set_cHRM_XYZ` | any of `red_X..blue_Z` not representable as fixed point (`pngset.c:120-128`) | `png_error(png_ptr, "fixed point overflow in cHRM Red X")` (etc.) |
| | `png_set_cICP` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:142-143`) | silent `return` |
| | `png_set_cICP` | `matrix_coefficients != 0` (only identity matrix allowed in PNG) (`pngset.c:150-154`) | `png_warning(png_ptr, "Invalid cICP matrix coefficients")`; `PNG_INFO_cICP` not set |
| | `png_set_cLLI_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:170-171`) | silent `return` |
| | `png_set_cLLI_fixed` | `maxCLL > 0x7FFFFFFFU \|\| maxFALL > 0x7FFFFFFFU` (`pngset.c:174-185`) | `png_chunk_report(png_ptr, "cLLI light level exceeds PNG limit", PNG_CHUNK_WRITE_ERROR)`; chunk not stored |
| | `png_set_cLLI` | `maxCLL` or `maxFALL` negative or `floor(10000*fp+.5) > 2147483647` (`pngset.c:197-199`) | `png_fixed_error` → `png_error(png_ptr, "fixed point overflow in png_set_cLLI(maxCLL)")` (or `(maxFALL)`) |
| | `png_ITU_fixed_16` | `v/2 > 65535 \|\| v/2 < 0` after halving the fixed-point chromaticity (`pngset.c:215-219`) | `*error = 1`, `return 0` |
| | `png_set_mDCV_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:238-239`) | silent `return` |
| | `png_set_mDCV_fixed` | any of `white_x, white_y, red_x, red_y, green_x, green_y, blue_x, blue_y` rejected by `png_ITU_fixed_16` (`error != 0`) (`pngset.c:243-258`) | `png_chunk_report(png_ptr, "mDCV chromaticities outside representable range", PNG_CHUNK_WRITE_ERROR)`; chunk not stored |
| | `png_set_mDCV_fixed` | `maxDL > 0x7FFFFFFFU \|\| minDL > 0x7FFFFFFFU` (`pngset.c:261-272`) | `png_chunk_report(png_ptr, "mDCV display light level exceeds PNG limit", PNG_CHUNK_WRITE_ERROR)`; chunk not stored |
| | `png_set_mDCV` | any chromaticity double not representable as fixed point (`pngset.c:303-310`) | `png_error(png_ptr, "fixed point overflow in png_set_mDCV(white(x))")` (etc.) |
| | `png_set_mDCV` | `maxDL`/`minDL` negative or `> 214748.3647` (`png_fixed_ITU`) (`pngset.c:311-312`) | `png_error(png_ptr, "fixed point overflow in png_set_mDCV(maxDL)")` (or `(minDL)`) |
| | `png_set_eXIf` | any call (API permanently disabled) (`pngset.c:322`) | `png_warning(png_ptr, "png_set_eXIf does not work; use png_set_eXIf_1")`; nothing stored |
| | `png_set_eXIf_1` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:335`) | silent `return` |
| | `png_set_eXIf_1` | `(png_ptr->mode & PNG_WROTE_eXIf) != 0` (eXIf already written) (`pngset.c:336`) | silent `return` |
| | `png_set_eXIf_1` | `exif == NULL` (`pngset.c:337`) | silent `return` |
| | `png_set_eXIf_1` | `png_malloc_warn(png_ptr, num_exif)` returns `NULL` (out of memory / `num_exif` too large) (`pngset.c:340-346`) | `png_warning(png_ptr, "Insufficient memory for eXIf chunk data")`; `PNG_INFO_eXIf` not set |
| | `png_set_gAMA_fixed` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:366-367`) | silent `return` |
| | `png_set_gAMA` | `file_gamma` not representable as fixed point (`floor(100000*fp+.5)` out of `int32` range) (`pngset.c:377-378`) | `png_error(png_ptr, "fixed point overflow in png_set_gAMA")` |
| | `png_set_hIST` | `png_ptr == NULL \|\| info_ptr == NULL \|\| hist == NULL` (`pngset.c:393-394`) | silent `return` |
| | `png_set_hIST` | `info_ptr->num_palette == 0 \|\| info_ptr->num_palette > PNG_MAX_PALETTE_LENGTH` (hIST set before/with invalid PLTE) (`pngset.c:396-403`) | `png_warning(png_ptr, "Invalid palette size, hIST allocation skipped")`; `PNG_INFO_hIST` not set |
| | `png_set_hIST` | `png_malloc_warn(...)` for `PNG_MAX_PALETTE_LENGTH` entries returns `NULL` (`pngset.c:417-424`) | `png_warning(png_ptr, "Insufficient memory for hIST chunk data")` |
| | `png_set_IHDR` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:442-443`) | silent `return` |
| | `png_set_IHDR` | any invalid IHDR field (`width`/`height` == 0 or > limits, `bit_depth` not in {1,2,4,8,16}, invalid `color_type`, bit-depth/color-type combination, `interlace_type` > 1, `compression_type != PNG_COMPRESSION_TYPE_BASE`, `filter_type != PNG_FILTER_TYPE_BASE`) — checked by `png_check_IHDR` (`pngset.c:453-455`) | `png_error`/`png_warning` from `png_check_IHDR` (e.g. `"Invalid image width"`, `"Invalid bit depth"`, `"Invalid color type"`, `"Invalid image size in IHDR"`) |
| | `png_set_oFFs` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:481-482`) | silent `return` |
| | `png_set_pCAL` | `png_ptr == NULL \|\| info_ptr == NULL \|\| purpose == NULL \|\| units == NULL \|\| (nparams > 0 && params == NULL)` (`pngset.c:502-504`) | silent `return` |
| | `png_set_pCAL` | `type < 0 \|\| type > 3` (equation type outside PNG spec) (`pngset.c:513-518`) | `png_chunk_report(png_ptr, "Invalid pCAL equation type", PNG_CHUNK_WRITE_ERROR)`; chunk not stored |
| | `png_set_pCAL` | `nparams < 0 \|\| nparams > 255` (`pngset.c:520-525`) | `png_chunk_report(png_ptr, "Invalid pCAL parameter count", PNG_CHUNK_WRITE_ERROR)` |
| | `png_set_pCAL` | some `params[i] == NULL` or `!png_check_fp_string(params[i], strlen(params[i]))` (not a valid PNG floating-point string) (`pngset.c:528-537`) | `png_chunk_report(png_ptr, "Invalid format for pCAL parameter", PNG_CHUNK_WRITE_ERROR)` |
| | `png_set_pCAL` | `png_malloc_warn` for `pcal_purpose` returns `NULL` (`pngset.c:539-547`) | `png_chunk_report(png_ptr, "Insufficient memory for pCAL purpose", PNG_CHUNK_WRITE_ERROR)` |
| | `png_set_pCAL` | `png_malloc_warn` for `pcal_units` returns `NULL` (`pngset.c:563-570`) | `png_warning(png_ptr, "Insufficient memory for pCAL units")`; `PNG_INFO_pCAL` not set |
| | `png_set_pCAL` | `png_malloc_warn` for the `pcal_params` array returns `NULL` (`pngset.c:574-581`) | `png_warning(png_ptr, "Insufficient memory for pCAL params")` |
| | `png_set_pCAL` | `png_malloc_warn` for an individual `pcal_params[i]` returns `NULL` (`pngset.c:592-598`) | `png_warning(png_ptr, "Insufficient memory for pCAL parameter")` |
| | `png_set_sCAL_s` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:616-617`) | silent `return` |
| | `png_set_sCAL_s` | `unit != 1 && unit != 2` (only meter/radian allowed) (`pngset.c:622-623`) | `png_error(png_ptr, "Invalid sCAL unit")` |
| | `png_set_sCAL_s` | `swidth == NULL \|\| strlen(swidth) == 0 \|\| swidth[0] == '-' \|\| !png_check_fp_string(swidth, lengthw)` (`pngset.c:625-627`) | `png_error(png_ptr, "Invalid sCAL width")` |
| | `png_set_sCAL_s` | `sheight == NULL \|\| strlen(sheight) == 0 \|\| sheight[0] == '-' \|\| !png_check_fp_string(sheight, lengthh)` (`pngset.c:629-631`) | `png_error(png_ptr, "Invalid sCAL height")` |
| | `png_set_sCAL_s` | `png_malloc_warn` for `scal_s_width` returns `NULL` (`pngset.c:639-647`) | `png_warning(png_ptr, "Memory allocation failed while processing sCAL")`; `PNG_INFO_sCAL` not set |
| | `png_set_sCAL_s` | `png_malloc_warn` for `scal_s_height` returns `NULL` (`pngset.c:655-665`) | frees `scal_s_width`, `png_warning(png_ptr, "Memory allocation failed while processing sCAL")` |
| | `png_set_sCAL` | `width <= 0` (`pngset.c:681-682`) | `png_warning(png_ptr, "Invalid sCAL width ignored")`; nothing stored |
| | `png_set_sCAL` | `height <= 0` (`pngset.c:684-685`) | `png_warning(png_ptr, "Invalid sCAL height ignored")` |
| | `png_set_sCAL_fixed` | `width <= 0` (`pngset.c:711-712`) | `png_warning(png_ptr, "Invalid sCAL width ignored")` |
| | `png_set_sCAL_fixed` | `height <= 0` (`pngset.c:714-715`) | `png_warning(png_ptr, "Invalid sCAL height ignored")` |
| | `png_set_pHYs` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:739-740`) | silent `return` |
| | `png_set_PLTE` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:758-759`) | silent `return` |
| | `png_set_PLTE` | `num_palette < 0 \|\| num_palette > (1 << info_ptr->bit_depth)` when `info_ptr->color_type == PNG_COLOR_TYPE_PALETTE` (`pngset.c:761-767`) | `png_error(png_ptr, "Invalid palette length")` |
| | `png_set_PLTE` | `num_palette < 0 \|\| num_palette > PNG_MAX_PALETTE_LENGTH` when `color_type != PNG_COLOR_TYPE_PALETTE` (`pngset.c:764`, `:771-773`) | `png_warning(png_ptr, "Invalid palette length")` then `return` |
| | `png_set_PLTE` | `num_palette > 0 && palette == NULL` (`pngset.c:777`, `:784`) | `png_error(png_ptr, "Invalid palette")` |
| | `png_set_PLTE` | `num_palette == 0` and `(png_ptr->mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0` (empty PLTE not permitted) (`pngset.c:778-785`) | `png_error(png_ptr, "Invalid palette")` |
| | `png_set_sBIT` | `png_ptr == NULL \|\| info_ptr == NULL \|\| sig_bit == NULL` (`pngset.c:840-841`) | silent `return` |
| | `png_set_sRGB` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:854-855`) | silent `return` |
| | `png_set_sRGB_gAMA_and_cHRM` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:867-868`) | silent `return` |
| | `png_set_iCCP` | `png_ptr == NULL \|\| info_ptr == NULL \|\| name == NULL \|\| profile == NULL` (`pngset.c:900-901`) | silent `return` |
| | `png_set_iCCP` | `compression_type != PNG_COMPRESSION_TYPE_BASE` (`pngset.c:903-904`) | `png_app_error(png_ptr, "Invalid iCCP compression method")` |
| | `png_set_iCCP` | `png_malloc_warn(png_ptr, strlen(name)+1)` returns `NULL` (`pngset.c:907-914`) | `png_benign_error(png_ptr, "Insufficient memory to process iCCP chunk")`; `PNG_INFO_iCCP` not set |
| | `png_set_iCCP` | `png_malloc_warn(png_ptr, proflen)` returns `NULL` (`pngset.c:917-927`) | frees name, `png_benign_error(png_ptr, "Insufficient memory to process iCCP profile")` |
| | `png_set_text` | `png_set_text_2()` returns non-zero (allocation failure / too many text chunks) (`pngset.c:947-950`) | `png_error(png_ptr, "Insufficient memory to store text")` |
| | `png_set_text_2` | `png_ptr == NULL \|\| info_ptr == NULL \|\| num_text <= 0 \|\| text_ptr == NULL` (`pngset.c:963-964`) | `return 0` (nothing stored) |
| | `png_set_text_2` | growth overflow: `num_text > INT_MAX - max_text` where `max_text = info_ptr->num_text` (`pngset.c:979`) | `new_text` stays `NULL` → `png_chunk_report(png_ptr, "too many text chunks", PNG_CHUNK_WRITE_ERROR)`, `return 1` |
| | `png_set_text_2` | `png_realloc_array(...)` for the text array returns `NULL` (out of memory / array-size overflow) (`pngset.c:993-1004`) | `png_chunk_report(png_ptr, "too many text chunks", PNG_CHUNK_WRITE_ERROR)`, `return 1` |
| | `png_set_text_2` | `text_ptr[i].key == NULL` (`pngset.c:1025-1026`) | `continue` — entry silently skipped |
| | `png_set_text_2` | `text_ptr[i].compression < PNG_TEXT_COMPRESSION_NONE \|\| text_ptr[i].compression >= PNG_TEXT_COMPRESSION_LAST` (`pngset.c:1028-1034`) | `png_chunk_report(png_ptr, "text compression mode is out of range", PNG_CHUNK_WRITE_ERROR)`, `continue` |
| | `png_set_text_2` | `text_ptr[i].compression > 0` (iTXt) when built without `PNG_iTXt_SUPPORTED` (`pngset.c:1061-1066`) | `png_chunk_report(png_ptr, "iTXt chunk not supported", PNG_CHUNK_WRITE_ERROR)`, `continue` |
| | `png_set_text_2` | `png_malloc_base(png_ptr, key_len + text_length + lang_len + lang_key_len + 4)` returns `NULL` (`pngset.c:1087-1097`) | `png_chunk_report(png_ptr, "text chunk: out of memory", PNG_CHUNK_WRITE_ERROR)`, `return 1` |
| | `png_set_tIME` | `png_ptr == NULL \|\| info_ptr == NULL \|\| mod_time == NULL` (`pngset.c:1161`) | silent `return` |
| | `png_set_tIME` | `(png_ptr->mode & PNG_WROTE_tIME) != 0` (tIME already written) (`pngset.c:1162`) | silent `return` |
| | `png_set_tIME` | `mod_time->month == 0 \|\| mod_time->month > 12` (`pngset.c:1165`) | `png_warning(png_ptr, "Ignoring invalid time value")`; `PNG_INFO_tIME` not set |
| | `png_set_tIME` | `mod_time->day == 0 \|\| mod_time->day > 31` (`pngset.c:1166`) | `png_warning(png_ptr, "Ignoring invalid time value")` |
| | `png_set_tIME` | `mod_time->hour > 23` (`pngset.c:1167`) | `png_warning(png_ptr, "Ignoring invalid time value")` |
| | `png_set_tIME` | `mod_time->minute > 59` (`pngset.c:1167`) | `png_warning(png_ptr, "Ignoring invalid time value")` |
| | `png_set_tIME` | `mod_time->second > 60` (`pngset.c:1168`) | `png_warning(png_ptr, "Ignoring invalid time value")` |
| | `png_set_tRNS` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:1187-1189`) | silent `return` |
| | `png_set_tRNS` | `trans_alpha != NULL` but `num_trans <= 0 \|\| num_trans > PNG_MAX_PALETTE_LENGTH` (`pngset.c:1198`, `:1205`, `:1233-1237`) | alpha array not copied; `png_ptr->trans_alpha` freed and set to `NULL` |
| | `png_set_tRNS` | `info_ptr->bit_depth < 16` and `color_type == PNG_COLOR_TYPE_GRAY && trans_color->gray > (1 << bit_depth) - 1` (`pngset.c:1243-1253`) | `png_warning(png_ptr, "tRNS chunk has out-of-range samples for bit_depth")` (value still stored) |
| | `png_set_tRNS` | `info_ptr->bit_depth < 16` and `color_type == PNG_COLOR_TYPE_RGB` and `trans_color->red/green/blue > (1 << bit_depth) - 1` (`pngset.c:1249-1254`) | `png_warning(png_ptr, "tRNS chunk has out-of-range samples for bit_depth")` |
| | `png_set_sPLT` | `png_ptr == NULL \|\| info_ptr == NULL \|\| nentries <= 0 \|\| entries == NULL` (`pngset.c:1292-1293`) | silent `return` |
| | `png_set_sPLT` | `png_realloc_array(...)` for `splt_palettes` returns `NULL` (out of memory / too many palettes) (`pngset.c:1298-1307`) | `png_chunk_report(png_ptr, "too many sPLT chunks", PNG_CHUNK_WRITE_ERROR)` |
| | `png_set_sPLT` | `entries->name == NULL \|\| entries->entries == NULL` for some input entry (`pngset.c:1324-1330`) | `png_app_error(png_ptr, "png_set_sPLT: invalid sPLT")`, entry skipped via `continue` |
| | `png_set_sPLT` | `png_malloc_base(png_ptr, strlen(entries->name)+1)` returns `NULL` (`pngset.c:1338-1341`) | `break` out of loop → falls through to `png_chunk_report(png_ptr, "sPLT out of memory", PNG_CHUNK_WRITE_ERROR)` |
| | `png_set_sPLT` | `png_malloc_array(png_ptr, entries->nentries, sizeof (png_sPLT_entry))` returns `NULL` (`pngset.c:1349-1357`) | frees `np->name`, `break` → `png_chunk_report(png_ptr, "sPLT out of memory", PNG_CHUNK_WRITE_ERROR)` |
| | `png_set_sPLT` | `nentries > 0` remaining after the loop terminated early (`pngset.c:1378-1379`) | `png_chunk_report(png_ptr, "sPLT out of memory", PNG_CHUNK_WRITE_ERROR)` |
| | `check_location` | `(location & (PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT)) == 0` on a write struct (`(png_ptr->mode & PNG_IS_READ_STRUCT) == 0`) (`pngset.c:1393-1401`) | `png_app_warning(png_ptr, "png_set_unknown_chunks now expects a valid location")`, falls back to `png_ptr->mode` bits |
| | `check_location` | `location == 0` after the fallback (e.g. read struct with no valid location bits) (`pngset.c:1406-1407`) | `png_error(png_ptr, "invalid location in png_set_unknown_chunks")` |
| | `png_set_unknown_chunks` | `png_ptr == NULL \|\| info_ptr == NULL \|\| num_unknowns <= 0 \|\| unknowns == NULL` (`pngset.c:1428-1430`) | silent `return` |
| | `png_set_unknown_chunks` | called on a read struct in a build without `PNG_READ_UNKNOWN_CHUNKS_SUPPORTED` (`pngset.c:1440-1445`) | `png_app_error(png_ptr, "no unknown chunk support on read")`, `return` |
| | `png_set_unknown_chunks` | called on a write struct in a build without `PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED` (`pngset.c:1449-1454`) | `png_app_error(png_ptr, "no unknown chunk support on write")`, `return` |
| | `png_set_unknown_chunks` | `png_realloc_array(...)` for `unknown_chunks` returns `NULL` (out of memory / count overflow) (`pngset.c:1462-1471`) | `png_chunk_report(png_ptr, "too many unknown chunks", PNG_CHUNK_WRITE_ERROR)`, `return` |
| | `png_set_unknown_chunks` | `png_malloc_base(png_ptr, unknowns->size)` returns `NULL` for one chunk's data (`pngset.c:1500-1509`) | `png_chunk_report(png_ptr, "unknown chunk: out of memory", PNG_CHUNK_WRITE_ERROR)`, chunk skipped via `continue` |
| | `png_set_unknown_chunk_location` | `png_ptr == NULL \|\| info_ptr == NULL \|\| chunk < 0 \|\| chunk >= info_ptr->unknown_chunks_num` (index out of range) (`pngset.c:1535-1536`) | silent no-op |
| | `png_set_unknown_chunk_location` | `(location & (PNG_HAVE_IHDR\|PNG_HAVE_PLTE\|PNG_AFTER_IDAT)) == 0` (`pngset.c:1538-1547`) | `png_app_error(png_ptr, "invalid unknown chunk location")`, then location forced to `PNG_AFTER_IDAT` or `PNG_HAVE_IHDR` |
| | `png_permit_mng_features` | `png_ptr == NULL` (`pngset.c:1561-1562`) | `return 0` |
| | `png_permit_mng_features` | bits set in `mng_features` outside `PNG_ALL_MNG_FEATURES` (`pngset.c:1564`) | unsupported bits masked off; `return png_ptr->mng_features_permitted` (subset of request) |
| | `png_set_keep_unknown_chunks` | `png_ptr == NULL` (`pngset.c:1606-1607`) | silent `return` |
| | `png_set_keep_unknown_chunks` | `keep < 0 \|\| keep >= PNG_HANDLE_CHUNK_LAST` (`pngset.c:1609-1613`) | `png_app_error(png_ptr, "png_set_keep_unknown_chunks: invalid keep")`, `return` |
| | `png_set_keep_unknown_chunks` | `num_chunks_in == 0` (`pngset.c:1616-1622`) | only `png_ptr->unknown_default = keep`, then `return` (no list processed) |
| | `png_set_keep_unknown_chunks` | `num_chunks_in > 0 && chunk_list == NULL` (`pngset.c:1660-1667`) | `png_app_error(png_ptr, "png_set_keep_unknown_chunks: no chunk list")`, `return` |
| | `png_set_keep_unknown_chunks` | `num_chunks + old_num_chunks > UINT_MAX/5` (list-size overflow) (`pngset.c:1679-1684`) | `png_app_error(png_ptr, "png_set_keep_unknown_chunks: too many chunks")`, `return` |
| | `png_set_read_user_chunk_fn` | `png_ptr == NULL` (`pngset.c:1767-1768`) | silent `return` |
| | `png_set_rows` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:1782-1783`) | silent `return` |
| | `png_set_compression_buffer_size` | `png_ptr == NULL` (`pngset.c:1801-1802`) | silent `return` |
| | `png_set_compression_buffer_size` | `size == 0 \|\| size > PNG_UINT_31_MAX` (`pngset.c:1804-1805`) | `png_error(png_ptr, "invalid compression buffer size")` |
| | `png_set_compression_buffer_size` | write struct with `png_ptr->zowner != 0` (zstream in use) (`pngset.c:1818-1824`) | `png_warning(png_ptr, "Compression buffer size cannot be changed because it is in use")`, `return` |
| | `png_set_compression_buffer_size` | `size > ZLIB_IO_MAX` (`pngset.c:1830-1835`) | `png_warning(png_ptr, "Compression buffer size limited to system maximum")`, `size = ZLIB_IO_MAX` |
| | `png_set_compression_buffer_size` | `size < 6` (would hang deflate on SYNC_FLUSH) (`pngset.c:1838-1847`) | `png_warning(png_ptr, "Compression buffer size cannot be reduced below 6")`, `return` |
| | `png_set_invalid` | `png_ptr == NULL \|\| info_ptr == NULL` (`pngset.c:1861-1862`) | silent no-op |
| | `png_set_user_limits` | `png_ptr == NULL` (`pngset.c:1878-1879`) | silent `return` |
| | `png_set_chunk_cache_max` | `png_ptr == NULL` (`pngset.c:1891-1892`) | silent no-op |
| | `png_set_chunk_malloc_max` | `png_ptr == NULL` (`pngset.c:1907`) | silent no-op |
| | `png_set_chunk_malloc_max` | `user_chunk_malloc_max == 0U` (request for "unlimited") (`pngset.c:1909-1916`) | value replaced by `PNG_SIZE_MAX` (or `65536U` under `PNG_MAX_MALLOC_64K`) |
| | `png_check_keyword` | `key == NULL` (`pngset.c:1992-1996`) | `*new_key = 0`, `return 0` (caller must `png_error`) |
| | `png_check_keyword` | character not in `(32 < ch <= 126)` nor `ch >= 161` (control chars, 0x80..0xA0 incl. non-break space) (`pngset.c:2002-2020`) | character dropped/collapsed to a single space, first offender recorded in `bad_character` |
| | `png_check_keyword` | trailing space, i.e. `key_len > 0 && space != 0` (`pngset.c:2023-2028`) | trailing space removed, `bad_character = 32` |
| | `png_check_keyword` | resulting `key_len == 0` (keyword empty or all-invalid) (`pngset.c:2033-2034`) | `return 0` (caller must `png_error`) |
| | `png_check_keyword` | keyword longer than 79 characters, so `*key != 0` after the `key_len < 79` loop (`pngset.c:1998`, `:2038-2039`) | `png_warning(png_ptr, "keyword truncated")`, keyword truncated to 79 chars |
| | `png_check_keyword` | `bad_character != 0` (`pngset.c:2041-2049`) | `png_formatted_warning(png_ptr, p, "keyword \"@1\": bad character '0x@2'")` |
| | `png_set_bgr` | `png_ptr == NULL` (`pngtrans.c:24-25`) | silent `return` |
| | `png_set_swap` | `png_ptr == NULL` (`pngtrans.c:38-39`) | silent `return` |
| | `png_set_swap` | `png_ptr->bit_depth != 16` (`pngtrans.c:41`) | request silently ignored; `PNG_SWAP_BYTES` not set |
| | `png_set_packing` | `png_ptr == NULL` (`pngtrans.c:53-54`) | silent `return` |
| | `png_set_packing` | `png_ptr->bit_depth >= 8` (`pngtrans.c:56`) | request silently ignored; `PNG_PACK` not set |
| | `png_set_packswap` | `png_ptr == NULL` (`pngtrans.c:73-74`) | silent `return` |
| | `png_set_packswap` | `png_ptr->bit_depth >= 8` (`pngtrans.c:76`) | request silently ignored; `PNG_PACKSWAP` not set |
| | `png_set_shift` | `png_ptr == NULL \|\| true_bits == NULL` (`pngtrans.c:87-88`) | silent `return` |
| | `png_set_shift` | color image (`color_type & PNG_COLOR_MASK_COLOR`) and `true_bits->red == 0 \|\| red > bit_depth \|\| green == 0 \|\| green > bit_depth \|\| blue == 0 \|\| blue > bit_depth` (`pngtrans.c:95-101`, `:112-116`) | `png_app_error(png_ptr, "png_set_shift: invalid shift values")`, `return` |
| | `png_set_shift` | grayscale image and `true_bits->gray == 0 \|\| true_bits->gray > bit_depth` (`pngtrans.c:102-106`, `:112-116`) | `png_app_error(png_ptr, "png_set_shift: invalid shift values")`, `return` |
| | `png_set_shift` | `(color_type & PNG_COLOR_MASK_ALPHA) != 0 && (true_bits->alpha == 0 \|\| true_bits->alpha > bit_depth)` (`pngtrans.c:108-110`, `:112-116`) | `png_app_error(png_ptr, "png_set_shift: invalid shift values")`, `return` |
| | `png_set_interlace_handling` | `png_ptr == 0 \|\| png_ptr->interlaced == 0` (`pngtrans.c:131`, `:137`) | `return 1` (single pass; `PNG_INTERLACE` not set) |
| | `png_set_filler` | `png_ptr == NULL` (`pngtrans.c:152-153`) | silent `return` |
| | `png_set_filler` | read struct in a build without `PNG_READ_FILLER_SUPPORTED` (`pngtrans.c:158`, `:171-173`) | `png_app_error(png_ptr, "png_set_filler not supported on read")`, `return` |
| | `png_set_filler` | write with `color_type == PNG_COLOR_TYPE_GRAY && png_ptr->bit_depth < 8` (`pngtrans.c:189-206`) | `png_app_error(png_ptr, "png_set_filler is invalid for low bit depth gray output")`, `return` |
| | `png_set_filler` | write with `color_type` other than `PNG_COLOR_TYPE_RGB`/`PNG_COLOR_TYPE_GRAY` (palette, GA, RGBA) (`pngtrans.c:208-211`) | `png_app_error(png_ptr, "png_set_filler: inappropriate color type")`, `return` |
| | `png_set_filler` | write struct in a build without `PNG_WRITE_FILLER_SUPPORTED` (`pngtrans.c:213-215`) | `png_app_error(png_ptr, "png_set_filler not supported on write")`, `return` |
| | `png_set_add_alpha` | `png_ptr == NULL` (`pngtrans.c:237-238`) | silent `return` |
| | `png_set_add_alpha` | `png_set_filler()` failed so `(png_ptr->transformations & PNG_FILLER) == 0` (`pngtrans.c:242-243`) | `PNG_ADD_ALPHA` not set (error already reported by `png_set_filler`) |
| | `png_set_swap_alpha` | `png_ptr == NULL` (`pngtrans.c:255-256`) | silent `return` |
| | `png_set_invert_alpha` | `png_ptr == NULL` (`pngtrans.c:269-270`) | silent `return` |
| | `png_set_invert_mono` | `png_ptr == NULL` (`pngtrans.c:282-283`) | silent `return` |
| | `png_do_invert` | `row_info->color_type` is neither `PNG_COLOR_TYPE_GRAY`, nor `PNG_COLOR_TYPE_GRAY_ALPHA` with `bit_depth` 8 or 16 (`pngtrans.c:297`, `:310-311`, `:325-326`) | row left unchanged (no branch taken) |
| | `png_do_swap` | `row_info->bit_depth != 16` (`pngtrans.c:351`) | row left unchanged |
| | `png_do_packswap` | `row_info->bit_depth >= 8` (`pngtrans.c:487`) | row left unchanged |
| | `png_do_packswap` | `row_info->bit_depth < 8` but not 1, 2 or 4 (e.g. 3, 5, 6, 7) (`pngtrans.c:493-503`) | `return` (no swap table) |
| | `png_do_strip_channel` | `row_info->channels == 2` and `row_info->bit_depth` neither 8 nor 16 (`pngtrans.c:541`, `:559`, `:576-577`) | `return` (bad bit depth; row and `rowbytes` untouched) |
| | `png_do_strip_channel` | `row_info->channels == 4` and `row_info->bit_depth` neither 8 nor 16 (`pngtrans.c:589`, `:607`, `:627-628`) | `return` (bad bit depth) |
| | `png_do_strip_channel` | `row_info->channels` neither 2 nor 4 (filler channel already gone) (`pngtrans.c:637-638`) | `return` |
| | `png_do_bgr` | `(row_info->color_type & PNG_COLOR_MASK_COLOR) == 0` (grayscale) (`pngtrans.c:652`) | row left unchanged |
| | `png_do_bgr` | color row with `bit_depth` neither 8 nor 16 (`pngtrans.c:655`, `:685`) | row left unchanged |
| | `png_do_bgr` | `color_type` neither `PNG_COLOR_TYPE_RGB` nor `PNG_COLOR_TYPE_RGB_ALPHA` inside the 8-/16-bit branches (`pngtrans.c:657`, `:670`, `:687`, `:703`) | row left unchanged |
| | `png_do_check_palette_indexes` | `png_ptr->num_palette >= (1 << row_info->bit_depth) \|\| png_ptr->num_palette == 0` (complete palette, or MNG empty palette) (`pngtrans.c:732-733`) | no index checking performed; `num_palette_max` untouched |
| | `png_do_check_palette_indexes` | `row_info->bit_depth` not in {1,2,4,8} (`default:` case) (`pngtrans.c:822-823`) | `break` — no index checking performed |
| | `png_set_user_transform_info` | `png_ptr == NULL` (`pngtrans.c:838-839`) | silent `return` |
| | `png_set_user_transform_info` | read struct with `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (called too late) (`pngtrans.c:842-848`) | `png_app_error(png_ptr, "info change after png_start_read_image or png_read_update_info")`, `return` |
| | `png_get_user_transform_ptr` | `png_ptr == NULL` (`pngtrans.c:866-867`) | `return NULL` |
| | `png_get_current_row_number` | `png_ptr == NULL` (`pngtrans.c:880-883`) | `return PNG_UINT_32_MAX` |
| | `png_get_current_pass_number` | `png_ptr == NULL` (`pngtrans.c:889-891`) | `return 8` (invalid pass) |
