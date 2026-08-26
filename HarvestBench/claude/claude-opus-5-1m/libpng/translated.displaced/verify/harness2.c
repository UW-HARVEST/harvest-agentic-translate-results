/* Second behavioural comparison harness: covers the parts of the API that
 * harness.c does not reach (file I/O, user memory handlers, the floating point
 * setters/getters, iCCP/pCAL/hIST chunks, MNG intrapixel differencing, the
 * write side transforms, the colour-mapped simplified API, the default error
 * handlers, ...).
 *
 * As with harness.c everything printed is deterministic so the C and the Rust
 * build can be diffed byte for byte.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include "png.h"
#include <zlib.h>

/* Internal (but exported) helpers used below. */
extern png_fixed_point png_fixed(png_const_structrp png_ptr, double fp, png_const_charp text);

/* sizeof(png_info) in this build of the library (png_info is opaque in png.h). */
#define HARNESS_PNG_INFO_SIZE 352
#define PNG_NUMBER_FORMAT_u 1
#define PNG_NUMBER_FORMAT_d 1
#define PNG_NUMBER_FORMAT_02u 2
#define PNG_NUMBER_FORMAT_x 3
#define PNG_NUMBER_FORMAT_02x 4
#define PNG_CHUNK_WARNING 0
#define PNG_CHUNK_WRITE_ERROR 1
#define PNG_CHUNK_ERROR 2

static unsigned long fnv(const unsigned char *p, size_t n)
{
   unsigned long h = 1469598103934665603UL;
   size_t i;
   for (i = 0; i < n; ++i) { h ^= p[i]; h *= 1099511628211UL; }
   return h;
}

static void fill_row(unsigned char *row, size_t rowbytes, unsigned seed)
{
   size_t i;
   unsigned x = seed * 2654435761u + 1;
   for (i = 0; i < rowbytes; ++i)
   {
      x = x * 1103515245u + 12345u;
      row[i] = (unsigned char)((x >> 16) & 0xff);
   }
}

static void err_fn(png_structp png_ptr, png_const_charp msg)
{
   printf("  ERROR: %s\n", msg);
   longjmp(*(jmp_buf*)png_get_error_ptr(png_ptr), 1);
}
static void warn_fn(png_structp png_ptr, png_const_charp msg)
{
   (void)png_ptr;
   printf("  WARNING: %s\n", msg);
}

/* ------------------------------------------------------- user memory hooks */
static unsigned long alloc_count = 0, free_count = 0;

static png_voidp user_malloc(png_structp png_ptr, png_alloc_size_t size)
{
   (void)png_ptr;
   ++alloc_count;
   return malloc(size);
}
static void user_free(png_structp png_ptr, png_voidp ptr)
{
   (void)png_ptr;
   ++free_count;
   free(ptr);
}

/* -------------------------------------------------------------- ICC profile */
static png_byte *make_icc_profile(int color, png_uint_32 *lenp)
{
   /* 132 byte header + one tag (12 byte table entry) + 20 bytes of tag data. */
   static const png_byte D50[12] =
      { 0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d };
   png_uint_32 len = 132 + 12 + 1024;
   png_byte *p = calloc(1, len);

   png_save_uint_32(p, len);
   memcpy(p + 4, "none", 4);       /* preferred CMM */
   p[8] = 2;                       /* major version */
   memcpy(p + 12, "mntr", 4);      /* profile class: monitor */
   memcpy(p + 16, color ? "RGB " : "GRAY", 4);
   memcpy(p + 20, "XYZ ", 4);      /* PCS */
   memcpy(p + 36, "acsp", 4);      /* file signature */
   png_save_uint_32(p + 64, 0);    /* rendering intent: perceptual */
   memcpy(p + 68, D50, 12);        /* PCS illuminant */
   png_save_uint_32(p + 128, 1);   /* tag count */
   memcpy(p + 132, "desc", 4);     /* tag signature */
   png_save_uint_32(p + 136, 144); /* tag offset (aligned) */
   png_save_uint_32(p + 140, 1024); /* tag length */
   memcpy(p + 144, "desc\0\0\0\0", 8);
   /* Incompressible-ish payload so the compressed iCCP chunk is comfortably
    * longer than the 81+LZ77Min bytes libpng requires.
    */
   fill_row(p + 152, 1016, 42);

   *lenp = len;
   return p;
}


typedef struct { const unsigned char *data; size_t size, pos; } rdbuf2;

static void mem_read2(png_structp png_ptr, png_bytep data, size_t length)
{
   rdbuf2 *rb = (rdbuf2*)png_get_io_ptr(png_ptr);
   size_t avail = rb->size - rb->pos;
   if (length > avail)
   {
      png_error(png_ptr, "read past end");
      return;
   }
   memcpy(data, rb->data + rb->pos, length);
   rb->pos += length;
}

/* ------------------------------------------------------------ file helpers */
static const char *tmpname(const char *base)
{
   static char buf[512];
   const char *dir = getenv("TMPDIR");
   if (dir == NULL) dir = "/tmp";
   snprintf(buf, sizeof buf, "%s/%s", dir, base);
   return buf;
}

static size_t slurp(const char *path, unsigned char **datap)
{
   FILE *f = fopen(path, "rb");
   size_t n = 0, cap = 65536;
   unsigned char *d;
   if (f == NULL) { *datap = NULL; return 0; }
   d = malloc(cap);
   for (;;)
   {
      size_t got = fread(d + n, 1, cap - n, f);
      n += got;
      if (n < cap) break;
      cap *= 2;
      d = realloc(d, cap);
   }
   fclose(f);
   *datap = d;
   return n;
}

/* ==================================================================== 1 ==== */
/* File based write using png_init_io, all the floating point setters, iCCP,
 * pCAL, hIST, sPLT, user memory handlers, MNG intrapixel differencing.
 */
