| | `png_create_read_struct` / `png_create_read_struct_2` | `png_create_png_struct` fails (out of memory, or `user_png_ver` incompatible with the library) — `png_ptr` stays `NULL` (pngread.c:30, 46, 50, 79) | returns `NULL`; no read struct created |
| | `png_read_info` | `png_ptr == NULL \|\| info_ptr == NULL` (pngread.c:101-102) | silent `return`; nothing read |
| | `png_read_info` | first `IDAT` seen while `(png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngread.c:117-118) | `png_chunk_error(png_ptr, "Missing IHDR before IDAT")` — fatal |
| | `png_read_info` | `IDAT` when `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0` (pngread.c:120-122) | `png_chunk_error(png_ptr, "Missing PLTE before IDAT")` — fatal |
| | `png_read_info` | `IDAT` seen after a non-IDAT chunk already followed IDAT: `(png_ptr->mode & PNG_AFTER_IDAT) != 0` (pngread.c:124-125) | `png_chunk_benign_error(png_ptr, "Too many IDATs found")` (error or warning per benign-error flag) |
| | `png_read_update_info` | `png_ptr == NULL` (pngread.c:176) | silent `return`; info not updated |
| | `png_read_update_info` | called twice / after `png_start_read_image`: `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (pngread.c:178, 191-192) | `png_app_error(png_ptr, "png_read_update_info/png_start_read_image: duplicate call")` |
| | `png_start_read_image` | `png_ptr == NULL` (pngread.c:207) | silent `return` |
| | `png_start_read_image` | called twice / after `png_read_update_info`: `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (pngread.c:209, 214-215) | `png_app_error(png_ptr, "png_start_read_image/png_read_update_info: duplicate call")` |
| | `png_do_read_intrapixel` | MNG intrapixel differencing at `bit_depth == 8` with a color type that is neither `PNG_COLOR_TYPE_RGB` nor `PNG_COLOR_TYPE_RGB_ALPHA` (pngread.c:241-248) | early `return`; row left untransformed |
| | `png_do_read_intrapixel` | MNG intrapixel differencing at `bit_depth == 16` with a color type that is neither `PNG_COLOR_TYPE_RGB` nor `PNG_COLOR_TYPE_RGB_ALPHA` (pngread.c:261-268) | early `return`; row left untransformed |
| | `png_read_row` | `png_ptr == NULL` (pngread.c:292-293) | silent `return` |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_INVERT_MONO) != 0` but `PNG_READ_INVERT_SUPPORTED` not compiled in (pngread.c:317-318) | `png_warning(png_ptr, "PNG_READ_INVERT_SUPPORTED is not defined")`; transform skipped |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_FILLER) != 0` but `PNG_READ_FILLER_SUPPORTED` not compiled in (pngread.c:322-323) | `png_warning(png_ptr, "PNG_READ_FILLER_SUPPORTED is not defined")` |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_PACKSWAP) != 0` but `PNG_READ_PACKSWAP_SUPPORTED` not compiled in (pngread.c:328-329) | `png_warning(png_ptr, "PNG_READ_PACKSWAP_SUPPORTED is not defined")` |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_PACK) != 0` but `PNG_READ_PACK_SUPPORTED` not compiled in (pngread.c:333-334) | `png_warning(png_ptr, "PNG_READ_PACK_SUPPORTED is not defined")` |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_SHIFT) != 0` but `PNG_READ_SHIFT_SUPPORTED` not compiled in (pngread.c:338-339) | `png_warning(png_ptr, "PNG_READ_SHIFT_SUPPORTED is not defined")` |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_BGR) != 0` but `PNG_READ_BGR_SUPPORTED` not compiled in (pngread.c:343-344) | `png_warning(png_ptr, "PNG_READ_BGR_SUPPORTED is not defined")` |
| | `png_read_row` | on first row, `(png_ptr->transformations & PNG_SWAP_BYTES) != 0` but `PNG_READ_SWAP_SUPPORTED` not compiled in (pngread.c:348-349) | `png_warning(png_ptr, "PNG_READ_SWAP_SUPPORTED is not defined")` |
| | `png_read_row` | row data requested but no IDAT has been reached: `(png_ptr->mode & PNG_HAVE_IDAT) == 0` (pngread.c:443-444) | `png_error(png_ptr, "Invalid attempt to read row data")` — fatal |
| | `png_read_row` | filter byte in the decompressed row `png_ptr->row_buf[0] >= PNG_FILTER_VALUE_LAST` (i.e. > 4; includes the sentinel 255 written when no data was produced) (pngread.c:450-456) | `png_error(png_ptr, "bad adaptive filter value")` — fatal |
| | `png_read_row` | after transforms, first row's `row_info.pixel_depth > png_ptr->maximum_pixel_depth` (pngread.c:485-489) | `png_error(png_ptr, "sequential row overflow")` — fatal |
| | `png_read_row` | later row's `png_ptr->transformed_pixel_depth != row_info.pixel_depth` (pngread.c:492-493) | `png_error(png_ptr, "internal sequential row size calculation error")` — fatal |
| | `png_read_rows` | `png_ptr == NULL` (pngread.c:562-563) | silent `return` |
| | `png_read_rows` | both `row == NULL` and `display_row == NULL` (pngread.c:567-590) | no branch taken; no rows read, no error reported |
| | `png_read_image` | `png_ptr == NULL` (pngread.c:616-617) | silent `return` |
| | `png_read_image` | interlaced file where the app called `png_start_read_image`/`png_read_update_info` without enabling `PNG_INTERLACE`: `png_ptr->interlaced != 0 && (png_ptr->transformations & PNG_INTERLACE) == 0` (pngread.c:628-638) | `png_warning(png_ptr, "Interlace handling should be turned on when using png_read_image")`; `num_rows` forced to `height` |
| | `png_read_image` | interlaced file (`png_ptr->interlaced` non-zero) in a build without `PNG_READ_INTERLACING_SUPPORTED` (pngread.c:647-649) | `png_error(png_ptr, "Cannot read interlaced image -- interlace handler disabled")` — fatal |
| | `png_read_end` | `png_ptr == NULL` (pngread.c:682-683) | silent `return` |
| | `png_read_end` | palette image where a pixel index exceeded the palette: `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= png_ptr->num_palette` (pngread.c:695-697) | `png_benign_error(png_ptr, "Read palette index exceeding num_palette")` |
| | `png_read_end` | IDAT handled via unknown-chunk path with `(length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0` (pngread.c:725-729) | `png_benign_error(png_ptr, ".Too many IDATs found")` |
| | `png_read_end` | trailing IDAT with `(length > 0 && !(png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED)) \|\| (png_ptr->mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0` (pngread.c:737-747) | `png_benign_error(png_ptr, "..Too many IDATs found")`, then chunk CRC-skipped |
| | `png_destroy_read_struct` | `png_ptr_ptr == NULL` or `*png_ptr_ptr == NULL` (pngread.c:837-841) | silent `return`; nothing freed |
| | `png_set_read_status_fn` | `png_ptr == NULL` (pngread.c:858-859) | silent `return` |
| | `png_read_png` | `png_ptr == NULL \|\| info_ptr == NULL` (pngread.c:873-874) | silent `return` |
| | `png_read_png` | `info_ptr->height > PNG_UINT_32_MAX/(sizeof (png_bytep))` (row-pointer array would overflow) (pngread.c:880-881) | `png_error(png_ptr, "Image is too high to process with png_read_png()")` — fatal |
| | `png_read_png` | `transforms & PNG_TRANSFORM_SCALE_16` in a build without `PNG_READ_SCALE_16_TO_8_SUPPORTED` (pngread.c:892-900) | `png_app_error(png_ptr, "PNG_TRANSFORM_SCALE_16 not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_STRIP_16` without `PNG_READ_STRIP_16_TO_8_SUPPORTED` (pngread.c:906-911) | `png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_16 not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_STRIP_ALPHA` without `PNG_READ_STRIP_ALPHA_SUPPORTED` (pngread.c:916-921) | `png_app_error(png_ptr, "PNG_TRANSFORM_STRIP_ALPHA not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_PACKING` without `PNG_READ_PACK_SUPPORTED` (pngread.c:926-931) | `png_app_error(png_ptr, "PNG_TRANSFORM_PACKING not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_PACKSWAP` without `PNG_READ_PACKSWAP_SUPPORTED` (pngread.c:936-941) | `png_app_error(png_ptr, "PNG_TRANSFORM_PACKSWAP not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_EXPAND` without `PNG_READ_EXPAND_SUPPORTED` (pngread.c:948-953) | `png_app_error(png_ptr, "PNG_TRANSFORM_EXPAND not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_INVERT_MONO` without `PNG_READ_INVERT_SUPPORTED` (pngread.c:960-965) | `png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_MONO not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_SHIFT` without `PNG_READ_SHIFT_SUPPORTED` (pngread.c:971-977) | `png_app_error(png_ptr, "PNG_TRANSFORM_SHIFT not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_SHIFT` requested but file has no sBIT: `(info_ptr->valid & PNG_INFO_sBIT) == 0` (pngread.c:971-974) | `png_set_shift` not called; transform silently ignored, no diagnostic |
| | `png_read_png` | `transforms & PNG_TRANSFORM_BGR` without `PNG_READ_BGR_SUPPORTED` (pngread.c:980-985) | `png_app_error(png_ptr, "PNG_TRANSFORM_BGR not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_SWAP_ALPHA` without `PNG_READ_SWAP_ALPHA_SUPPORTED` (pngread.c:988-993) | `png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ALPHA not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_SWAP_ENDIAN` without `PNG_READ_SWAP_SUPPORTED` (pngread.c:996-1001) | `png_app_error(png_ptr, "PNG_TRANSFORM_SWAP_ENDIAN not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_INVERT_ALPHA` without `PNG_READ_INVERT_ALPHA_SUPPORTED` (pngread.c:1005-1010) | `png_app_error(png_ptr, "PNG_TRANSFORM_INVERT_ALPHA not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_GRAY_TO_RGB` without `PNG_READ_GRAY_TO_RGB_SUPPORTED` (pngread.c:1014-1019) | `png_app_error(png_ptr, "PNG_TRANSFORM_GRAY_TO_RGB not supported")` |
| | `png_read_png` | `transforms & PNG_TRANSFORM_EXPAND_16` without `PNG_READ_EXPAND_16_SUPPORTED` (pngread.c:1022-1027) | `png_app_error(png_ptr, "PNG_TRANSFORM_EXPAND_16 not supported")` |
| | `png_image_read_init` | `image->opaque != NULL` on entry (image already in use / not zeroed) (pngread.c:1130, 1172) | `png_image_error(image, "png_image_read: opaque pointer not NULL")` → returns 0 |
| | `png_image_read_init` | `png_create_read_struct`, `png_create_info_struct` or `png_malloc_warn(control)` returns `NULL` (pngread.c:1141-1169) | cleanup, then `png_image_error(image, "png_image_read: out of memory")` → returns 0 |
| | `chromaticities_match_sRGB` | any of `whitex/whitey/redx/redy/greenx/greeny/bluex/bluey` differs from the BT.709 sRGB value by more than `sRGB_TOLERANCE` (1000) — `PNG_OUT_OF_RANGE(...)` (pngread.c:1217-1225) | `return 0`; caller marks image `PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB` |
| | `png_gamma_not_sRGB` | `g < PNG_LIB_GAMMA_MIN \|\| g > PNG_LIB_GAMMA_MAX` (includes uninitialized `g == 0`) (pngread.c:1238-1239) | `return 0`; treated as "same as sRGB" (no gamma work) |
| | `png_image_read_header` | computed `cmap_entries > 256` (e.g. `1U << bit_depth` for gray, or `num_palette`) (pngread.c:1326-1327) | clamped: `cmap_entries = 256` |
| | `png_image_begin_read_from_stdio` | `file == NULL` (pngread.c:1341, 1354-1356) | `png_image_error(image, "png_image_begin_read_from_stdio: invalid argument")` → returns 0 |
| | `png_image_begin_read_from_stdio` | `image->version != PNG_IMAGE_VERSION` (pngread.c:1339, 1359-1361) | `png_image_error(image, "png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION")` → returns 0 |
| | `png_image_begin_read_from_stdio` | `image == NULL` (pngread.c:1339, 1363) | `return 0`; no error text recordable |
| | `png_image_begin_read_from_stdio` | `png_image_read_init(image) == 0` (allocation failure) (pngread.c:1343, 1363) | falls through, `return 0` (message set by `png_image_read_init`) |
| | `png_image_begin_read_from_file` | `file_name == NULL` (pngread.c:1371, 1392-1394) | `png_image_error(image, "png_image_begin_read_from_file: invalid argument")` → returns 0 |
| | `png_image_begin_read_from_file` | `fopen(file_name, "rb") == NULL` (pngread.c:1373-1389) | `png_image_error(image, strerror(errno))` → returns 0 |
| | `png_image_begin_read_from_file` | `image->version != PNG_IMAGE_VERSION` (pngread.c:1369, 1397-1399) | `png_image_error(image, "png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION")` → returns 0 |
| | `png_image_begin_read_from_file` | `image == NULL` (pngread.c:1369, 1401) | `return 0` |
| | `png_image_begin_read_from_file` | `png_image_read_init(image) == 0` after successful `fopen` (pngread.c:1377-1385, 1401) | file closed with `fclose`, `return 0` |
| | `png_image_memory_read` | request beyond the supplied buffer: `memory == NULL \|\| size < need` (pngread.c:1419-1427) | `png_error(png_ptr, "read beyond end of data")` — fatal (longjmp out of `png_safe_execute`) |
| | `png_image_memory_read` | `io_ptr` image is `NULL` or `image->opaque == NULL` (pngread.c:1410-1431) | `png_error(png_ptr, "invalid memory read")` — fatal |
| | `png_image_memory_read` | `png_ptr == NULL` (pngread.c:1408) | silent `return`; nothing copied into `out` |
| | `png_image_begin_read_from_memory` | `memory == NULL \|\| size == 0` (pngread.c:1440, 1457-1459) | `png_image_error(image, "png_image_begin_read_from_memory: invalid argument")` → returns 0 |
| | `png_image_begin_read_from_memory` | `image->version != PNG_IMAGE_VERSION` (pngread.c:1438, 1462-1464) | `png_image_error(image, "png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION")` → returns 0 |
| | `png_image_begin_read_from_memory` | `image == NULL` (pngread.c:1438, 1466) | `return 0` |
| | `png_image_begin_read_from_memory` | `png_image_read_init(image) == 0` (pngread.c:1442, 1466) | `return 0` |
| | `set_file_encoding` | `png_resolve_file_gamma(png_ptr) == 0` (no gAMA/sRGB/default gamma resolvable) (pngread.c:1531-1537) | `png_error(png_ptr, "internal: default gamma not set")` — fatal |
| | `decode_gamma` | `encoding` not one of `P_FILE/P_sRGB/P_LINEAR/P_LINEAR8` (GNUC `default:` arm) (pngread.c:1584-1588) | `png_error(png_ptr, "unexpected encoding (internal error)")` — fatal |
| | `png_create_colormap_entry` | color-map index `ip > 255` (pngread.c:1642-1643) | `png_error(image->opaque->png_ptr, "color-map index out of range")` — fatal |
| | `png_create_colormap_entry` | after conversion `encoding != output_encoding` (pngread.c:1742-1743) | `png_error(image->opaque->png_ptr, "bad encoding (internal error)")` — fatal |
| | `png_image_read_colormap` | input has alpha/tRNS, output format has no `PNG_FORMAT_FLAG_ALPHA`, output is sRGB, and `display->background == NULL` (pngread.c:1989-1998) | `png_error(png_ptr, "background color must be supplied to remove alpha/transparency")` — fatal |
| | `png_image_read_colormap` | gray, `bit_depth <= 8`: `(1U << bit_depth) > image->colormap_entries` (pngread.c:2054-2056) | `png_error(png_ptr, "gray[8] color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | gray, `bit_depth == 16`: `PNG_GRAY_COLORMAP_ENTRIES (256) > image->colormap_entries` (pngread.c:2134-2135) | `png_error(png_ptr, "gray[16] color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | GRAY_ALPHA with alpha kept: `PNG_GA_COLORMAP_ENTRIES (256) > image->colormap_entries` (pngread.c:2232-2233) | `png_error(png_ptr, "gray+alpha color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | GRAY_ALPHA, alpha removed on a gray background: `PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2266-2267) | `png_error(png_ptr, "gray-alpha color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | GRAY_ALPHA, alpha removed on a colored background: `PNG_GA_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2300-2301) | `png_error(png_ptr, "ga-alpha color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | RGB/RGBA → gray output with alpha in both: `PNG_GA_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2405-2406) | `png_error(png_ptr, "rgb[ga] color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | RGB/RGBA → gray output without alpha: `PNG_GRAY_COLORMAP_ENTRIES > image->colormap_entries` (pngread.c:2421-2422) | `png_error(png_ptr, "rgb[gray] color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | RGBA/tRNS → color output with alpha: `PNG_RGB_COLORMAP_ENTRIES+1+27 (244) > image->colormap_entries` (pngread.c:2529-2530) | `png_error(png_ptr, "rgb+alpha color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | RGBA/tRNS → color output, alpha removed: `PNG_RGB_COLORMAP_ENTRIES+1+27 > image->colormap_entries` (pngread.c:2578-2579) | `png_error(png_ptr, "rgb-alpha color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | opaque RGB → color output: `PNG_RGB_COLORMAP_ENTRIES (216) > image->colormap_entries` (pngread.c:2663-2664) | `png_error(png_ptr, "rgb color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | palette image with `png_ptr->num_palette > 256` (pngread.c:2690-2692) | clamped: `cmap_entries = 256` |
| | `png_image_read_colormap` | palette image: `cmap_entries > (unsigned int)image->colormap_entries` (pngread.c:2694-2695) | `png_error(png_ptr, "palette color-map: too few entries")` — fatal |
| | `png_image_read_colormap` | `png_ptr->color_type` not one of the 5 valid PNG color types (switch `default:`) (pngread.c:2737-2738) | `png_error(png_ptr, "invalid PNG color type")` — fatal |
| | `png_image_read_colormap` | `data_encoding` left as something other than `P_sRGB`/`P_FILE` (GNUC `default:`) (pngread.c:2759-2762) | `png_error(png_ptr, "bad data option (internal error)")` — fatal |
| | `png_image_read_colormap` | `cmap_entries > 256 \|\| cmap_entries > image->colormap_entries` after building the map (pngread.c:2765-2766) | `png_error(png_ptr, "color map overflow (BAD internal error)")` — fatal |
| | `png_image_read_colormap` | `output_processing` not one of the 5 `PNG_CMAP_*` values (switch `default:`) (pngread.c:2799-2800) | `png_error(png_ptr, "bad processing option (internal error)")` — fatal |
| | `png_image_read_colormap` | `PNG_CMAP_NONE` but `background_index != PNG_CMAP_NONE_BACKGROUND (256)` (pngread.c:2773-2775, 2802-2803) | `goto bad_background` → `png_error(png_ptr, "bad background index (internal error)")` |
| | `png_image_read_colormap` | `PNG_CMAP_GA` but `background_index != PNG_CMAP_GA_BACKGROUND (231)` (pngread.c:2778-2780, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` |
| | `png_image_read_colormap` | `PNG_CMAP_TRANS` but `background_index >= cmap_entries \|\| background_index != PNG_CMAP_TRANS_BACKGROUND (254)` (pngread.c:2783-2786, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` |
| | `png_image_read_colormap` | `PNG_CMAP_RGB` but `background_index != PNG_CMAP_RGB_BACKGROUND (256)` (pngread.c:2789-2791, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` |
| | `png_image_read_colormap` | `PNG_CMAP_RGB_ALPHA` but `background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND (216)` (pngread.c:2794-2796, 2802-2803) | `png_error(png_ptr, "bad background index (internal error)")` |
| | `png_image_read_and_map` | `png_ptr->interlaced` is neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:2825-2836) | `png_error(png_ptr, "unknown interlace type")` — fatal |
| | `png_image_read_colormapped` | `PNG_CMAP_NONE` but result is not `(PALETTE\|GRAY)` with `info_ptr->bit_depth == 8` (pngread.c:3031-3036, 3073-3074) | `goto bad_output` → `png_error(png_ptr, "bad color-map processing (internal error)")` |
| | `png_image_read_colormapped` | `PNG_CMAP_TRANS`/`PNG_CMAP_GA` but not `GRAY_ALPHA`, depth 8, `screen_gamma == PNG_GAMMA_sRGB`, `colormap_entries == 256` (pngread.c:3038-3050, 3073-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` |
| | `png_image_read_colormapped` | `PNG_CMAP_RGB` but not `RGB`, depth 8, sRGB screen gamma, `colormap_entries == 216` (pngread.c:3052-3060, 3073-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` |
| | `png_image_read_colormapped` | `PNG_CMAP_RGB_ALPHA` but not `RGB_ALPHA`, depth 8, sRGB screen gamma, `colormap_entries == 244` (pngread.c:3062-3070, 3073-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` |
| | `png_image_read_colormapped` | `display->colormap_processing` is not one of the 5 `PNG_CMAP_*` values (switch `default:`) (pngread.c:3072-3074) | `png_error(png_ptr, "bad color-map processing (internal error)")` |
| | `png_image_read_direct_scaled` | `png_ptr->interlaced` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:3148-3159) | `png_error(png_ptr, "unknown interlace type")` — fatal |
| | `png_image_read_composite` | `png_ptr->interlaced` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:3197-3208) | `png_error(png_ptr, "unknown interlace type")` — fatal |
| | `png_image_read_composite` | optimized-alpha path where the composed value exceeds the linear range: `component > 255*65535` (data not linear-premultiplied; CVE-2025-66293 hardening) (pngread.c:3290-3291) | clamped to `255*65535` before `PNG_sRGB_FROM_LINEAR` |
| | `png_image_read_composite` | non-optimized path where `component > 255` after compositing (pngread.c:3309-3310) | clamped to `255` |
| | `png_image_read_background` | `(png_ptr->transformations & PNG_RGB_TO_GRAY) == 0` on entry (pngread.c:3357-3358) | `png_error(png_ptr, "lost rgb to gray")` — fatal |
| | `png_image_read_background` | `(png_ptr->transformations & PNG_COMPOSE) != 0` on entry (pngread.c:3360-3361) | `png_error(png_ptr, "unexpected compose")` — fatal |
| | `png_image_read_background` | `png_get_channels(png_ptr, info_ptr) != 2` (pngread.c:3363-3364) | `png_error(png_ptr, "lost/gained channels")` — fatal |
| | `png_image_read_background` | 8-bit output that still carries alpha: `(image->format & PNG_FORMAT_FLAG_LINEAR) == 0 && (image->format & PNG_FORMAT_FLAG_ALPHA) != 0` (pngread.c:3367-3369) | `png_error(png_ptr, "unexpected 8-bit transformation")` — fatal |
| | `png_image_read_background` | `png_ptr->interlaced` neither `PNG_INTERLACE_NONE` nor `PNG_INTERLACE_ADAM7` (pngread.c:3371-3382) | `png_error(png_ptr, "unknown interlace type")` — fatal |
| | `png_image_read_background` | `info_ptr->bit_depth` neither 8 nor 16 (GNUC `default:`) (pngread.c:3390, 3607-3610) | `png_error(png_ptr, "unexpected bit depth")` — fatal |
| | `png_image_read_direct` | requested `image->format` needs a transform libpng cannot supply: `change != 0` after all transform handling (pngread.c:3924-3925) | `png_error(png_ptr, "png_read_image: unsupported transformation")` — fatal |
| | `png_image_read_direct` | `do_local_compose != 0` yet `info_ptr->color_type` has no alpha channel (pngread.c:3959-3960) | `png_error(png_ptr, "png_image_read: alpha channel lost")` — fatal |
| | `png_image_read_direct` | `do_local_background == 2` while libpng has `PNG_SWAP_ALPHA`/front-filler `PNG_ADD_ALPHA` set (pngread.c:3981-3986) | `png_error(png_ptr, "unexpected alpha swap transformation")` — fatal |
| | `png_image_read_direct` | the format libpng will actually produce does not match the requested one: `info_format != format` (pngread.c:3993-3994) | `png_error(png_ptr, "png_read_image: invalid transformations")` — fatal |
| | `png_image_finish_read` | `image->width > 0x7fffffffU/channels` (row stride cannot be represented in a signed 32-bit value) (pngread.c:4105, 4192-4194) | `png_image_error(image, "png_image_finish_read: row_stride too large")` → returns 0 |
| | `png_image_finish_read` | `image->opaque == NULL \|\| buffer == NULL \|\| check < png_row_stride` (no begin_read, no output buffer, or stride smaller than one row) (pngread.c:4123, 4187-4189) | `png_image_error(image, "png_image_finish_read: invalid argument")` → returns 0 |
| | `png_image_finish_read` | `image->height > 0xffffffffU/PNG_IMAGE_PIXEL_COMPONENT_SIZE(image->format)/check` (buffer-size calculation overflows 32 bits) (pngread.c:4141-4142, 4182-4184) | `png_image_error(image, "png_image_finish_read: image too large")` → returns 0 |
| | `png_image_finish_read` | color-mapped output requested but `image->colormap_entries == 0` or `colormap == NULL` (pngread.c:4144-4145, 4177-4179) | `png_image_error(image, "png_image_finish_read[color-map]: no color-map")` → returns 0 |
| | `png_image_finish_read` | `image->version != PNG_IMAGE_VERSION` (pngread.c:4091, 4197-4199) | `png_image_error(image, "png_image_finish_read: damaged PNG_IMAGE_VERSION")` → returns 0 |
| | `png_image_finish_read` | `image == NULL` (pngread.c:4091, 4201) | `return 0` |
| | `png_set_crc_action` | `png_ptr == NULL` (pngrtran.c:45-46) | silent `return` |
| | `png_set_crc_action` | `crit_action == PNG_CRC_WARN_DISCARD` (discarding critical data is not allowed) (pngrtran.c:65-67) | `png_warning(png_ptr, "Can't discard critical data on CRC error")`, then falls through to the default (error/quit) behavior |
| | `png_set_crc_action` | `crit_action` is not a recognized `PNG_CRC_*` value (switch `default:`) (pngrtran.c:71-74) | silently reset to default: `png_ptr->flags &= ~PNG_FLAG_CRC_CRITICAL_MASK` |
| | `png_set_crc_action` | `ancil_action` is not a recognized `PNG_CRC_*` value (switch `default:`) (pngrtran.c:101-104) | silently reset to default: `png_ptr->flags &= ~PNG_FLAG_CRC_ANCILLARY_MASK` |
| | `png_rtran_ok` | any read-transform setter called after row init: `(png_ptr->flags & PNG_FLAG_ROW_INIT) != 0` (pngrtran.c:119-121) | `png_app_error(png_ptr, "invalid after png_start_read_image or png_read_update_info")`, `return 0` |
| | `png_rtran_ok` | transform requiring IHDR called too early: `need_IHDR && (png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngrtran.c:123-124) | `png_app_error(png_ptr, "invalid before the PNG header has been read")`, `return 0` |
| | `png_rtran_ok` | `png_ptr == NULL` (pngrtran.c:117, 135) | `return 0` (no `png_error` possible); caller aborts silently |
| | `png_set_background_fixed` | `png_rtran_ok(png_ptr, 0) == 0` or `background_color == NULL` (pngrtran.c:148-149) | `return`; background not set |
| | `png_set_background_fixed` | `background_gamma_code == PNG_BACKGROUND_GAMMA_UNKNOWN` (pngrtran.c:151-155) | `png_warning(png_ptr, "Application must supply a known background gamma")` and `return` |
| | `png_set_scale_16` | `png_rtran_ok(png_ptr, 0) == 0` (NULL ptr, or after row init) (pngrtran.c:192-193) | `return`; `PNG_SCALE_16_TO_8` not set |
| | `png_set_strip_16` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:206-207) | `return`; `PNG_16_TO_8` not set |
| | `png_set_strip_alpha` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:219-220) | `return`; `PNG_STRIP_ALPHA` not set |
| | `convert_gamma_value` | `output_gamma > PNG_FP_MAX \|\| output_gamma < PNG_FP_MIN` after scaling/rounding (pngrtran.c:324-325) | `png_fixed_error(png_ptr, "gamma value")` — fatal |
| | `unsupported_gamma` | `gamma < PNG_LIB_GAMMA_MIN \|\| gamma > PNG_LIB_GAMMA_MAX` with `warn != 0` (called from `png_set_gamma_fixed`) (pngrtran.c:344-351) | `png_app_warning(png_ptr, "gamma out of supported range")`, returns 1 → caller returns without setting gamma |
| | `unsupported_gamma` | `gamma < PNG_LIB_GAMMA_MIN \|\| gamma > PNG_LIB_GAMMA_MAX` with `warn == 0` (called from `png_set_alpha_mode_fixed`) (pngrtran.c:344-350) | `png_app_error(png_ptr, "gamma out of supported range")`, returns 1 → caller returns |
| | `png_set_alpha_mode_fixed` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:369-370) | `return`; alpha mode not set |
| | `png_set_alpha_mode_fixed` | translated `output_gamma` outside `[PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX]` (pngrtran.c:372-374) | `png_app_error` via `unsupported_gamma`, then `return` |
| | `png_set_alpha_mode_fixed` | `mode` not one of `PNG_ALPHA_PNG/ASSOCIATED/OPTIMIZED/BROKEN` (switch `default:`) (pngrtran.c:433-434) | `png_error(png_ptr, "invalid alpha mode")` — fatal |
| | `png_set_alpha_mode_fixed` | pre-multiplying mode requested when `PNG_COMPOSE` is already set (i.e. `png_set_background` already called) (pngrtran.c:451-453) | `png_error(png_ptr, "conflicting calls to set alpha mode and background")` — fatal |
| | `png_set_quantize` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:495-496) | `return`; quantize not enabled |
| | `png_set_quantize` | `palette == NULL` (pngrtran.c:498-499) | `return`; quantize not enabled |
| | `png_set_quantize` | `num_palette > maximum_colors` (pngrtran.c:524, 814) | palette reduced in place; `num_palette = maximum_colors` (no diagnostic) |
| | `png_set_quantize` | `png_malloc_warn` for a `png_dsort` node returns `NULL` (`t == NULL`) during the no-histogram reduction (pngrtran.c:708-712, 720-721) | loops `break` out early; reduction abandoned for that pass, no error reported |
| | `png_set_gamma_fixed` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:898-899) | `return`; gamma not set |
| | `png_set_gamma_fixed` | `file_gamma <= 0` after flag translation (pngrtran.c:916-917) | `png_app_error(png_ptr, "invalid file gamma in png_set_gamma")` |
| | `png_set_gamma_fixed` | `scrn_gamma <= 0` after flag translation (pngrtran.c:918-919) | `png_app_error(png_ptr, "invalid screen gamma in png_set_gamma")` |
| | `png_set_gamma_fixed` | `file_gamma` outside `[PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX]` (pngrtran.c:921) | `png_app_warning(png_ptr, "gamma out of supported range")` then `return`; neither gamma stored |
| | `png_set_gamma_fixed` | `scrn_gamma` outside `[PNG_LIB_GAMMA_MIN, PNG_LIB_GAMMA_MAX]` (pngrtran.c:922) | `png_app_warning(png_ptr, "gamma out of supported range")` then `return`; neither gamma stored |
| | `png_set_expand` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:953-954) | `return`; `PNG_EXPAND\|PNG_EXPAND_tRNS` not set |
| | `png_set_palette_to_rgb` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:983-984) | `return`; transform not set |
| | `png_set_expand_gray_1_2_4_to_8` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:995-996) | `return`; `PNG_EXPAND` not set |
| | `png_set_tRNS_to_alpha` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:1007-1008) | `return`; transform not set |
| | `png_set_expand_16` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:1023-1024) | `return`; `PNG_EXPAND_16` not set |
| | `png_set_gray_to_rgb` | `png_rtran_ok(png_ptr, 0) == 0` (pngrtran.c:1036-1037) | `return`; `PNG_GRAY_TO_RGB` not set |
| | `png_set_rgb_to_gray_fixed` | `png_rtran_ok(png_ptr, 1) == 0` (NULL ptr, after row init, or before IHDR) (pngrtran.c:1054-1055) | `return`; rgb-to-gray not set |
| | `png_set_rgb_to_gray_fixed` | `error_action` not `PNG_ERROR_ACTION_NONE/WARN/ERROR` (switch `default:`) (pngrtran.c:1071-1072) | `png_error(png_ptr, "invalid error action to rgb_to_gray")` — fatal |
| | `png_set_rgb_to_gray_fixed` | `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE` in a build without `PNG_READ_EXPAND_SUPPORTED` (pngrtran.c:1075-1088) | `png_error(png_ptr, "Cannot do RGB_TO_GRAY without EXPAND_SUPPORTED")` — fatal |
| | `png_set_rgb_to_gray_fixed` | `red >= 0 && green >= 0` but `red + green > PNG_FP_1` (pngrtran.c:1090, 1107-1109) | `png_app_warning(png_ptr, "ignoring out of range rgb_to_gray coefficients")`; default coefficients kept |
| | `png_set_rgb_to_gray_fixed` | `red < 0` or `green < 0` (pngrtran.c:1090, 1107) | neither branch taken: coefficients silently left at their defaults, no diagnostic |
| | `png_resolve_file_gamma` | `file_gamma`, `chunk_gamma`, `default_gamma` and `screen_gamma` are all 0 (or `png_reciprocal` overflows) (pngrtran.c:1365-1384) | returns 0 → "no usable file gamma"; callers must treat gamma handling as disabled |
| | `png_init_gamma_values` | resolved `file_gamma <= 0` (nothing set) (pngrtran.c:1402, 1414-1415) | `file_gamma = screen_gamma = PNG_FP_1`; gamma correction suppressed (returns 0) |
| | `png_init_read_transformations` | `PNG_STRIP_ALPHA` set with no `PNG_COMPOSE` (pngrtran.c:1491-1510) | `PNG_BACKGROUND_EXPAND\|PNG_ENCODE_ALPHA\|PNG_EXPAND_tRNS` cleared and `png_ptr->num_trans = 0`; tRNS data silently discarded |
| | `png_init_read_transformations` | gamma correction combined with background composition and rgb-to-gray: `PNG_COMPOSE` and `PNG_RGB_TO_GRAY` both set with gamma tables built (pngrtran.c:1696-1698) | `png_warning(png_ptr, "libpng does not support gamma+background+rgb_to_gray")`; result is double gamma corrected |
| | `png_init_read_transformations` | `png_ptr->background_gamma_type` not `SCREEN`/`FILE`/`UNIQUE` for a non-palette image (switch `default:`) (pngrtran.c:1885-1886) | `png_error(png_ptr, "invalid background gamma type")` — fatal |
| | `png_init_read_transformations` | palette + `PNG_SHIFT` where red sBIT gives `shift = 8 - sig_bit.red` outside `0 < shift < 8` (e.g. `sig_bit.red == 0` or `>= 8`) (pngrtran.c:2025, 2033) | shift silently not applied to red palette entries ("error condition which is silently ignored") |
| | `png_init_read_transformations` | palette + `PNG_SHIFT` where `shift = 8 - sig_bit.green` is outside `0 < shift < 8` (pngrtran.c:2042-2043) | shift silently not applied to green palette entries |
| | `png_init_read_transformations` | palette + `PNG_SHIFT` where `shift = 8 - sig_bit.blue` is outside `0 < shift < 8` (pngrtran.c:2052-2053) | shift silently not applied to blue palette entries |
| | `png_read_transform_info` | `PNG_EXPAND` on a palette image with `png_ptr->palette == NULL` (pngrtran.c:2086-2104) | `png_error(png_ptr, "Palette is NULL in indexed image")` — fatal |
| | `png_do_unshift` | for any channel `shift[c] <= 0 \|\| shift[c] >= bit_depth` (sBIT value 0, or >= bit depth) (pngrtran.c:2427-2433) | that channel's `shift[c]` forced to 0 (invalid sBIT silently ignored) |
| | `png_do_unshift` | all channels end up with zero shift: `have_shift == 0` (pngrtran.c:2439-2440) | early `return`; row unchanged |
| | `png_do_unshift` | `bit_depth` is 1 (or otherwise unexpected) — switch `default:` "should not be here" (pngrtran.c:2443-2448) | `break` with no processing; row left unshifted |
| | `png_do_encode_alpha` | called with a row that has no alpha channel, or bit depth not 8/16, or the required `gamma_from_1`/`gamma_16_from_1` table is `NULL` (pngrtran.c:4292-4341) | `png_warning(png_ptr, "png_do_encode_alpha: unexpected call")`; row not encoded |
| | `png_do_expand_palette` | palette index in the row is >= `num_trans`: `(int)(*sp) >= num_trans` while expanding tRNS (pngrtran.c:4471-4476) | alpha defaulted to `0xff` (opaque) instead of reading past `trans_alpha[]` |
| | `png_do_read_transformations` | `png_ptr->row_buf == NULL` (pngrtran.c:4885-4891) | `png_error(png_ptr, "NULL row buffer")` — fatal |
| | `png_do_read_transformations` | transforms set but neither `png_start_read_image` nor `png_read_update_info` called: `(flags & PNG_FLAG_DETECT_UNINITIALIZED) != 0 && (flags & PNG_FLAG_ROW_INIT) == 0` (pngrtran.c:4900-4907) | `png_error(png_ptr, "Uninitialized row")` — fatal |
| | `png_do_read_transformations` | non-gray pixel found during rgb-to-gray with `PNG_RGB_TO_GRAY_WARN` requested (pngrtran.c:4960-4965) | `png_warning(png_ptr, "png_do_rgb_to_gray found nongray pixel")`; `rgb_to_gray_status = 1` |
| | `png_do_read_transformations` | non-gray pixel found during rgb-to-gray with `PNG_RGB_TO_GRAY_ERR` requested (pngrtran.c:4967-4969) | `png_error(png_ptr, "png_do_rgb_to_gray found nongray pixel")` — fatal |
| | `png_do_read_transformations` | palette row with index checking enabled: `row_info->color_type == PNG_COLOR_TYPE_PALETTE && png_ptr->num_palette_max >= 0` (pngrtran.c:5117-5119) | `png_do_check_palette_indexes` records the max index; out-of-range indices reported later as `"Read palette index exceeding num_palette"` |
| | `png_process_data` | `png_ptr == NULL \|\| info_ptr == NULL` (pngpread.c:53-54) | silent `return`; supplied buffer ignored |
| | `png_process_data_pause` | `png_ptr == NULL` (pngpread.c:67, 88) | `return 0` |
| | `png_process_data_pause` | `save == 0` and `png_ptr->save_buffer_size >= remaining` (all pending data is in the save buffer) (pngpread.c:83-88) | `return 0` (no bytes handed back to the caller) |
| | `png_process_data_skip` | any call — the API is unimplemented (pngpread.c:99-101) | `png_app_warning(png_ptr, "png_process_data_skip is not implemented in any current version of libpng")`, `return 0` |
| | `png_process_some_data` | `png_ptr == NULL` (pngpread.c:110-111) | silent `return` |
| | `png_process_some_data` | `png_ptr->process_mode` is not SIG/CHUNK/IDAT mode (e.g. `PNG_READ_DONE_MODE`, `PNG_ERROR_MODE`, tEXt/zTXt/iTXt modes) — switch `default:` (pngpread.c:133-137) | `png_ptr->buffer_size = 0`; remaining input silently discarded |
| | `png_push_read_sig` | signature mismatch within the first 4 bytes: `png_sig_cmp(...) != 0 && num_checked < 4 && png_sig_cmp(signature, num_checked, num_to_check - 4) != 0` (pngpread.c:162-166) | `png_error(png_ptr, "Not a PNG file")` — fatal |
| | `png_push_read_sig` | signature mismatch only in the later bytes (CR/LF mangling) (pngpread.c:162, 168-169) | `png_error(png_ptr, "PNG file corrupted by ASCII conversion")` — fatal |
| | `png_push_read_chunk` | fewer than 8 buffered bytes for the chunk length+tag: `png_ptr->buffer_size < 8` (`PNG_PUSH_SAVE_BUFFER_IF_LT(8)`) (pngpread.c:196, 30-32) | data saved via `png_push_save_buffer`, `return`; caller must supply more data |
| | `png_push_read_chunk` | `IDAT` reached with `(png_ptr->mode & PNG_HAVE_IHDR) == 0` (pngpread.c:212-213) | `png_error(png_ptr, "Missing IHDR before IDAT")` — fatal |
| | `png_push_read_chunk` | `IDAT` with `png_ptr->color_type == PNG_COLOR_TYPE_PALETTE && (png_ptr->mode & PNG_HAVE_PLTE) == 0` (pngpread.c:215-217) | `png_error(png_ptr, "Missing PLTE before IDAT")` — fatal |
| | `png_push_read_chunk` | zero-length `IDAT` after IDAT already seen and no intervening chunk: `HAVE_IDAT && !HAVE_CHUNK_AFTER_IDAT && push_length == 0` (pngpread.c:221-224) | early `return` (chunk header left pending); chunk not processed |
| | `png_push_read_chunk` | `IDAT` encountered when `(png_ptr->mode & PNG_AFTER_IDAT) != 0` (pngpread.c:228-229) | `png_benign_error(png_ptr, "Too many IDATs found")` |
| | `png_push_read_chunk` | `IHDR` whose `png_ptr->push_length != 13` (pngpread.c:242-243) | `png_error(png_ptr, "Invalid IHDR length")` — fatal |
| | `png_push_read_chunk` | whole chunk + CRC not yet buffered: `png_ptr->push_length + 4 > png_ptr->buffer_size` (`PNG_PUSH_SAVE_BUFFER_IF_FULL` for IHDR/IEND/unknown/other chunks) (pngpread.c:245, 251, 261, 283; macro at 27-29) | data saved via `png_push_save_buffer`, `return`; chunk retried when more data arrives |
| | `png_push_read_IDAT` | fewer than 8 buffered bytes for the next chunk header: `buffer_size < 8` (`PNG_PUSH_SAVE_BUFFER_IF_LT(8)`) (pngpread.c:412) | data saved, `return` |
| | `png_push_read_IDAT` | next chunk is not `IDAT` while the zlib stream has not ended: `chunk_name != png_IDAT && (png_ptr->flags & PNG_FLAG_ZSTREAM_ENDED) == 0` (pngpread.c:420-425) | `png_error(png_ptr, "Not enough compressed data")` — fatal |
| | `png_push_read_IDAT` | fewer than 4 buffered bytes for the IDAT CRC: `buffer_size < 4` (`PNG_PUSH_SAVE_BUFFER_IF_LT(4)`) (pngpread.c:488) | data saved, `return` |
| | `png_push_save_buffer` | save-buffer growth would overflow: `png_ptr->save_buffer_size > PNG_SIZE_MAX - (png_ptr->current_buffer_size + 256)` (pngpread.c:358-361) | `png_error(png_ptr, "Potential overflow of save_buffer")` — fatal |
| | `png_push_save_buffer` | `png_malloc_warn(new_max)` returns `NULL` (pngpread.c:366-373) | old buffer freed, `png_error(png_ptr, "Insufficient memory for save_buffer")` — fatal |
| | `png_push_save_buffer` | inconsistent state: `old_buffer == NULL` while `png_ptr->save_buffer_size != 0` (pngpread.c:375-378) | `png_error(png_ptr, "save_buffer error")` — fatal |
| | `png_push_fill_buffer` | `png_ptr == NULL` (pngpread.c:295-296) | silent `return`; caller's buffer left unfilled |
| | `png_process_IDAT_data` | `!(buffer_length > 0) \|\| buffer == NULL` (pngpread.c:501-502) | `png_error(png_ptr, "No IDAT data (internal error)")` — fatal |
| | `png_process_IDAT_data` | zlib returns neither `Z_OK` nor `Z_STREAM_END` and all rows are already done: `png_ptr->row_number >= png_ptr->num_rows \|\| png_ptr->pass > 6` (pngpread.c:544-555) | zstream marked ended; `png_warning(png_ptr, "Truncated compressed data in IDAT")`, `return` |
| | `png_process_IDAT_data` | zlib returns `Z_DATA_ERROR` while rows are still expected (pngpread.c:559-560) | `png_benign_error(png_ptr, "IDAT: ADLER32 checksum mismatch")`, `return` |
| | `png_process_IDAT_data` | zlib returns any other failure while rows are still expected (pngpread.c:561-562) | `png_error(png_ptr, "Decompression error in IDAT")` — fatal |
| | `png_process_IDAT_data` | inflate produced output after the last row: `next_out != row_buf` and `row_number >= num_rows \|\| pass > 6` (pngpread.c:570-580) | `png_warning(png_ptr, "Extra compressed data in IDAT")`; zstream force-ended, `return` |
| | `png_process_IDAT_data` | bytes left after the zlib end code: `png_ptr->zstream.avail_in > 0` on exit (pngpread.c:604-605) | `png_warning(png_ptr, "Extra compression data in IDAT")` |
| | `png_push_process_row` | filter byte `png_ptr->row_buf[0] >= PNG_FILTER_VALUE_LAST` (pngpread.c:621-627) | `png_error(png_ptr, "bad adaptive filter value")` — fatal |
| | `png_push_process_row` | first row's `row_info.pixel_depth > png_ptr->maximum_pixel_depth` after transforms (pngpread.c:643-647) | `png_error(png_ptr, "progressive row overflow")` — fatal |
| | `png_push_process_row` | later row's `png_ptr->transformed_pixel_depth != row_info.pixel_depth` (pngpread.c:650-651) | `png_error(png_ptr, "internal progressive row size calculation error")` — fatal |
| | `png_read_push_finish_row` | interlace pass counter runs past the last pass: `png_ptr->pass > 7` (pngpread.c:859-860) | clamped with `png_ptr->pass--`, then loop `break` at `pass >= 7` |
| | `png_progressive_combine_row` | `png_ptr == NULL` (pngpread.c:910-911) | silent `return`; no combining done |
| | `png_progressive_combine_row` | `new_row == NULL` (callback was invoked for an empty interlace row) (pngpread.c:917-918) | `png_combine_row` not called; `old_row` left unchanged |
| | `png_set_progressive_read_fn` | `png_ptr == NULL` (pngpread.c:927-928) | silent `return`; callbacks not installed |
| | `png_get_progressive_ptr` | `png_ptr == NULL` (pngpread.c:940-941) | `return NULL` |