static int write_file(const char *path, int color_type, int bit_depth, int mng,
                      int interlace, unsigned width, unsigned height)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   FILE *fp;
   unsigned y;
   size_t rowbytes;
   unsigned char **rows = NULL;

   fp = fopen(path, "wb");
   if (fp == NULL) { printf("  cannot open %s\n", path); return 0; }

   alloc_count = free_count = 0;
   png_ptr = png_create_write_struct_2(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn,
                                       (png_voidp)&alloc_count, user_malloc, user_free);
   if (png_ptr == NULL) { printf("  create_write_struct_2 failed\n"); fclose(fp); return 0; }
   printf("  mem_ptr set: %d\n", png_get_mem_ptr(png_ptr) == (png_voidp)&alloc_count);
   info_ptr = png_create_info_struct(png_ptr);

   if (setjmp(jb))
   {
      png_destroy_write_struct(&png_ptr, &info_ptr);
      fclose(fp);
      return 0;
   }

   png_init_io(png_ptr, fp);
   printf("  io_ptr set: %d\n", png_get_io_ptr(png_ptr) == (png_voidp)fp);

   if (mng)
      png_permit_mng_features(png_ptr, PNG_ALL_MNG_FEATURES);
   png_set_IHDR(png_ptr, info_ptr, width, height, bit_depth, color_type, interlace,
                PNG_COMPRESSION_TYPE_DEFAULT,
                mng ? PNG_INTRAPIXEL_DIFFERENCING : PNG_FILTER_TYPE_DEFAULT);

   /* Floating point variants of the setters. */
   png_set_gAMA(png_ptr, info_ptr, 0.45455);
   png_set_cHRM(png_ptr, info_ptr, 0.3127, 0.329, 0.64, 0.33, 0.30, 0.60, 0.15, 0.06);
   png_set_cHRM_XYZ(png_ptr, info_ptr, 0.4124, 0.2126, 0.0193,
                    0.3576, 0.7152, 0.1192, 0.1805, 0.0722, 0.9505);
   png_set_cLLI(png_ptr, info_ptr, 1000.0, 400.0);
   png_set_mDCV(png_ptr, info_ptr, 0.3127, 0.329, 0.64, 0.33, 0.30, 0.60, 0.15, 0.06,
                1000.0, 0.005);
   png_set_sCAL(png_ptr, info_ptr, PNG_SCALE_METER, 1.2345, 6.54321);

   {
      png_byte exif[10] = { 77, 77, 0, 42, 0, 0, 0, 8, 0, 1 };
      png_set_eXIf(png_ptr, info_ptr, exif);
   }

   {
      png_charp params[2];
      params[0] = (png_charp)"1.5";
      params[1] = (png_charp)"-2.25";
      png_set_pCAL(png_ptr, info_ptr, (png_charp)"calibration", -100, 1000,
                   PNG_EQUATION_LINEAR, 2, (png_charp)"units", params);
   }

   {
      png_uint_32 plen;
      png_byte *profile = make_icc_profile((color_type & PNG_COLOR_MASK_COLOR) != 0, &plen);
      png_set_iCCP(png_ptr, info_ptr, "ICC profile", PNG_COMPRESSION_TYPE_BASE,
                   profile, plen);
      free(profile);
   }

   if ((color_type & PNG_COLOR_MASK_PALETTE) != 0)
   {
      png_color pal[256];
      png_uint_16 hist[256];
      png_byte trans[256];
      int i, n = 1 << bit_depth;
      if (n > 256) n = 256;
      for (i = 0; i < n; ++i)
      {
         pal[i].red = (png_byte)(i * 5 + 2);
         pal[i].green = (png_byte)(i * 11 + 7);
         pal[i].blue = (png_byte)(255 - i * 3);
         hist[i] = (png_uint_16)(1000 - i * 3);
         trans[i] = (png_byte)(i * 9 + 1);
      }
      png_set_PLTE(png_ptr, info_ptr, pal, n);
      png_set_hIST(png_ptr, info_ptr, hist);
      png_set_tRNS(png_ptr, info_ptr, trans, n, NULL);
   }

   {
      png_text text[2];
      memset(text, 0, sizeof text);
      text[0].compression = PNG_ITXT_COMPRESSION_zTXt;
      text[0].key = (png_charp)"Author";
      text[0].text = (png_charp)"compressed international text, compressed international text";
      text[0].lang = (png_charp)"en-GB";
      text[0].lang_key = (png_charp)"Author";
      text[1].compression = PNG_TEXT_COMPRESSION_NONE;
      text[1].key = (png_charp)"Software";
      text[1].text = (png_charp)"harness2";
      png_set_text(png_ptr, info_ptr, text, 2);
   }

   /* png_set_invalid / png_set_sig_bytes / png_data_freer exercise */
   png_set_invalid(png_ptr, info_ptr, PNG_INFO_sCAL);
   png_set_sCAL_fixed(png_ptr, info_ptr, PNG_SCALE_RADIAN, 250000, 125000);
   png_data_freer(png_ptr, info_ptr, PNG_DESTROY_WILL_FREE_DATA, PNG_FREE_ALL);

   png_set_filter(png_ptr, PNG_FILTER_TYPE_BASE, PNG_FILTER_SUB | PNG_FILTER_AVG);
   {
      png_fixed_point weights[3] = { 150000, 125000, 100000 };
      png_fixed_point costs[PNG_FILTER_VALUE_LAST] =
         { 100000, 100000, 100000, 100000, 100000 };
      png_set_filter_heuristics_fixed(png_ptr, PNG_FILTER_HEURISTIC_WEIGHTED, 3,
                                      weights, costs);
   }

   png_write_info(png_ptr, info_ptr);

   /* Write side transforms. */
   png_set_packing(png_ptr);
   if ((color_type & PNG_COLOR_MASK_PALETTE) == 0)
   {
      png_color_8 sh;
      memset(&sh, 0, sizeof sh);
      sh.red = sh.green = sh.blue = sh.gray = (png_byte)(bit_depth > 8 ? 12 : 6);
      sh.alpha = sh.gray;
      png_set_shift(png_ptr, &sh);
   }
   if ((color_type & PNG_COLOR_MASK_ALPHA) != 0)
   {
      png_set_invert_alpha(png_ptr);
      png_set_swap_alpha(png_ptr);
   }
   if ((color_type & PNG_COLOR_MASK_COLOR) != 0 &&
       (color_type & PNG_COLOR_MASK_PALETTE) == 0)
      png_set_bgr(png_ptr);
   if (bit_depth == 16)
      png_set_swap(png_ptr);

   rowbytes = png_get_rowbytes(png_ptr, info_ptr);
   printf("  file write rowbytes=%lu\n", (unsigned long)rowbytes);

   rows = malloc(height * sizeof(unsigned char*));
   for (y = 0; y < height; ++y)
   {
      /* png_set_packing means the input has one sample per byte. */
      size_t inbytes = rowbytes;
      if (bit_depth < 8)
         inbytes = (size_t)width * png_get_channels(png_ptr, info_ptr);
      rows[y] = malloc(inbytes + 8);
      memset(rows[y], 0, inbytes + 8);
      fill_row(rows[y], inbytes, y + 3);
      if (bit_depth < 8)
      {
         size_t i;
         png_byte mask = (png_byte)((1 << bit_depth) - 1);
         for (i = 0; i < inbytes; ++i) rows[y][i] &= mask;
      }
   }

   if (interlace != PNG_INTERLACE_NONE)
   {
      int pass, passes = png_set_interlace_handling(png_ptr);
      for (pass = 0; pass < passes; ++pass)
         png_write_rows(png_ptr, rows, height);
   }
   else
      png_write_rows(png_ptr, rows, height);

   png_write_end(png_ptr, info_ptr);
   png_destroy_write_struct(&png_ptr, &info_ptr);
   fclose(fp);
   for (y = 0; y < height; ++y) free(rows[y]);
   free(rows);
   printf("  allocations: %d frees: %d\n", alloc_count > 0, free_count > 0);
   return 1;
}

/* ==================================================================== 2 ==== */
/* File based read with png_init_io, all the float getters, unknown chunk
 * handling variants and MNG features.
 */
static void read_file(const char *path, int mng, const char *tag)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   FILE *fp;
   unsigned y, height;
   size_t rowbytes;
   unsigned char **rows = NULL;
   unsigned long h = 0;
   png_byte sig[8];

   fp = fopen(path, "rb");
   if (fp == NULL) { printf("  %s: cannot open\n", tag); return; }
   if (fread(sig, 1, 8, fp) != 8) { printf("  %s: short read\n", tag); fclose(fp); return; }
   printf("  %s sig_cmp=%d\n", tag, png_sig_cmp(sig, 0, 8));

   png_ptr = png_create_read_struct_2(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn,
                                      NULL, user_malloc, user_free);
   info_ptr = png_create_info_struct(png_ptr);

   if (setjmp(jb))
   {
      printf("  %s aborted\n", tag);
      png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
      fclose(fp);
      return;
   }

   png_init_io(png_ptr, fp);
   png_set_sig_bytes(png_ptr, 8);
   if (mng) png_permit_mng_features(png_ptr, PNG_ALL_MNG_FEATURES);
   png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_IF_SAFE,
                               (png_const_bytep)"prVt", 1);
   png_set_read_user_chunk_fn(png_ptr, NULL, NULL);
   png_read_info(png_ptr, info_ptr);

   printf("  %s io_chunk_type=%u buffer_size=%lu\n", tag,
          (unsigned)png_get_io_chunk_type(png_ptr),
          (unsigned long)png_get_compression_buffer_size(png_ptr));
   printf("  %s limits w=%u h=%u cache=%u malloc=%lu\n", tag,
          (unsigned)png_get_user_width_max(png_ptr),
          (unsigned)png_get_user_height_max(png_ptr),
          (unsigned)png_get_chunk_cache_max(png_ptr),
          (unsigned long)png_get_chunk_malloc_max(png_ptr));

   {
      double wx, wy, rx, ry, gx, gy, bx, by;
      if (png_get_cHRM(png_ptr, info_ptr, &wx, &wy, &rx, &ry, &gx, &gy, &bx, &by) != 0)
         printf("  %s cHRM %.6f %.6f %.6f %.6f %.6f %.6f %.6f %.6f\n", tag,
                wx, wy, rx, ry, gx, gy, bx, by);
   }
   {
      double rX, rY, rZ, gX, gY, gZ, bX, bY, bZ;
      if (png_get_cHRM_XYZ(png_ptr, info_ptr, &rX, &rY, &rZ, &gX, &gY, &gZ,
                           &bX, &bY, &bZ) != 0)
         printf("  %s cHRM_XYZ %.6f %.6f %.6f %.6f %.6f %.6f %.6f %.6f %.6f\n", tag,
                rX, rY, rZ, gX, gY, gZ, bX, bY, bZ);
   }
   {
      int unit = 0;
      double sw = 0, sh = 0;
      if (png_get_sCAL(png_ptr, info_ptr, &unit, &sw, &sh) != 0)
         printf("  %s sCAL %d %.6f %.6f\n", tag, unit, sw, sh);
   }
   {
      png_uint_32 res_x = 0, res_y = 0;
      int unit = 0;
      if (png_get_pHYs_dpi(png_ptr, info_ptr, &res_x, &res_y, &unit) != 0)
         printf("  %s pHYs_dpi %u %u %d\n", tag, (unsigned)res_x, (unsigned)res_y, unit);
      printf("  %s ppm=%u ppi=%u aspect_fixed=%ld\n", tag,
             (unsigned)png_get_pixels_per_meter(png_ptr, info_ptr),
             (unsigned)png_get_pixels_per_inch(png_ptr, info_ptr),
             (long)png_get_pixel_aspect_ratio_fixed(png_ptr, info_ptr));
      printf("  %s offs px=%ld,%ld inches_fixed=%ld,%ld\n", tag,
             (long)png_get_x_offset_pixels(png_ptr, info_ptr),
             (long)png_get_y_offset_pixels(png_ptr, info_ptr),
             (long)png_get_x_offset_inches_fixed(png_ptr, info_ptr),
             (long)png_get_y_offset_inches_fixed(png_ptr, info_ptr));
   }
   {
      png_charp name = NULL;
      int comp = 0;
      png_bytep prof = NULL;
      png_uint_32 plen = 0;
      if (png_get_iCCP(png_ptr, info_ptr, &name, &comp, &prof, &plen) != 0)
         printf("  %s iCCP '%s' comp=%d len=%u hash=%016lx\n", tag, name, comp,
                (unsigned)plen, fnv(prof, plen));
   }
   {
      png_charp purpose = NULL, units = NULL;
      png_charpp params = NULL;
      png_int_32 X0 = 0, X1 = 0;
      int type = 0, nparams = 0;
      if (png_get_pCAL(png_ptr, info_ptr, &purpose, &X0, &X1, &type, &nparams,
                       &units, &params) != 0)
      {
         int i;
         printf("  %s pCAL '%s' %ld %ld type=%d n=%d units='%s'\n", tag, purpose,
                (long)X0, (long)X1, type, nparams, units);
         for (i = 0; i < nparams; ++i) printf("    %s param[%d]='%s'\n", tag, i, params[i]);
      }
   }
   {
      png_uint_16p hist = NULL;
      if (png_get_hIST(png_ptr, info_ptr, &hist) != 0)
         printf("  %s hIST hash=%016lx\n", tag, fnv((unsigned char*)hist, 512));
   }
   {
      png_bytep exif = NULL;
      png_uint_32 n = 0;
      if (png_get_eXIf_1(png_ptr, info_ptr, &n, &exif) != 0)
         printf("  %s eXIf n=%u hash=%016lx\n", tag, (unsigned)n, fnv(exif, n));
   }
   {
      png_textp text = NULL;
      int nt = 0;
      int i;
      png_get_text(png_ptr, info_ptr, &text, &nt);
      for (i = 0; i < nt; ++i)
         printf("  %s text[%d] comp=%d key='%s' text='%s' lang='%s'\n", tag, i,
                text[i].compression, text[i].key, text[i].text ? text[i].text : "",
                text[i].lang ? text[i].lang : "");
   }
   {
      png_unknown_chunkp unk = NULL;
      int n = png_get_unknown_chunks(png_ptr, info_ptr, &unk);
      int i;
      for (i = 0; i < n; ++i)
         printf("  %s unknown[%d] '%s' size=%lu loc=%d\n", tag, i, (char*)unk[i].name,
                (unsigned long)unk[i].size, unk[i].location);
   }

   /* read side transforms not covered by harness.c */
   png_set_palette_to_rgb(png_ptr);
   png_set_tRNS_to_alpha(png_ptr);
   png_set_add_alpha(png_ptr, 0xffff, PNG_FILLER_AFTER);
   png_read_update_info(png_ptr, info_ptr);

   height = png_get_image_height(png_ptr, info_ptr);
   rowbytes = png_get_rowbytes(png_ptr, info_ptr);
   rows = malloc(height * sizeof(unsigned char*));
   for (y = 0; y < height; ++y)
   {
      rows[y] = malloc(rowbytes + 8);
      memset(rows[y], 0, rowbytes + 8);
   }

   /* png_read_rows with both row and display_row set. */
   {
      unsigned char **disp = malloc(height * sizeof(unsigned char*));
      for (y = 0; y < height; ++y)
      {
         disp[y] = malloc(rowbytes + 8);
         memset(disp[y], 0, rowbytes + 8);
      }
      for (y = 0; y < height; ++y)
         png_read_rows(png_ptr, rows + y, disp + y, 1);
      for (y = 0; y < height; ++y) h ^= fnv(disp[y], rowbytes) * (y + 3);
      printf("  %s display hash=%016lx\n", tag, h);
      for (y = 0; y < height; ++y) free(disp[y]);
      free(disp);
   }

   h = 0;
   for (y = 0; y < height; ++y) h ^= fnv(rows[y], rowbytes) * (y + 1);
   printf("  %s rows hash=%016lx rowbytes=%lu\n", tag, h, (unsigned long)rowbytes);

   png_read_end(png_ptr, info_ptr);
   printf("  %s reset_zstream=%d\n", tag, png_reset_zstream(png_ptr));
   png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
   fclose(fp);
   for (y = 0; y < height; ++y) free(rows[y]);
   free(rows);
}

/* ==================================================================== 3 ==== */
/* Default error/warning handlers (no error_fn) plus png_set_longjmp_fn. */
static void default_handlers(void)
{
   png_structp png_ptr;
   jmp_buf *jbp;

   printf("== default handlers ==\n");
   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
   if (png_ptr == NULL) { printf("  create failed\n"); return; }

   jbp = png_set_longjmp_fn(png_ptr, longjmp, (size_t)sizeof(jmp_buf));
   if (jbp == NULL) { printf("  no jmp_buf\n"); return; }

   if (setjmp(*jbp) == 0)
   {
      png_infop info_ptr = png_create_info_struct(png_ptr);
      /* A warning through the default handler (stderr). */
      png_warning(png_ptr, "default warning path");
      /* png_fixed() with an out of range value calls png_fixed_error. */
      (void)png_fixed(png_ptr, 1e10, "harness2 test");
      printf("  unreachable\n");
      (void)info_ptr;
   }
   else
      printf("  longjmp taken\n");

   png_destroy_read_struct(&png_ptr, NULL, NULL);
}

/* ==================================================================== 4 ==== */
/* Progressive reader: pause/skip and png_progressive_combine_row. */
typedef struct { unsigned long h; unsigned rows; unsigned char *row; size_t rowbytes; } prog2;

static void prog2_info(png_structp png_ptr, png_infop info_ptr)
{
   prog2 *st = (prog2*)png_get_progressive_ptr(png_ptr);
   png_set_interlace_handling(png_ptr);
   png_read_update_info(png_ptr, info_ptr);
   st->rowbytes = png_get_rowbytes(png_ptr, info_ptr);
   st->row = malloc(st->rowbytes + 8);
   memset(st->row, 0, st->rowbytes + 8);
   printf("  prog2 info rowbytes=%lu\n", (unsigned long)st->rowbytes);
}

static void prog2_row(png_structp png_ptr, png_bytep new_row, png_uint_32 row_num, int pass)
{
   prog2 *st = (prog2*)png_get_progressive_ptr(png_ptr);
   if (new_row != NULL)
   {
      png_progressive_combine_row(png_ptr, st->row, new_row);
      st->h ^= fnv(st->row, st->rowbytes) * (row_num + 1) * (unsigned)(pass + 1);
      st->rows++;
   }
}

static void prog2_end(png_structp png_ptr, png_infop info_ptr)
{
   (void)png_ptr; (void)info_ptr;
   printf("  prog2 end\n");
}

static void progressive2(const unsigned char *data, size_t size)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   prog2 st;
   size_t pos = 0;
   int paused = 0;

   printf("== progressive pause/skip ==\n");
   st.h = 0; st.rows = 0; st.row = NULL; st.rowbytes = 0;

   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   info_ptr = png_create_info_struct(png_ptr);
   if (setjmp(jb))
   {
      printf("  aborted\n");
      png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
      free(st.row);
      return;
   }
   png_set_progressive_read_fn(png_ptr, &st, prog2_info, prog2_row, prog2_end);
   while (pos < size)
   {
      size_t n = size - pos < 29 ? size - pos : 29;
      png_process_data(png_ptr, info_ptr, (png_bytep)(data + pos), n);
      pos += n;
      if (paused == 0)
      {
         size_t remaining = png_process_data_pause(png_ptr, 0);
         printf("  paused remaining=%lu\n", (unsigned long)remaining);
         paused = 1;
      }
   }
   printf("  prog2 rows=%u hash=%016lx\n", st.rows, st.h);
   png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
   free(st.row);
}

/* ==================================================================== 5 ==== */
/* Simplified API: colour-mapped output, file and stdio variants, background. */
static void simplified2(unsigned width, unsigned height, unsigned in_format,
                        unsigned out_format, int use_background, int to_file)
{
   png_image image;
   unsigned char *buf;
   size_t bufsize;
   const char *path = tmpname("harness2_simple.png");

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = width;
   image.height = height;
   image.format = in_format;

   bufsize = PNG_IMAGE_SIZE(image);
   buf = malloc(bufsize);
   fill_row(buf, bufsize, 11);

   if (to_file != 0)
   {
      if (png_image_write_to_file(&image, path, 0, buf,
                                  (png_int_32)PNG_IMAGE_ROW_STRIDE(image), NULL) == 0)
      {
         printf("  write_to_file failed: %s\n", image.message);
         free(buf);
         return;
      }
   }
   else
   {
      FILE *f = fopen(path, "wb");
      if (f == NULL) { free(buf); return; }
      if (png_image_write_to_stdio(&image, f, 1 /*convert_to_8_bit*/, buf,
                                   (png_int_32)PNG_IMAGE_ROW_STRIDE(image), NULL) == 0)
         printf("  write_to_stdio failed: %s\n", image.message);
      fclose(f);
   }

   {
      unsigned char *data;
      size_t n = slurp(path, &data);
      printf("  simplified2 in=%u out=%u bg=%d file=%d wrote %lu hash=%016lx\n",
             in_format, out_format, use_background, to_file, (unsigned long)n,
             fnv(data, n));
      free(data);
   }

   {
      png_image ri;
      memset(&ri, 0, sizeof ri);
      ri.version = PNG_IMAGE_VERSION;
      if (png_image_begin_read_from_file(&ri, path) != 0)
      {
         unsigned char *out;
         unsigned char *cmap = NULL;
         png_color bg;

         ri.format = out_format;
         out = malloc(PNG_IMAGE_SIZE(ri) + 16);
         memset(out, 0, PNG_IMAGE_SIZE(ri) + 16);
         if ((out_format & PNG_FORMAT_FLAG_COLORMAP) != 0)
         {
            cmap = malloc(PNG_IMAGE_COLORMAP_SIZE(ri) + 16);
            memset(cmap, 0, PNG_IMAGE_COLORMAP_SIZE(ri) + 16);
         }
         memset(&bg, 0, sizeof bg);
         bg.red = 0x40; bg.green = 0x80; bg.blue = 0xc0;

         if (png_image_finish_read(&ri, use_background ? &bg : NULL, out, 0, cmap) != 0)
            printf("  simplified2 read %ux%u fmt=%u cmap_entries=%u out_hash=%016lx cmap_hash=%016lx\n",
                   (unsigned)ri.width, (unsigned)ri.height, (unsigned)ri.format,
                   (unsigned)ri.colormap_entries, fnv(out, PNG_IMAGE_SIZE(ri)),
                   cmap != NULL ? fnv(cmap, PNG_IMAGE_COLORMAP_SIZE(ri)) : 0);
         else
            printf("  simplified2 read failed: %s\n", ri.message);
         free(out);
         free(cmap);
      }
      else printf("  begin_read_from_file failed: %s\n", ri.message);
      png_image_free(&ri);
   }

   /* stdio read variant */
   {
      png_image ri;
      FILE *f = fopen(path, "rb");
      memset(&ri, 0, sizeof ri);
      ri.version = PNG_IMAGE_VERSION;
      if (f != NULL)
      {
         if (png_image_begin_read_from_stdio(&ri, f) != 0)
         {
            unsigned char *out;
            ri.format = out_format & ~PNG_FORMAT_FLAG_COLORMAP;
            out = malloc(PNG_IMAGE_SIZE(ri) + 16);
            memset(out, 0, PNG_IMAGE_SIZE(ri) + 16);
            if (png_image_finish_read(&ri, NULL, out, 0, NULL) != 0)
               printf("  simplified2 stdio read hash=%016lx\n",
                      fnv(out, PNG_IMAGE_SIZE(ri)));
            else
               printf("  simplified2 stdio read failed: %s\n", ri.message);
            free(out);
         }
         else printf("  begin_read_from_stdio failed: %s\n", ri.message);
         png_image_free(&ri);
         fclose(f);
      }
   }

   free(buf);
   remove(path);
}

/* ==================================================================== 6 ==== */
static void info_utilities(void)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;

   printf("== info utilities ==\n");
   png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   if (png_ptr == NULL || setjmp(jb) != 0) { printf("  failed\n"); return; }

   info_ptr = png_create_info_struct(png_ptr);
   png_info_init_3(&info_ptr, HARNESS_PNG_INFO_SIZE);
   png_set_IHDR(png_ptr, info_ptr, 5, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
   printf("  after init_3 rowbytes=%lu valid=%08lx\n",
          (unsigned long)png_get_rowbytes(png_ptr, info_ptr),
          (unsigned long)png_get_valid(png_ptr, info_ptr, 0xffffffff));

   png_set_gAMA_fixed(png_ptr, info_ptr, 100000);
   png_set_sRGB_gAMA_and_cHRM(png_ptr, info_ptr, PNG_sRGB_INTENT_RELATIVE);
   printf("  sRGB_gAMA_and_cHRM valid=%08lx\n",
          (unsigned long)png_get_valid(png_ptr, info_ptr, 0xffffffff));
   png_set_invalid(png_ptr, info_ptr, PNG_INFO_gAMA | PNG_INFO_cHRM);
   printf("  after invalid valid=%08lx\n",
          (unsigned long)png_get_valid(png_ptr, info_ptr, 0xffffffff));

   {
      png_unknown_chunk unk;
      memset(&unk, 0, sizeof unk);
      memcpy(unk.name, "uNKn", 5);
      unk.data = (png_byte*)"1234567890";
      unk.size = 10;
      unk.location = PNG_HAVE_IHDR;
      png_set_unknown_chunks(png_ptr, info_ptr, &unk, 1);
      png_set_unknown_chunk_location(png_ptr, info_ptr, 0, PNG_AFTER_IDAT);
      printf("  unknown count=%d\n", png_get_unknown_chunks(png_ptr, info_ptr, NULL));
   }

   printf("  convert_to_rfc1123 (deprecated) ");
   {
      png_time t;
      memset(&t, 0, sizeof t);
      t.year = 2000; t.month = 1; t.day = 2; t.hour = 3; t.minute = 4; t.second = 5;
      printf("'%s'\n", png_convert_to_rfc1123(png_ptr, &t));
   }

   png_set_check_for_invalid_index(png_ptr, 1);
   png_set_check_for_invalid_index(png_ptr, 0);
   png_destroy_write_struct(&png_ptr, &info_ptr);
}


/* ==================================================================== 7 ==== */
/* Functions that are hidden behind macros, the "default" allocators, the
 * deprecated aliases and the single-filter write paths.
 */
#undef png_get_uint_32
#undef png_get_uint_16
#undef png_get_int_32

extern png_voidp png_malloc_default(png_const_structrp png_ptr, png_alloc_size_t size);
extern void png_free_default(png_const_structrp png_ptr, png_voidp ptr);
extern png_voidp png_malloc_base(png_const_structrp png_ptr, png_alloc_size_t size);
extern png_voidp png_malloc_array(png_const_structrp png_ptr, int nelements,
                                  size_t element_size);
extern png_voidp png_realloc_array(png_const_structrp png_ptr, png_const_voidp array,
                                   int old_elements, int add_elements,
                                   size_t element_size);
extern void png_reset_crc(png_structrp png_ptr);
extern void png_calculate_crc(png_structrp png_ptr, png_const_bytep ptr, size_t length);
extern size_t png_safecat(png_charp buffer, size_t bufsize, size_t pos,
                          png_const_charp string);
extern void png_warning_parameter(char p[8][32], int number, png_const_charp string);
extern void png_warning_parameter_unsigned(char p[8][32], int number, int format,
                                           png_alloc_size_t value);
extern void png_warning_parameter_signed(char p[8][32], int number, int format,
                                         png_int_32 value);
extern void png_formatted_warning(png_const_structrp png_ptr, char p[8][32],
                                  png_const_charp message);
extern void png_chunk_report(png_const_structrp png_ptr, png_const_charp message,
                             int error);
extern void png_app_warning(png_const_structrp png_ptr, png_const_charp message);
extern void png_app_error(png_const_structrp png_ptr, png_const_charp message);
extern void png_benign_error(png_const_structrp png_ptr, png_const_charp message);
extern int png_image_error(png_imagep image, png_const_charp error_message);
extern int png_muldiv(png_fixed_point *res, png_fixed_point a, png_int_32 m, png_int_32 d);

static void internals(void)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   png_byte b[4] = { 0x80, 0x01, 0x02, 0x03 };
   png_byte c[4] = { 0x00, 0x7f, 0xff, 0xfe };

   printf("== internals ==\n");
   printf("  get_uint_32=%u get_uint_16=%u get_int_32=%ld\n",
          (unsigned)png_get_uint_32(b), png_get_uint_16(b), (long)png_get_int_32(b));
   printf("  get_uint_32b=%u get_uint_16b=%u get_int_32b=%ld\n",
          (unsigned)png_get_uint_32(c), png_get_uint_16(c), (long)png_get_int_32(c));

   png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   if (png_ptr == NULL) return;
   info_ptr = png_create_info_struct(png_ptr);

   if (setjmp(jb) == 0)
   {
      png_voidp p;

      printf("  get_uint_31=%u\n", (unsigned)png_get_uint_31(png_ptr, c));

      p = png_malloc_default(png_ptr, 100);
      printf("  malloc_default=%d\n", p != NULL);
      png_free_default(png_ptr, p);
      p = png_malloc_base(png_ptr, 200);
      printf("  malloc_base=%d\n", p != NULL);
      png_free(png_ptr, p);
      p = png_calloc(png_ptr, 64);
      printf("  calloc=%d first_byte=%d\n", p != NULL, p != NULL ? *(png_bytep)p : -1);
      png_free(png_ptr, p);
      p = png_malloc_warn(png_ptr, 32);
      printf("  malloc_warn=%d\n", p != NULL);
      png_free(png_ptr, p);
      p = png_malloc_array(png_ptr, 4, 16);
      printf("  malloc_array=%d\n", p != NULL);
      {
         png_voidp q = png_realloc_array(png_ptr, p, 4, 4, 16);
         printf("  realloc_array=%d\n", q != NULL);
         png_free(png_ptr, q);
      }
      png_free(png_ptr, p);

      png_reset_crc(png_ptr);
      png_calculate_crc(png_ptr, (png_const_bytep)"123456789", 9);
      png_reset_crc(png_ptr);

      {
         char buf[32];
         memset(buf, 0, sizeof buf);
         printf("  safecat=%lu '%s'\n",
                (unsigned long)png_safecat(buf, sizeof buf, 0, "abcdefghij"), buf);
         printf("  safecat2=%lu '%s'\n",
                (unsigned long)png_safecat(buf, sizeof buf, 10, "klmnopqrstuvwxyz0123456789"),
                buf);
      }

      {
         char params[8][32];
         memset(params, 0, sizeof params);
         png_warning_parameter(params, 1, "first");
         png_warning_parameter_unsigned(params, 2, PNG_NUMBER_FORMAT_u, 12345);
         png_warning_parameter_signed(params, 3, PNG_NUMBER_FORMAT_d, -678);
         png_formatted_warning(png_ptr, params, "formatted @1 @2 @3 end");
      }

      png_app_warning(png_ptr, "app warning");

      {
         png_fixed_point res = 0;
         printf("  muldiv=%d res=%ld\n", png_muldiv(&res, 1000, 3, 7), (long)res);
      }

      /* deprecated alias */
      png_set_IHDR(png_ptr, info_ptr, 4, 4, 8, PNG_COLOR_TYPE_GRAY, PNG_INTERLACE_NONE,
                   PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
      {
         /* deprecated chunk writing API */
         png_byte name[5] = { 'p', 'r', 'V', 't', 0 };
         png_write_sig(png_ptr);
         png_write_chunk_start(png_ptr, name, 4);
         png_write_chunk_data(png_ptr, (png_const_bytep)"abcd", 4);
         png_write_chunk_end(png_ptr);
      }
      printf("  invalid index flag\n");
      png_set_check_for_invalid_index(png_ptr, 1);
      png_set_check_for_invalid_index(png_ptr, 0);
      {
         png_time t;
         memset(&t, 0, sizeof t);
         t.year = 1970; t.month = 6; t.day = 15; t.hour = 12; t.minute = 30; t.second = 0;
         printf("  rfc1123='%s'\n", png_convert_to_rfc1123(png_ptr, &t));
      }

      /* These two may turn into errors and longjmp out, so do them last. */
      png_chunk_report(png_ptr, "chunk report warning", PNG_CHUNK_WARNING);
      png_benign_error(png_ptr, "benign error on write");
   }
   else printf("  internals aborted\n");

   png_destroy_write_struct(&png_ptr, &info_ptr);
}

/* Single filter selections: exercises the png_setup_*_row_only paths. */
static void single_filters(void)
{
   static const struct { int f; const char *n; } fl[] = {
      { PNG_FILTER_NONE, "none" }, { PNG_FILTER_SUB, "sub" }, { PNG_FILTER_UP, "up" },
      { PNG_FILTER_AVG, "avg" }, { PNG_FILTER_PAETH, "paeth" },
      { PNG_FILTER_SUB | PNG_FILTER_UP, "sub+up" },
      { PNG_FILTER_AVG | PNG_FILTER_PAETH, "avg+paeth" }
   };
   unsigned i;

   printf("== single filters ==\n");
   for (i = 0; i < sizeof fl / sizeof fl[0]; ++i)
   {
      const char *path = tmpname("harness2_f.png");
      png_structp png_ptr;
      png_infop info_ptr;
      jmp_buf jb;
      FILE *fp = fopen(path, "wb");
      unsigned char *row;
      unsigned y;

      if (fp == NULL) continue;
      png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
      info_ptr = png_create_info_struct(png_ptr);
      if (setjmp(jb) == 0)
      {
         png_init_io(png_ptr, fp);
         png_set_flush(png_ptr, 2);
         png_set_filter(png_ptr, PNG_FILTER_TYPE_BASE, fl[i].f);
         png_set_IHDR(png_ptr, info_ptr, 19, 7, 8, PNG_COLOR_TYPE_RGB_ALPHA,
                      PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_DEFAULT,
                      PNG_FILTER_TYPE_DEFAULT);
         png_write_info(png_ptr, info_ptr);
         row = malloc(19 * 4);
         for (y = 0; y < 7; ++y)
         {
            fill_row(row, 19 * 4, y + 17);
            png_write_row(png_ptr, row);
         }
         png_write_flush(png_ptr);
         png_write_end(png_ptr, info_ptr);
         free(row);
         png_destroy_write_struct(&png_ptr, &info_ptr);
         fclose(fp);
         {
            unsigned char *data;
            size_t n = slurp(path, &data);
            printf("  filter %s size=%lu hash=%016lx\n", fl[i].n, (unsigned long)n,
                   fnv(data, n));
            free(data);
         }
      }
      else
      {
         printf("  filter %s aborted\n", fl[i].n);
         png_destroy_write_struct(&png_ptr, &info_ptr);
         fclose(fp);
      }
      remove(path);
   }
}

/* An ICC profile that fails the checks: exercises png_icc_profile_error and
 * friends (tag name printing, signature validation, chunk reports).
 */
static void bad_icc(void)
{
   static const struct { int what; const char *n; } cases[] = {
      { 0, "short" }, { 1, "badsig" }, { 2, "badspace" }, { 3, "badclass" },
      { 4, "badpcs" }, { 5, "badtagcount" }, { 6, "badtagoffset" }, { 7, "badintent" }
   };
   unsigned i;

   printf("== bad ICC profiles ==\n");
   for (i = 0; i < sizeof cases / sizeof cases[0]; ++i)
   {
      png_structp png_ptr;
      png_infop info_ptr;
      jmp_buf jb;
      png_uint_32 plen;
      png_byte *profile = make_icc_profile(1, &plen);

      switch (cases[i].what)
      {
         case 0: png_save_uint_32(profile, 100); plen = 100; break;
         case 1: memcpy(profile + 36, "junk", 4); break;
         case 2: memcpy(profile + 16, "CMYK", 4); break;
         case 3: memcpy(profile + 12, "abst", 4); break;
         case 4: memcpy(profile + 20, "junk", 4); break;
         case 5: png_save_uint_32(profile + 128, 1000000); break;
         case 6: png_save_uint_32(profile + 136, 7); break;
         case 7: png_save_uint_32(profile + 64, 77); break;
      }

      printf("-- bad icc %s\n", cases[i].n);
      png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
      info_ptr = png_create_info_struct(png_ptr);
      if (setjmp(jb) == 0)
      {
         png_set_benign_errors(png_ptr, 1);
         png_set_IHDR(png_ptr, info_ptr, 4, 4, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
                      PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
         png_set_iCCP(png_ptr, info_ptr, "bad profile", PNG_COMPRESSION_TYPE_BASE,
                      profile, plen);
         printf("  valid=%08lx\n",
                (unsigned long)png_get_valid(png_ptr, info_ptr, PNG_INFO_iCCP));
      }
      else printf("  aborted\n");
      png_destroy_write_struct(&png_ptr, &info_ptr);
      free(profile);
   }
}

/* Simplified API failure paths: png_image_error / png_safe_error / png_safe_warning. */
static void data_skip(const unsigned char *data, size_t size)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;

   printf("== process_data_skip ==\n");
   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   info_ptr = png_create_info_struct(png_ptr);
   if (setjmp(jb) == 0)
   {
      png_set_progressive_read_fn(png_ptr, NULL, NULL, NULL, NULL);
      png_process_data(png_ptr, info_ptr, (png_bytep)data, size < 40 ? size : 40);
      printf("  skip=%lu\n", (unsigned long)png_process_data_skip(png_ptr));
   }
   else printf("  skip aborted\n");
   png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
}

static void simplified_errors(void)
{
   png_image image;
   static const unsigned char junk[16] =
      { 137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 'I', 'H', 'D', 'R' };

   printf("== simplified errors ==\n");
   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   printf("  truncated: %d '%s'\n",
          png_image_begin_read_from_memory(&image, junk, sizeof junk), image.message);
   png_image_free(&image);

   memset(&image, 0, sizeof image);
   image.version = 0;
   printf("  bad version: %d '%s'\n",
          png_image_begin_read_from_memory(&image, junk, sizeof junk), image.message);
   png_image_free(&image);

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   printf("  no memory: %d '%s'\n",
          png_image_begin_read_from_memory(&image, NULL, 0), image.message);
   png_image_free(&image);

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   printf("  missing file: %d '%s'\n",
          png_image_begin_read_from_file(&image, "/nonexistent/harness2/none.png"),
          image.message);
   png_image_free(&image);

   /* A write with an invalid format */
   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = 4;
   image.height = 4;
   image.format = 0xff;
   {
      unsigned char buf[256];
      size_t size = sizeof buf;
      unsigned char out[4096];
      size_t outsize = sizeof out;
      memset(buf, 0x5a, sizeof buf);
      printf("  bad format write: %d '%s'\n",
             png_image_write_to_memory(&image, out, &outsize, 0, buf, 0, NULL),
             image.message);
      (void)size;
   }
   png_image_free(&image);
}

/* Simplified write of a colour-mapped image: png_image_set_PLTE, png_unpremultiply. */
static void simplified_colormap_write(void)
{
   png_image image;
   png_byte colormap[4 * 256];
   png_byte pixels[16 * 9];
   const char *path = tmpname("harness2_cmap.png");
   unsigned i;

   printf("== simplified colormap write ==\n");
   for (i = 0; i < 256; ++i)
   {
      colormap[4 * i + 0] = (png_byte)(i * 3 + 1);
      colormap[4 * i + 1] = (png_byte)(255 - i);
      colormap[4 * i + 2] = (png_byte)(i * 7 + 3);
      colormap[4 * i + 3] = (png_byte)(i | 0x40);
   }
   for (i = 0; i < sizeof pixels; ++i) pixels[i] = (png_byte)((i * 37) & 0xff);

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = 16;
   image.height = 9;
   image.format = PNG_FORMAT_RGBA_COLORMAP;
   image.colormap_entries = 256;

   if (png_image_write_to_file(&image, path, 0, pixels, 16, colormap) != 0)
   {
      unsigned char *data;
      size_t n = slurp(path, &data);
      printf("  colormap write %lu bytes hash=%016lx\n", (unsigned long)n, fnv(data, n));
      free(data);
   }
   else printf("  colormap write failed: %s\n", image.message);
   png_image_free(&image);
   remove(path);

   /* Premultiplied (associated) alpha input exercises png_unpremultiply. */
   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = 8;
   image.height = 5;
   image.format = PNG_FORMAT_LINEAR_RGB_ALPHA | PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
   {
      png_uint_16 buf16[8 * 5 * 4];
      unsigned j;
      for (j = 0; j < 8 * 5 * 4; ++j)
         buf16[j] = (png_uint_16)((j * 1234) & 0xffff);
      /* make sure alpha >= colour components (premultiplied) */
      for (j = 0; j < 8 * 5; ++j)
      {
         png_uint_16 a = 0xf000;
         buf16[4 * j + 3] = a;
         buf16[4 * j + 0] = (png_uint_16)(buf16[4 * j + 0] % (a + 1));
         buf16[4 * j + 1] = (png_uint_16)(buf16[4 * j + 1] % (a + 1));
         buf16[4 * j + 2] = (png_uint_16)(buf16[4 * j + 2] % (a + 1));
      }
      if (png_image_write_to_file(&image, path, 1 /*convert_to_8_bit*/, buf16, 0, NULL) != 0)
      {
         unsigned char *data;
         size_t n = slurp(path, &data);
         printf("  premultiplied write %lu bytes hash=%016lx\n", (unsigned long)n,
                fnv(data, n));
         free(data);
      }
      else printf("  premultiplied write failed: %s\n", image.message);
   }
   png_image_free(&image);
   remove(path);
}


/* Simplified reads of a non-sRGB (linear gAMA) file: exercises the colour-map
 * composition and background handling paths.
 */
static void simplified_linear(const char *path)
{
   static const unsigned formats[] = {
      PNG_FORMAT_RGB, PNG_FORMAT_RGBA, PNG_FORMAT_RGB_COLORMAP,
      PNG_FORMAT_RGBA_COLORMAP, PNG_FORMAT_GRAY, PNG_FORMAT_GA,
      PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP,
      PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP,
      PNG_FORMAT_LINEAR_RGB, PNG_FORMAT_LINEAR_RGB_ALPHA
   };
   unsigned f, bg;

   for (bg = 0; bg < 2; ++bg)
   for (f = 0; f < sizeof formats / sizeof formats[0]; ++f)
   {
      png_image ri;
      memset(&ri, 0, sizeof ri);
      ri.version = PNG_IMAGE_VERSION;
      if (png_image_begin_read_from_file(&ri, path) != 0)
      {
         unsigned char *out;
         unsigned char *cmap = NULL;
         png_color background;

         ri.format = formats[f];
         out = malloc(PNG_IMAGE_SIZE(ri) + 16);
         memset(out, 0, PNG_IMAGE_SIZE(ri) + 16);
         if ((formats[f] & PNG_FORMAT_FLAG_COLORMAP) != 0)
         {
            cmap = malloc(PNG_IMAGE_COLORMAP_SIZE(ri) + 16);
            memset(cmap, 0, PNG_IMAGE_COLORMAP_SIZE(ri) + 16);
         }
         background.red = 0x33; background.green = 0x66; background.blue = 0x99;
         if (png_image_finish_read(&ri, bg ? &background : NULL, out, 0, cmap) != 0)
            printf("  linear bg=%u fmt=%u flags=%u entries=%u hash=%016lx cmap=%016lx\n",
                   bg, formats[f], (unsigned)ri.flags, (unsigned)ri.colormap_entries,
                   fnv(out, PNG_IMAGE_SIZE(ri)),
                   cmap != NULL ? fnv(cmap, PNG_IMAGE_COLORMAP_SIZE(ri)) : 0);
         else
            printf("  linear bg=%u fmt=%u failed: %s\n", bg, formats[f], ri.message);
         free(out);
         free(cmap);
      }
      else printf("  linear begin failed: %s\n", ri.message);
      png_image_free(&ri);
   }
}

/* ==================================================================== 8 ==== */
/* Hand crafted PNG datastreams: lets us build chunk layouts libpng's own
 * writer never produces (hIST before PLTE, MNG intrapixel filtering, broken
 * iCCP profiles, ...).
 */
typedef struct { unsigned char *d; size_t n, cap; } buf_t;

static void bput(buf_t *b, const void *p, size_t n)
{
   if (b->n + n > b->cap)
   {
      b->cap = (b->n + n) * 2 + 256;
      b->d = realloc(b->d, b->cap);
   }
   memcpy(b->d + b->n, p, n);
   b->n += n;
}

static void bchunk(buf_t *b, const char *type, const unsigned char *data, size_t n)
{
   unsigned char len[4];
   unsigned char crcb[4];
   unsigned long crc;
   png_save_uint_32(len, (png_uint_32)n);
   bput(b, len, 4);
   bput(b, type, 4);
   if (n > 0) bput(b, data, n);
   crc = crc32(0, (const Bytef*)type, 4);
   if (n > 0) crc = crc32(crc, (const Bytef*)data, (uInt)n);
   png_save_uint_32(crcb, (png_uint_32)crc);
   bput(b, crcb, 4);
}

static void bihdr(buf_t *b, png_uint_32 w, png_uint_32 h, int bd, int ct, int filter,
                  int interlace)
{
   unsigned char ihdr[13];
   png_save_uint_32(ihdr, w);
   png_save_uint_32(ihdr + 4, h);
   ihdr[8] = (unsigned char)bd;
   ihdr[9] = (unsigned char)ct;
   ihdr[10] = 0;
   ihdr[11] = (unsigned char)filter;
   ihdr[12] = (unsigned char)interlace;
   bchunk(b, "IHDR", ihdr, 13);
}

static void bidat(buf_t *b, png_uint_32 w, png_uint_32 h, size_t rowbytes, unsigned seed)
{
   /* Uncompressed rows with filter byte 0, deflated with zlib. */
   size_t raw = (rowbytes + 1) * h;
   unsigned char *rows = malloc(raw);
   unsigned long clen;
   unsigned char *comp;
   png_uint_32 y;

   for (y = 0; y < h; ++y)
   {
      rows[(rowbytes + 1) * y] = 0;
      fill_row(rows + (rowbytes + 1) * y + 1, rowbytes, seed + y);
   }
   clen = compressBound(raw);
   comp = malloc(clen);
   if (compress2(comp, &clen, rows, raw, 6) == Z_OK)
      bchunk(b, "IDAT", comp, clen);
   free(rows);
   free(comp);
   (void)w;
}

static const unsigned char png_sig8[8] = { 137, 80, 78, 71, 13, 10, 26, 10 };

static void read_crafted(const unsigned char *data, size_t size, int mng,
                         const char *tag)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   rdbuf2 rb;
   unsigned y, height;
   size_t rowbytes;
   unsigned char *row;
   unsigned long h = 0;

   rb.data = data; rb.size = size; rb.pos = 0;
   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   info_ptr = png_create_info_struct(png_ptr);
   if (setjmp(jb))
   {
      printf("  %s aborted\n", tag);
      png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
      return;
   }
   png_set_read_fn(png_ptr, &rb, mem_read2);
   if (mng) png_permit_mng_features(png_ptr, PNG_ALL_MNG_FEATURES);
   png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_ALWAYS, NULL, 0);
   png_read_info(png_ptr, info_ptr);

   printf("  %s %ux%u ct=%d bd=%d valid=%08lx\n", tag,
          (unsigned)png_get_image_width(png_ptr, info_ptr),
          (unsigned)png_get_image_height(png_ptr, info_ptr),
          png_get_color_type(png_ptr, info_ptr),
          png_get_bit_depth(png_ptr, info_ptr),
          (unsigned long)png_get_valid(png_ptr, info_ptr, 0xffffffff));
   {
      png_uint_16p hist = NULL;
      if (png_get_hIST(png_ptr, info_ptr, &hist) != 0)
         printf("  %s hIST hash=%016lx\n", tag, fnv((unsigned char*)hist, 512));
   }
   {
      png_charp name = NULL;
      int comp = 0;
      png_bytep prof = NULL;
      png_uint_32 plen = 0;
      if (png_get_iCCP(png_ptr, info_ptr, &name, &comp, &prof, &plen) != 0)
         printf("  %s iCCP '%s' len=%u\n", tag, name, (unsigned)plen);
   }
   {
      png_bytep exif = NULL;
      png_uint_32 n = 0;
      /* deprecated getter */
      if (png_get_eXIf(png_ptr, info_ptr, &exif) != 0)
         printf("  %s eXIf(dep) ptr=%d\n", tag, exif != NULL);
      if (png_get_eXIf_1(png_ptr, info_ptr, &n, &exif) != 0)
         printf("  %s eXIf n=%u\n", tag, (unsigned)n);
   }

   png_read_update_info(png_ptr, info_ptr);
   height = png_get_image_height(png_ptr, info_ptr);
   rowbytes = png_get_rowbytes(png_ptr, info_ptr);
   row = malloc(rowbytes + 8);
   for (y = 0; y < height; ++y)
   {
      memset(row, 0, rowbytes + 8);
      png_read_row(png_ptr, row, NULL);
      h ^= fnv(row, rowbytes) * (y + 1);
   }
   printf("  %s rows hash=%016lx\n", tag, h);
   png_read_end(png_ptr, info_ptr);
   free(row);
   png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
}

static void crafted(void)
{
   printf("== crafted datastreams ==\n");

   /* (a) hIST before PLTE: covers the hIST reader body. */
   {
      buf_t b;
      unsigned char plte[3 * 16];
      unsigned char hist[2 * 16];
      int i;
      memset(&b, 0, sizeof b);
      bput(&b, png_sig8, 8);
      bihdr(&b, 8, 4, 4, PNG_COLOR_TYPE_PALETTE, 0, 0);
      for (i = 0; i < 16; ++i)
      {
         plte[3 * i + 0] = (unsigned char)(i * 16);
         plte[3 * i + 1] = (unsigned char)(255 - i * 16);
         plte[3 * i + 2] = (unsigned char)(i * 7);
         png_save_uint_16(hist + 2 * i, (png_uint_16)(i * 100 + 1));
      }
      bchunk(&b, "hIST", hist, sizeof hist);
      bchunk(&b, "PLTE", plte, sizeof plte);
      bidat(&b, 8, 4, 4, 5);
      bchunk(&b, "IEND", NULL, 0);
      read_crafted(b.d, b.n, 0, "hIST-first");
      free(b.d);
   }

   /* (b) hIST after PLTE but with the correct declared size. */
   {
      buf_t b;
      unsigned char plte[3 * 4];
      unsigned char hist[2 * 4];
      int i;
      memset(&b, 0, sizeof b);
      bput(&b, png_sig8, 8);
      bihdr(&b, 8, 4, 2, PNG_COLOR_TYPE_PALETTE, 0, 0);
      for (i = 0; i < 4; ++i)
      {
         plte[3 * i + 0] = (unsigned char)(i * 60);
         plte[3 * i + 1] = (unsigned char)(i * 30);
         plte[3 * i + 2] = (unsigned char)(i * 15);
         png_save_uint_16(hist + 2 * i, (png_uint_16)(i + 7));
      }
      bchunk(&b, "PLTE", plte, sizeof plte);
      bchunk(&b, "hIST", hist, sizeof hist);
      bidat(&b, 8, 4, 2, 9);
      bchunk(&b, "IEND", NULL, 0);
      read_crafted(b.d, b.n, 0, "hIST-after");
      free(b.d);
   }

   /* (c) MNG intrapixel differencing (filter method 64). */
   {
      buf_t b;
      memset(&b, 0, sizeof b);
      bput(&b, png_sig8, 8);
      bihdr(&b, 6, 3, 8, PNG_COLOR_TYPE_RGB, PNG_INTRAPIXEL_DIFFERENCING, 0);
      bidat(&b, 6, 3, 18, 13);
      bchunk(&b, "IEND", NULL, 0);
      read_crafted(b.d, b.n, 1, "mng-intrapixel8");
      read_crafted(b.d, b.n, 0, "mng-not-permitted");
      free(b.d);
   }
   {
      buf_t b;
      memset(&b, 0, sizeof b);
      bput(&b, png_sig8, 8);
      bihdr(&b, 5, 3, 16, PNG_COLOR_TYPE_RGB, PNG_INTRAPIXEL_DIFFERENCING, 0);
      bidat(&b, 5, 3, 30, 21);
      bchunk(&b, "IEND", NULL, 0);
      read_crafted(b.d, b.n, 1, "mng-intrapixel16");
      free(b.d);
   }

   /* (d) broken ICC profiles inside a real iCCP chunk. */
   {
      static const struct { int what; const char *n; } icc[] = {
         { 0, "short" }, { 1, "badsig" }, { 2, "badspace" }, { 3, "abstract" },
         { 4, "badpcs" }, { 5, "tagcount" }, { 6, "tagoffset" }, { 7, "intent" },
         { 8, "notd50" }, { 9, "badtagsig" }, { 10, "ok" }
      };
      unsigned k;
      for (k = 0; k < sizeof icc / sizeof icc[0]; ++k)
      {
         buf_t b;
         png_uint_32 plen;
         png_byte *profile = make_icc_profile(1, &plen);
         unsigned char *chunk;
         unsigned long clen;
         size_t namelen = 5; /* "icc" + nul + method */

         switch (icc[k].what)
         {
            case 0: png_save_uint_32(profile, 100); break;
            case 1: memcpy(profile + 36, "junk", 4); break;
            case 2: memcpy(profile + 16, "CMYK", 4); break;
            case 3: memcpy(profile + 12, "abst", 4); break;
            case 4: memcpy(profile + 20, "junk", 4); break;
            case 5: png_save_uint_32(profile + 128, 100000); break;
            case 6: png_save_uint_32(profile + 136, 7); break;
            case 7: png_save_uint_32(profile + 64, 77); break;
            case 8: memset(profile + 68, 0, 12); break;
            case 9: memcpy(profile + 132, "\001\002\003\004", 4); break;
            default: break;
         }

         clen = compressBound(plen);
         chunk = malloc(namelen + clen);
         memcpy(chunk, "icc", 3);
         chunk[3] = 0;
         chunk[4] = 0; /* compression method */
         if (compress2(chunk + namelen, &clen, profile, plen, 6) == Z_OK)
         {
            memset(&b, 0, sizeof b);
            bput(&b, png_sig8, 8);
            bihdr(&b, 4, 2, 8, PNG_COLOR_TYPE_RGB, 0, 0);
            bchunk(&b, "iCCP", chunk, namelen + clen);
            bidat(&b, 4, 2, 12, 33);
            bchunk(&b, "IEND", NULL, 0);
            printf("-- icc %s\n", icc[k].n);
            read_crafted(b.d, b.n, 0, icc[k].n);
            free(b.d);
         }
         free(chunk);
         free(profile);
      }
   }

   /* (d2) a linear-gamma 16 bit RGBA file for the simplified colour-map paths. */
   {
      buf_t b;
      unsigned char gama[4];
      const char *path = tmpname("harness2_linear.png");
      FILE *f;
      memset(&b, 0, sizeof b);
      bput(&b, png_sig8, 8);
      bihdr(&b, 9, 5, 16, PNG_COLOR_TYPE_RGB_ALPHA, 0, 0);
      png_save_uint_32(gama, 100000); /* gamma 1.0: not sRGB */
      bchunk(&b, "gAMA", gama, 4);
      bidat(&b, 9, 5, 9 * 8, 57);
      bchunk(&b, "IEND", NULL, 0);
      read_crafted(b.d, b.n, 0, "linear16");
      f = fopen(path, "wb");
      if (f != NULL)
      {
         fwrite(b.d, 1, b.n, f);
         fclose(f);
         printf("== simplified linear ==\n");
         simplified_linear(path);
         remove(path);
      }
      free(b.d);
   }

   /* (e) eXIf, cICP, cLLI, mDCV, sTER (unknown) and a duplicate gAMA. */
   {
      buf_t b;
      unsigned char gama[4], exif[10], cicp[4], clli[8], mdcv[24], ster[1];
      memset(&b, 0, sizeof b);
      bput(&b, png_sig8, 8);
      bihdr(&b, 4, 2, 8, PNG_COLOR_TYPE_RGB, 0, 0);
      png_save_uint_32(gama, 45455);
      bchunk(&b, "gAMA", gama, 4);
      bchunk(&b, "gAMA", gama, 4); /* duplicate */
      memcpy(exif, "MM\0*\0\0\0\010\0\001", 10);
      bchunk(&b, "eXIf", exif, 10);
      cicp[0] = 9; cicp[1] = 16; cicp[2] = 0; cicp[3] = 1;
      bchunk(&b, "cICP", cicp, 4);
      png_save_uint_32(clli, 10000000);
      png_save_uint_32(clli + 4, 4000000);
      bchunk(&b, "cLLI", clli, 8);
      {
         int i;
         for (i = 0; i < 8; ++i) png_save_uint_16(mdcv + 2 * i, (png_uint_16)(1000 * (i + 1)));
         png_save_uint_32(mdcv + 16, 10000000);
         png_save_uint_32(mdcv + 20, 50);
         bchunk(&b, "mDCV", mdcv, 24);
      }
      ster[0] = 1;
      bchunk(&b, "sTER", ster, 1);
      bidat(&b, 4, 2, 12, 41);
      bchunk(&b, "IEND", NULL, 0);
      read_crafted(b.d, b.n, 0, "extra-chunks");
      free(b.d);
   }
}

int main(void)
{
   static const struct { int ct, bd, mng, il; const char *name; } cases[] = {
      { PNG_COLOR_TYPE_GRAY, 1, 0, PNG_INTERLACE_NONE, "gray1" },
      { PNG_COLOR_TYPE_GRAY, 8, 0, PNG_INTERLACE_ADAM7, "gray8i" },
      { PNG_COLOR_TYPE_GRAY, 16, 0, PNG_INTERLACE_NONE, "gray16" },
      { PNG_COLOR_TYPE_GRAY_ALPHA, 8, 0, PNG_INTERLACE_NONE, "ga8" },
      { PNG_COLOR_TYPE_PALETTE, 4, 0, PNG_INTERLACE_NONE, "pal4" },
      { PNG_COLOR_TYPE_PALETTE, 8, 0, PNG_INTERLACE_ADAM7, "pal8i" },
      { PNG_COLOR_TYPE_RGB, 8, 0, PNG_INTERLACE_NONE, "rgb8" },
      { PNG_COLOR_TYPE_RGB, 8, 1, PNG_INTERLACE_NONE, "rgb8mng" },
      { PNG_COLOR_TYPE_RGB, 16, 1, PNG_INTERLACE_NONE, "rgb16mng" },
      { PNG_COLOR_TYPE_RGB_ALPHA, 8, 0, PNG_INTERLACE_NONE, "rgba8" },
      { PNG_COLOR_TYPE_RGB_ALPHA, 16, 0, PNG_INTERLACE_ADAM7, "rgba16i" }
   };
   unsigned i;
   unsigned char *keep = NULL;
   size_t keepsize = 0;

   for (i = 0; i < sizeof cases / sizeof cases[0]; ++i)
   {
      const char *path = tmpname("harness2.png");
      printf("== file %s (ct=%d bd=%d mng=%d il=%d) ==\n", cases[i].name, cases[i].ct,
             cases[i].bd, cases[i].mng, cases[i].il);
      if (write_file(path, cases[i].ct, cases[i].bd, cases[i].mng, cases[i].il, 21, 13) != 0)
      {
         unsigned char *data;
         size_t n = slurp(path, &data);
         printf("  file size=%lu hash=%016lx\n", (unsigned long)n, fnv(data, n));
         read_file(path, cases[i].mng, cases[i].name);
         if (keep == NULL && cases[i].mng == 0)
         {
            keep = data; keepsize = n;
         }
         else free(data);
      }
      remove(path);
   }

   default_handlers();
   info_utilities();
   internals();
   single_filters();
   bad_icc();
   simplified_errors();
   simplified_colormap_write();
   crafted();

   if (keep != NULL) progressive2(keep, keepsize);
   if (keep != NULL) data_skip(keep, keepsize);

   printf("== simplified colour-mapped ==\n");
   simplified2(17, 11, PNG_FORMAT_RGB, PNG_FORMAT_RGB_COLORMAP, 0, 1);
   simplified2(17, 11, PNG_FORMAT_RGBA, PNG_FORMAT_RGBA_COLORMAP, 0, 1);
   simplified2(17, 11, PNG_FORMAT_RGBA, PNG_FORMAT_RGB, 1, 1);
   simplified2(17, 11, PNG_FORMAT_GA, PNG_FORMAT_GRAY, 1, 0);
   simplified2(17, 11, PNG_FORMAT_GA, PNG_FORMAT_GA, 0, 0);
   simplified2(17, 11, PNG_FORMAT_LINEAR_RGB_ALPHA, PNG_FORMAT_RGBA_COLORMAP, 0, 1);
   simplified2(17, 11, PNG_FORMAT_LINEAR_Y, PNG_FORMAT_GRAY, 0, 0);
   simplified2(17, 11, PNG_FORMAT_GRAY, PNG_FORMAT_RGB_COLORMAP, 0, 1);
   simplified2(17, 11, PNG_FORMAT_BGRA, PNG_FORMAT_ABGR_COLORMAP, 1, 1);
   simplified2(17, 11, PNG_FORMAT_GRAY, PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP, 0, 1);
   simplified2(17, 11, PNG_FORMAT_GA, PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP, 0, 1);
   simplified2(17, 11, PNG_FORMAT_GA, PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP, 1, 1);
   simplified2(17, 11, PNG_FORMAT_LINEAR_Y, PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP, 1, 1);
   simplified2(17, 11, PNG_FORMAT_LINEAR_Y_ALPHA, PNG_FORMAT_GA, 1, 0);
   simplified2(17, 11, PNG_FORMAT_LINEAR_RGB, PNG_FORMAT_RGB, 0, 0);
   simplified2(17, 11, PNG_FORMAT_RGBA, PNG_FORMAT_LINEAR_RGB_ALPHA, 0, 1);
   simplified2(17, 11, PNG_FORMAT_RGB, PNG_FORMAT_LINEAR_Y, 0, 1);

   free(keep);
   printf("== done2 ==\n");
   return 0;
}
