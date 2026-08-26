/* Behavioural comparison harness for the C and Rust libpng builds.
 *
 * Uses only the public libpng API.  Everything it prints is deterministic, so
 * the output of the C build and of the Rust build can be diffed byte for byte.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include "png.h"
#include <zlib.h>

/* png_xy / png_XYZ live in the private header pngstruct.h; the two conversion
 * functions are exported, so declare what we need locally.
 */
typedef struct { png_fixed_point redx, redy, greenx, greeny, bluex, bluey, whitex, whitey; } h_png_xy;
typedef struct { png_fixed_point red_X, red_Y, red_Z, green_X, green_Y, green_Z,
                 blue_X, blue_Y, blue_Z; } h_png_XYZ;
extern int png_XYZ_from_xy(h_png_XYZ *XYZ, const h_png_xy *xy);
extern int png_xy_from_XYZ(h_png_xy *xy, const h_png_XYZ *XYZ);
extern png_uint_32 png_get_uint_31(png_const_structrp png_ptr, png_const_bytep buf);
extern void png_build_grayscale_palette(int bit_depth, png_colorp palette);
extern png_fixed_point png_fixed(png_const_structrp png_ptr, double fp, png_const_charp text);
extern png_uint_32 png_fixed_ITU(png_const_structrp png_ptr, double fp, png_const_charp text);
extern int png_muldiv(png_fixed_point *res, png_fixed_point a, png_int_32 times, png_int_32 div);
extern png_fixed_point png_reciprocal(png_fixed_point a);
extern png_fixed_point png_reciprocal2(png_fixed_point a, png_fixed_point b);
extern int png_gamma_significant(png_fixed_point gamma_value);
extern png_uint_16 png_gamma_16bit_correct(unsigned int value, png_fixed_point gamma_value);
extern png_byte png_gamma_8bit_correct(unsigned int value, png_fixed_point gamma_value);
extern int png_check_fp_number(png_const_charp string, size_t size, int *statep, size_t *whereami);
extern int png_check_fp_string(png_const_charp string, size_t size);
extern void png_ascii_from_fp(png_const_structrp png_ptr, png_charp ascii, size_t size, double fp, unsigned int precision);
extern void png_ascii_from_fixed(png_const_structrp png_ptr, png_charp ascii, size_t size, png_fixed_point fp);
extern size_t png_safecat(png_charp buffer, size_t bufsize, size_t pos, png_const_charp string);
extern png_charp png_format_number(png_const_charp start, png_charp end, int format, png_alloc_size_t number);
#define PNG_NUMBER_FORMAT_fixed 5


static unsigned long fnv(const unsigned char *p, size_t n)
{
   unsigned long h = 1469598103934665603UL;
   size_t i;
   for (i = 0; i < n; ++i) { h ^= p[i]; h *= 1099511628211UL; }
   return h;
}

/* ------------------------------------------------------------------ memory IO */
typedef struct { unsigned char *data; size_t size, cap; } membuf;

static void mem_write(png_structp png_ptr, png_bytep data, size_t length)
{
   membuf *mb = (membuf*)png_get_io_ptr(png_ptr);
   if (mb->size + length > mb->cap)
   {
      mb->cap = (mb->size + length) * 2 + 1024;
      mb->data = realloc(mb->data, mb->cap);
   }
   memcpy(mb->data + mb->size, data, length);
   mb->size += length;
}
static void mem_flush(png_structp png_ptr) { (void)png_ptr; }

typedef struct { const unsigned char *data; size_t size, pos; } rdbuf;

static void mem_read(png_structp png_ptr, png_bytep data, size_t length)
{
   rdbuf *rb = (rdbuf*)png_get_io_ptr(png_ptr);
   size_t avail = rb->size - rb->pos;
   if (length > avail)
   {
      png_error(png_ptr, "read past end");
      return;
   }
   memcpy(data, rb->data + rb->pos, length);
   rb->pos += length;
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

/* --------------------------------------------------------------- image maker */
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

/* ------------------------------------------------------------------- writing */
static int write_png(membuf *out, int color_type, int bit_depth, int interlace,
                     int filters, int level, int strategy, int with_ancillary,
                     unsigned width, unsigned height)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   unsigned char **rows = NULL;
   unsigned y;
   size_t rowbytes;

   out->data = NULL; out->size = out->cap = 0;

   png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   if (png_ptr == NULL) { printf("  create_write_struct failed\n"); return 0; }
   info_ptr = png_create_info_struct(png_ptr);
   if (info_ptr == NULL) { printf("  create_info_struct failed\n"); return 0; }

   if (setjmp(jb))
   {
      png_destroy_write_struct(&png_ptr, &info_ptr);
      if (rows != NULL) { for (y = 0; y < height; ++y) free(rows[y]); free(rows); }
      return 0;
   }

   png_set_write_fn(png_ptr, out, mem_write, mem_flush);
   png_set_filter(png_ptr, 0, filters);
   png_set_compression_level(png_ptr, level);
   png_set_compression_strategy(png_ptr, strategy);
   png_set_compression_window_bits(png_ptr, 14);
   png_set_compression_mem_level(png_ptr, 7);
   png_set_compression_method(png_ptr, 8);
   png_set_compression_buffer_size(png_ptr, 4096);

   png_set_IHDR(png_ptr, info_ptr, width, height, bit_depth, color_type,
                interlace, PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);

   if (color_type & PNG_COLOR_MASK_PALETTE)
   {
      png_color pal[256];
      png_byte trans[256];
      png_uint_16 hist[256];
      int i, n = 1 << bit_depth;
      if (n > 256) n = 256;
      for (i = 0; i < n; ++i)
      {
         pal[i].red = (png_byte)(i * 3 + 1);
         pal[i].green = (png_byte)(255 - i * 2);
         pal[i].blue = (png_byte)(i * 7 + 11);
         trans[i] = (png_byte)(i * 5 + 3);
         hist[i] = (png_uint_16)(i * 37 + 1);
      }
      png_set_PLTE(png_ptr, info_ptr, pal, n);
      if (with_ancillary)
      {
         png_set_tRNS(png_ptr, info_ptr, trans, n, NULL);
         png_set_hIST(png_ptr, info_ptr, hist);
      }
   }
   else if (with_ancillary && !(color_type & PNG_COLOR_MASK_ALPHA))
   {
      png_color_16 tc;
      unsigned maxv = (bit_depth >= 16) ? 65535u : ((1u << bit_depth) - 1u);
      memset(&tc, 0, sizeof tc);
      tc.red = (png_uint_16)(12 % (maxv + 1));
      tc.green = (png_uint_16)(34 % (maxv + 1));
      tc.blue = (png_uint_16)(56 % (maxv + 1));
      tc.gray = (png_uint_16)(7 % (maxv + 1));
      png_set_tRNS(png_ptr, info_ptr, NULL, 0, &tc);
   }

   if (with_ancillary)
   {
      png_color_16 bkgd;
      png_color_8 sbit;
      png_text text[3];
      png_time mod_time;
      png_byte exif[14] = { 73, 73, 42, 0, 8, 0, 0, 0, 1, 0, 0, 1, 3, 0 };
      png_sPLT_t splt;
      png_sPLT_entry entries[4];
      png_byte profile[132];
      int i;

      memset(&bkgd, 0, sizeof bkgd);
      {
         unsigned maxb = (bit_depth >= 16) ? 65535u : ((1u << bit_depth) - 1u);
         bkgd.index = 1;
         bkgd.red = (png_uint_16)(100 % (maxb + 1));
         bkgd.green = (png_uint_16)(200 % (maxb + 1));
         bkgd.blue = (png_uint_16)(300 % (maxb + 1));
         bkgd.gray = (png_uint_16)(42 % (maxb + 1));
      }
      png_set_bKGD(png_ptr, info_ptr, &bkgd);

      memset(&sbit, 0, sizeof sbit);
      sbit.red = sbit.green = sbit.blue = sbit.gray = (png_byte)(bit_depth > 8 ? 12 : (bit_depth > 4 ? 5 : 1));
      sbit.alpha = sbit.red;
      png_set_sBIT(png_ptr, info_ptr, &sbit);

      png_set_gAMA_fixed(png_ptr, info_ptr, 45455);
      png_set_cHRM_fixed(png_ptr, info_ptr, 31270, 32900, 64000, 33000,
                         30000, 60000, 15000, 6000);
      png_set_pHYs(png_ptr, info_ptr, 3779, 3779, PNG_RESOLUTION_METER);
      png_set_oFFs(png_ptr, info_ptr, -17, 39, PNG_OFFSET_PIXEL);
      png_set_sCAL_fixed(png_ptr, info_ptr, PNG_SCALE_METER, 123456, 654321);
      png_set_cICP(png_ptr, info_ptr, 9, 16, 0, 1);
      png_set_cLLI_fixed(png_ptr, info_ptr, 10000000, 4000000);
      png_set_mDCV_fixed(png_ptr, info_ptr, 34000, 16000, 13250, 34500,
                         7500, 3000, 15635, 16450, 10000000, 50);
      png_set_eXIf_1(png_ptr, info_ptr, sizeof exif, exif);

      memset(&mod_time, 0, sizeof mod_time);
      mod_time.year = 2024; mod_time.month = 6; mod_time.day = 15;
      mod_time.hour = 13; mod_time.minute = 45; mod_time.second = 59;
      png_set_tIME(png_ptr, info_ptr, &mod_time);

      memset(text, 0, sizeof text);
      text[0].compression = PNG_TEXT_COMPRESSION_NONE;
      text[0].key = (png_charp)"Title";
      text[0].text = (png_charp)"A plain tEXt chunk";
      text[1].compression = PNG_TEXT_COMPRESSION_zTXt;
      text[1].key = (png_charp)"Description";
      text[1].text = (png_charp)"compressed zTXt data compressed zTXt data compressed zTXt data";
      text[2].compression = PNG_ITXT_COMPRESSION_NONE;
      text[2].key = (png_charp)"Comment";
      text[2].text = (png_charp)"an iTXt chunk";
      text[2].lang = (png_charp)"en";
      text[2].lang_key = (png_charp)"Comment";
      png_set_text(png_ptr, info_ptr, text, 3);

      for (i = 0; i < 4; ++i)
      {
         entries[i].red = (png_uint_16)(i * 1000);
         entries[i].green = (png_uint_16)(i * 2000);
         entries[i].blue = (png_uint_16)(i * 3000);
         entries[i].alpha = (png_uint_16)(i * 4000);
         entries[i].frequency = (png_uint_16)(i + 1);
      }
      splt.name = (png_charp)"suggested";
      splt.depth = 16;
      splt.entries = entries;
      splt.nentries = 4;
      png_set_sPLT(png_ptr, info_ptr, &splt, 1);

      memset(profile, 0, sizeof profile);
      /* Minimal-ish fake profile; libpng validates the header. */
      profile[0] = 0; profile[1] = 0; profile[2] = 0; profile[3] = 132;
      memcpy(profile + 4, "ACSP", 4);
      png_set_unknown_chunks(png_ptr, info_ptr, NULL, 0);
      {
         png_unknown_chunk unk[2];
         memset(unk, 0, sizeof unk);
         memcpy(unk[0].name, "prVt", 5);
         unk[0].data = (png_byte*)"private data";
         unk[0].size = 12;
         unk[0].location = PNG_HAVE_IHDR;
         memcpy(unk[1].name, "poSt", 5);
         unk[1].data = (png_byte*)"after idat";
         unk[1].size = 10;
         unk[1].location = PNG_AFTER_IDAT;
         png_set_unknown_chunks(png_ptr, info_ptr, unk, 2);
      }
   }

   png_write_info(png_ptr, info_ptr);

   rowbytes = png_get_rowbytes(png_ptr, info_ptr);
   printf("  write rowbytes=%lu channels=%d\n", (unsigned long)rowbytes,
          png_get_channels(png_ptr, info_ptr));

   rows = malloc(height * sizeof(unsigned char*));
   for (y = 0; y < height; ++y)
   {
      rows[y] = malloc(rowbytes);
      fill_row(rows[y], rowbytes, y + 1);
   }

   png_write_image(png_ptr, rows);
   png_write_end(png_ptr, info_ptr);

   printf("  wrote %lu bytes hash=%016lx\n", (unsigned long)out->size, fnv(out->data, out->size));

   png_destroy_write_struct(&png_ptr, &info_ptr);
   for (y = 0; y < height; ++y) free(rows[y]);
   free(rows);
   return 1;
}

/* ------------------------------------------------------------------- reading */
static void dump_info2(png_structp png_ptr, png_infop info_ptr, const char *tag, int skip_ihdr)
{
   png_uint_32 w = 0, h = 0;
   int bd = 0, ct = 0, il = 0, comp = 0, filt = 0;
   png_uint_32 valid;

   if (skip_ihdr == 0)
      png_get_IHDR(png_ptr, info_ptr, &w, &h, &bd, &ct, &il, &comp, &filt);
   valid = png_get_valid(png_ptr, info_ptr, 0xffffffff);
   printf("  %s IHDR %ux%u bd=%d ct=%d il=%d comp=%d filt=%d valid=%08lx rowbytes=%lu\n",
          tag, (unsigned)w, (unsigned)h, bd, ct, il, comp, filt,
          (unsigned long)valid, (unsigned long)png_get_rowbytes(png_ptr, info_ptr));
   printf("  %s width=%u height=%u depth=%d ctype=%d chan=%d ilace=%d ctype2=%d ftype=%d\n",
          tag, (unsigned)png_get_image_width(png_ptr, info_ptr),
          (unsigned)png_get_image_height(png_ptr, info_ptr),
          png_get_bit_depth(png_ptr, info_ptr),
          png_get_color_type(png_ptr, info_ptr),
          png_get_channels(png_ptr, info_ptr),
          png_get_interlace_type(png_ptr, info_ptr),
          png_get_compression_type(png_ptr, info_ptr),
          png_get_filter_type(png_ptr, info_ptr));

   {
      double g = 0;
      png_fixed_point gf = 0;
      if (png_get_gAMA(png_ptr, info_ptr, &g) != 0) printf("  %s gAMA %.8f\n", tag, g);
      if (png_get_gAMA_fixed(png_ptr, info_ptr, &gf) != 0) printf("  %s gAMA_fixed %ld\n", tag, (long)gf);
   }
   {
      png_fixed_point wx, wy, rx, ry, gx, gy, bx, by;
      if (png_get_cHRM_fixed(png_ptr, info_ptr, &wx, &wy, &rx, &ry, &gx, &gy, &bx, &by) != 0)
         printf("  %s cHRM %ld %ld %ld %ld %ld %ld %ld %ld\n", tag, (long)wx, (long)wy,
                (long)rx, (long)ry, (long)gx, (long)gy, (long)bx, (long)by);
   }
   {
      png_fixed_point rX, rY, rZ, gX, gY, gZ, bX, bY, bZ;
      if (png_get_cHRM_XYZ_fixed(png_ptr, info_ptr, &rX, &rY, &rZ, &gX, &gY, &gZ, &bX, &bY, &bZ) != 0)
         printf("  %s cHRM_XYZ %ld %ld %ld %ld %ld %ld %ld %ld %ld\n", tag,
                (long)rX, (long)rY, (long)rZ, (long)gX, (long)gY, (long)gZ,
                (long)bX, (long)bY, (long)bZ);
   }
   {
      int intent = -1;
      if (png_get_sRGB(png_ptr, info_ptr, &intent) != 0) printf("  %s sRGB %d\n", tag, intent);
   }
   {
      png_uint_32 res_x = 0, res_y = 0;
      int unit = 0;
      if (png_get_pHYs(png_ptr, info_ptr, &res_x, &res_y, &unit) != 0)
         printf("  %s pHYs %u %u %d ppm(%u,%u) dpi(%u,%u) aspect=%.6f\n", tag,
                (unsigned)res_x, (unsigned)res_y, unit,
                (unsigned)png_get_x_pixels_per_meter(png_ptr, info_ptr),
                (unsigned)png_get_y_pixels_per_meter(png_ptr, info_ptr),
                (unsigned)png_get_x_pixels_per_inch(png_ptr, info_ptr),
                (unsigned)png_get_y_pixels_per_inch(png_ptr, info_ptr),
                png_get_pixel_aspect_ratio(png_ptr, info_ptr));
   }
   {
      png_int_32 ox = 0, oy = 0;
      int unit = 0;
      if (png_get_oFFs(png_ptr, info_ptr, &ox, &oy, &unit) != 0)
         printf("  %s oFFs %ld %ld %d inches(%.6f,%.6f)\n", tag, (long)ox, (long)oy, unit,
                png_get_x_offset_inches(png_ptr, info_ptr),
                png_get_y_offset_inches(png_ptr, info_ptr));
   }
   {
      int unit = 0;
      png_fixed_point sw = 0, sh = 0;
      png_charp swp = NULL, shp = NULL;
      if (png_get_sCAL_fixed(png_ptr, info_ptr, &unit, &sw, &sh) != 0)
         printf("  %s sCAL_fixed %d %ld %ld\n", tag, unit, (long)sw, (long)sh);
      if (png_get_sCAL_s(png_ptr, info_ptr, &unit, &swp, &shp) != 0)
         printf("  %s sCAL_s %d '%s' '%s'\n", tag, unit, swp, shp);
   }
   {
      png_color_16p bkgd = NULL;
      if (png_get_bKGD(png_ptr, info_ptr, &bkgd) != 0)
         printf("  %s bKGD %d %u %u %u %u\n", tag, bkgd->index, bkgd->red, bkgd->green,
                bkgd->blue, bkgd->gray);
   }
   {
      png_color_8p sbit = NULL;
      if (png_get_sBIT(png_ptr, info_ptr, &sbit) != 0)
         printf("  %s sBIT %d %d %d %d %d\n", tag, sbit->red, sbit->green, sbit->blue,
                sbit->gray, sbit->alpha);
   }
   {
      png_colorp pal = NULL;
      int n = 0;
      if (png_get_PLTE(png_ptr, info_ptr, &pal, &n) != 0)
      {
         printf("  %s PLTE %d entries hash=%016lx\n", tag, n,
                fnv((unsigned char*)pal, (size_t)n * sizeof(png_color)));
      }
   }
   {
      png_bytep ta = NULL;
      int nt = 0;
      png_color_16p tc = NULL;
      if (png_get_tRNS(png_ptr, info_ptr, &ta, &nt, &tc) != 0)
      {
         printf("  %s tRNS n=%d", tag, nt);
         if (ta != NULL) printf(" alpha_hash=%016lx", fnv(ta, (size_t)nt));
         if (tc != NULL) printf(" color=%u,%u,%u,%u,%d", tc->red, tc->green, tc->blue,
                                tc->gray, tc->index);
         printf("\n");
      }
   }
   {
      png_uint_16p hist = NULL;
      if (png_get_hIST(png_ptr, info_ptr, &hist) != 0)
         printf("  %s hIST hash=%016lx\n", tag, fnv((unsigned char*)hist, 2 * 256));
   }
   {
      png_timep t = NULL;
      if (png_get_tIME(png_ptr, info_ptr, &t) != 0)
      {
         char buf[29];
         printf("  %s tIME %u-%u-%u %u:%u:%u\n", tag, t->year, t->month, t->day,
                t->hour, t->minute, t->second);
         if (png_convert_to_rfc1123_buffer(buf, t) != 0)
            printf("  %s rfc1123 '%s'\n", tag, buf);
      }
   }
   {
      png_textp text = NULL;
      int nt = 0;
      if (png_get_text(png_ptr, info_ptr, &text, &nt) != 0)
      {
         int i;
         for (i = 0; i < nt; ++i)
            printf("  %s text[%d] comp=%d key='%s' len=%lu itxt=%lu lang='%s' lk='%s' text='%s'\n",
                   tag, i, text[i].compression, text[i].key,
                   (unsigned long)text[i].text_length, (unsigned long)text[i].itxt_length,
                   text[i].lang ? text[i].lang : "", text[i].lang_key ? text[i].lang_key : "",
                   text[i].text ? text[i].text : "");
      }
   }
   {
      png_sPLT_tp splt = NULL;
      int n = png_get_sPLT(png_ptr, info_ptr, &splt);
      int i;
      for (i = 0; i < n; ++i)
         printf("  %s sPLT[%d] '%s' depth=%d n=%ld hash=%016lx\n", tag, i, splt[i].name,
                splt[i].depth, (long)splt[i].nentries,
                fnv((unsigned char*)splt[i].entries,
                    (size_t)splt[i].nentries * sizeof(png_sPLT_entry)));
   }
   {
      png_bytep exif = NULL;
      png_uint_32 n = 0;
      if (png_get_eXIf_1(png_ptr, info_ptr, &n, &exif) != 0)
         printf("  %s eXIf n=%u hash=%016lx\n", tag, (unsigned)n, fnv(exif, n));
   }
   {
      png_byte cp = 0, tf = 0, mc = 0, vf = 0;
      if (png_get_cICP(png_ptr, info_ptr, &cp, &tf, &mc, &vf) != 0)
         printf("  %s cICP %d %d %d %d\n", tag, cp, tf, mc, vf);
   }
   {
      png_uint_32 maxcll = 0, maxfall = 0;
      if (png_get_cLLI_fixed(png_ptr, info_ptr, &maxcll, &maxfall) != 0)
         printf("  %s cLLI %u %u\n", tag, (unsigned)maxcll, (unsigned)maxfall);
   }
   {
      png_fixed_point wx, wy, rx, ry, gx, gy, bx, by;
      png_uint_32 maxdl, mindl;
      if (png_get_mDCV_fixed(png_ptr, info_ptr, &wx, &wy, &rx, &ry, &gx, &gy, &bx, &by,
                             &maxdl, &mindl) != 0)
         printf("  %s mDCV %ld %ld %ld %ld %ld %ld %ld %ld %u %u\n", tag, (long)wx,
                (long)wy, (long)rx, (long)ry, (long)gx, (long)gy, (long)bx, (long)by,
                (unsigned)maxdl, (unsigned)mindl);
      {
         double dwx, dwy, drx, dry, dgx, dgy, dbx, dby, dmax, dmin;
         if (png_get_mDCV(png_ptr, info_ptr, &dwx, &dwy, &drx, &dry, &dgx, &dgy, &dbx,
                          &dby, &dmax, &dmin) != 0)
            printf("  %s mDCV_fp %.6f %.6f %.6f %.6f %.6f %.6f %.6f %.6f %.6f %.6f\n",
                   tag, dwx, dwy, drx, dry, dgx, dgy, dbx, dby, dmax, dmin);
      }
      {
         double cll, fall;
         if (png_get_cLLI(png_ptr, info_ptr, &cll, &fall) != 0)
            printf("  %s cLLI_fp %.6f %.6f\n", tag, cll, fall);
      }
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
      if (png_get_pCAL(png_ptr, info_ptr, &purpose, &X0, &X1, &type, &nparams, &units,
                       &params) != 0)
      {
         int i;
         printf("  %s pCAL '%s' %ld %ld type=%d n=%d units='%s'\n", tag, purpose,
                (long)X0, (long)X1, type, nparams, units);
         for (i = 0; i < nparams; ++i) printf("    param[%d]='%s'\n", i, params[i]);
      }
   }
   {
      png_unknown_chunkp unk = NULL;
      int n = png_get_unknown_chunks(png_ptr, info_ptr, &unk);
      int i;
      for (i = 0; i < n; ++i)
         printf("  %s unknown[%d] '%s' size=%lu loc=%d hash=%016lx\n", tag, i,
                (char*)unk[i].name, (unsigned long)unk[i].size, unk[i].location,
                fnv(unk[i].data, unk[i].size));
   }
   printf("  %s palette_max=%d rgb_to_gray_status=%d io_state=%u signature_hash=%016lx\n",
          tag, png_get_palette_max(png_ptr, info_ptr),
          png_get_rgb_to_gray_status(png_ptr), (unsigned)png_get_io_state(png_ptr),
          fnv(png_get_signature(png_ptr, info_ptr) ? png_get_signature(png_ptr, info_ptr)
                                                   : (png_const_bytep)"", 8));
}

static void dump_info(png_structp png_ptr, png_infop info_ptr, const char *tag)
{
   dump_info2(png_ptr, info_ptr, tag, 0);
}

static int read_png(const unsigned char *data, size_t size, unsigned transforms,
                    int use_high_level, const char *tag)
{
   png_structp png_ptr;
   png_infop info_ptr;
   png_infop end_info;
   jmp_buf jb;
   rdbuf rb;
   unsigned y, height;
   size_t rowbytes;
   unsigned char **rows = NULL;
   unsigned long h = 0;

   rb.data = data; rb.size = size; rb.pos = 0;

   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   if (png_ptr == NULL) { printf("  %s create_read_struct failed\n", tag); return 0; }
   info_ptr = png_create_info_struct(png_ptr);
   end_info = png_create_info_struct(png_ptr);

   if (setjmp(jb))
   {
      printf("  %s read aborted\n", tag);
      png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
      return 0;
   }

   png_set_read_fn(png_ptr, &rb, mem_read);
   png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_ALWAYS, NULL, 0);
   png_set_user_limits(png_ptr, 100000, 100000);
   png_set_chunk_cache_max(png_ptr, 100);
   png_set_chunk_malloc_max(png_ptr, 4000000);
   png_set_crc_action(png_ptr, PNG_CRC_DEFAULT, PNG_CRC_DEFAULT);

   if (use_high_level != 0)
   {
      png_read_png(png_ptr, info_ptr, (int)transforms, NULL);
      dump_info(png_ptr, info_ptr, tag);
      {
         /* NOTE: png_read_png() allocates the rows with png_malloc(), so bytes
          * that the transform does not write keep uninitialised values; hashing
          * them would not be reproducible even between two runs of the same
          * build.  Only the shape is compared here.
          */
         png_bytepp rp = png_get_rows(png_ptr, info_ptr);
         height = png_get_image_height(png_ptr, info_ptr);
         rowbytes = png_get_rowbytes(png_ptr, info_ptr);
         printf("  %s high-level rows=%p-ish height=%u rowbytes=%lu\n", tag,
                (void*)(rp != NULL ? (void*)1 : (void*)0), height,
                (unsigned long)rowbytes);
         (void)h;
      }
      png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
      return 1;
   }

   png_read_info(png_ptr, info_ptr);
   dump_info(png_ptr, info_ptr, tag);

   if ((transforms & PNG_TRANSFORM_EXPAND) != 0) png_set_expand(png_ptr);
   if ((transforms & PNG_TRANSFORM_STRIP_16) != 0) png_set_strip_16(png_ptr);
   if ((transforms & PNG_TRANSFORM_SCALE_16) != 0) png_set_scale_16(png_ptr);
   if ((transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0) png_set_gray_to_rgb(png_ptr);
   if ((transforms & PNG_TRANSFORM_BGR) != 0) png_set_bgr(png_ptr);
   if ((transforms & PNG_TRANSFORM_PACKING) != 0) png_set_packing(png_ptr);
   if ((transforms & PNG_TRANSFORM_PACKSWAP) != 0) png_set_packswap(png_ptr);
   if ((transforms & PNG_TRANSFORM_INVERT_MONO) != 0) png_set_invert_mono(png_ptr);
   if ((transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0) png_set_swap(png_ptr);
   if ((transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0) png_set_invert_alpha(png_ptr);
   if ((transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0) png_set_swap_alpha(png_ptr);
   if ((transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0) png_set_strip_alpha(png_ptr);
   if ((transforms & PNG_TRANSFORM_EXPAND_16) != 0) png_set_expand_16(png_ptr);
   if ((transforms & 0x10000) != 0) png_set_gamma(png_ptr, 2.2, 0.45455);
   if ((transforms & 0x20000) != 0)
      png_set_rgb_to_gray(png_ptr, PNG_ERROR_ACTION_WARN, -1.0, -1.0);
   if ((transforms & 0x40000) != 0)
      png_set_filler(png_ptr, 0x9a9a, PNG_FILLER_AFTER);
   if ((transforms & 0x80000) != 0)
   {
      png_color_16 bg;
      memset(&bg, 0, sizeof bg);
      bg.red = 40000; bg.green = 20000; bg.blue = 10000; bg.gray = 30000;
      png_set_background(png_ptr, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 1.0);
   }
   if ((transforms & 0x100000) != 0)
   {
      png_color pal[16];
      int i;
      for (i = 0; i < 16; ++i)
      {
         pal[i].red = (png_byte)(i * 16);
         pal[i].green = (png_byte)(255 - i * 16);
         pal[i].blue = (png_byte)(i * 8);
      }
      png_set_quantize(png_ptr, pal, 16, 16, NULL, 1);
   }
   if ((transforms & 0x200000) != 0) png_set_alpha_mode(png_ptr, PNG_ALPHA_STANDARD, 2.2);
   if ((transforms & 0x400000) != 0)
   {
      png_color_8 sh;
      memset(&sh, 0, sizeof sh);
      sh.red = sh.green = sh.blue = sh.gray = 4; sh.alpha = 4;
      png_set_shift(png_ptr, &sh);
   }
   if ((transforms & 0x800000) != 0) png_set_interlace_handling(png_ptr);
   if ((transforms & 0x1000000) != 0)
      png_set_alpha_mode(png_ptr, PNG_ALPHA_OPTIMIZED, 2.2);
   if ((transforms & 0x2000000) != 0)
      png_set_alpha_mode(png_ptr, PNG_ALPHA_BROKEN, 1.8);

   if (png_get_interlace_type(png_ptr, info_ptr) != PNG_INTERLACE_NONE)
      png_set_interlace_handling(png_ptr);

   png_read_update_info(png_ptr, info_ptr);
   dump_info(png_ptr, info_ptr, "after-update");

   height = png_get_image_height(png_ptr, info_ptr);
   rowbytes = png_get_rowbytes(png_ptr, info_ptr);
   rows = malloc(height * sizeof(unsigned char*));
   for (y = 0; y < height; ++y)
   {
      rows[y] = malloc(rowbytes + 8);
      memset(rows[y], 0, rowbytes + 8);
   }

   png_read_image(png_ptr, rows);

   for (y = 0; y < height; ++y) h ^= fnv(rows[y], rowbytes) * (y + 1);
   printf("  %s rows hash=%016lx rowbytes=%lu height=%u\n", tag, h,
          (unsigned long)rowbytes, height);

   png_read_end(png_ptr, end_info);
   dump_info2(png_ptr, end_info, "end", 1);

   png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
   for (y = 0; y < height; ++y) free(rows[y]);
   free(rows);
   return 1;
}

/* --------------------------------------------------------- progressive read */
typedef struct { unsigned long h; unsigned rows; } prog_state;

static void prog_info(png_structp png_ptr, png_infop info_ptr)
{
   printf("  progressive info: %ux%u ct=%d bd=%d\n",
          (unsigned)png_get_image_width(png_ptr, info_ptr),
          (unsigned)png_get_image_height(png_ptr, info_ptr),
          png_get_color_type(png_ptr, info_ptr),
          png_get_bit_depth(png_ptr, info_ptr));
   png_start_read_image(png_ptr);
}

static void prog_row(png_structp png_ptr, png_bytep new_row, png_uint_32 row_num, int pass)
{
   prog_state *st = (prog_state*)png_get_progressive_ptr(png_ptr);
   if (new_row != NULL)
   {
      st->h ^= fnv(new_row, 16) * (row_num + 1) * (unsigned)(pass + 1);
      st->rows++;
   }
}

static void prog_end(png_structp png_ptr, png_infop info_ptr)
{
   (void)info_ptr;
   printf("  progressive end (current row %u pass %d)\n",
          (unsigned)png_get_current_row_number(png_ptr),
          png_get_current_pass_number(png_ptr));
}

static void progressive_read(const unsigned char *data, size_t size, size_t chunk)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   prog_state st;
   size_t pos = 0;

   st.h = 0; st.rows = 0;

   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   info_ptr = png_create_info_struct(png_ptr);
   if (setjmp(jb))
   {
      printf("  progressive aborted\n");
      png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
      return;
   }
   png_set_progressive_read_fn(png_ptr, &st, prog_info, prog_row, prog_end);
   while (pos < size)
   {
      size_t n = size - pos < chunk ? size - pos : chunk;
      png_process_data(png_ptr, info_ptr, (png_bytep)(data + pos), n);
      pos += n;
   }
   printf("  progressive rows=%u hash=%016lx\n", st.rows, st.h);
   png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
}

/* ----------------------------------------------------------- simplified API */
static void simplified(unsigned width, unsigned height, unsigned format)
{
   png_image image;
   unsigned char *buf;
   size_t bufsize;
   unsigned char *pngmem;
   size_t pngsize;

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = width;
   image.height = height;
   image.format = format;
   image.flags = 0;
   image.colormap_entries = 0;

   bufsize = PNG_IMAGE_SIZE(image);
   buf = malloc(bufsize);
   fill_row(buf, bufsize, 7);

   pngsize = PNG_IMAGE_PNG_SIZE_MAX(image);
   pngmem = malloc(pngsize);

   if (png_image_write_to_memory(&image, pngmem, &pngsize, 0, buf,
                                 (png_int_32)PNG_IMAGE_ROW_STRIDE(image), NULL) != 0)
   {
      printf("  simplified write %lu bytes hash=%016lx\n", (unsigned long)pngsize,
             fnv(pngmem, pngsize));

      {
         png_image ri;
         memset(&ri, 0, sizeof ri);
         ri.version = PNG_IMAGE_VERSION;
         if (png_image_begin_read_from_memory(&ri, pngmem, pngsize) != 0)
         {
            unsigned char *rbuf;
            printf("  simplified read %ux%u fmt=%u flags=%u cmap=%u\n",
                   (unsigned)ri.width, (unsigned)ri.height, (unsigned)ri.format,
                   (unsigned)ri.flags, (unsigned)ri.colormap_entries);
            ri.format = format;
            rbuf = malloc(PNG_IMAGE_SIZE(ri));
            if (png_image_finish_read(&ri, NULL, rbuf, 0, NULL) != 0)
               printf("  simplified read hash=%016lx\n", fnv(rbuf, PNG_IMAGE_SIZE(ri)));
            else
               printf("  simplified read failed: %s\n", ri.message);
            free(rbuf);
         }
         else printf("  simplified begin_read failed: %s\n", ri.message);
         png_image_free(&ri);
      }
   }
   else printf("  simplified write failed: %s\n", image.message);

   free(buf);
   free(pngmem);
}

/* ------------------------------------------------------------------ utility */
static void utility_tests(void)
{
   png_byte buf[8];
   png_time t;
   char ascii[64];
   png_structp png_ptr;
   jmp_buf jb;

   printf("== utility ==\n");
   printf("  version=%u '%s' '%s' '%s'\n", (unsigned)png_access_version_number(),
          png_get_libpng_ver(NULL), png_get_header_ver(NULL), png_get_header_version(NULL));
   printf("  copyright='%s'\n", png_get_copyright(NULL));

   png_save_uint_32(buf, 0x12345678);
   png_save_uint_16(buf + 4, 0xabcd);
   png_save_int_32(buf, -305419896);
   printf("  save/get %02x%02x%02x%02x %02x%02x u32=%u u16=%u i32=%ld u31=%u\n",
          buf[0], buf[1], buf[2], buf[3], buf[4], buf[5],
          (unsigned)png_get_uint_32(buf), png_get_uint_16(buf + 4),
          (long)png_get_int_32(buf), (unsigned)0);

   {
      png_byte sig[8] = { 137, 80, 78, 71, 13, 10, 26, 10 };
      printf("  sig_cmp %d %d %d\n", png_sig_cmp(sig, 0, 8), png_sig_cmp(sig, 1, 4),
             png_sig_cmp((png_bytep)"notapng!", 0, 8));
   }

   memset(&t, 0, sizeof t);
   t.year = 1999; t.month = 12; t.day = 31; t.hour = 23; t.minute = 59; t.second = 60;
   {
      char b2[29];
      if (png_convert_to_rfc1123_buffer(b2, &t) != 0) printf("  rfc1123 '%s'\n", b2);
   }
   {
      time_t tt = 1234567890;
      png_time pt;
      png_convert_from_time_t(&pt, tt);
      printf("  from_time_t %u-%u-%u %u:%u:%u\n", pt.year, pt.month, pt.day, pt.hour,
             pt.minute, pt.second);
   }

   png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   if (png_ptr != NULL && setjmp(jb) == 0)
   {
      png_fixed_point res = 0;
      double vals[8] = { 0.0, 1.0, -1.0, 0.5, 3.14159265358979, 1e-7, 12345.6789, 1.0/3.0 };
      int i;
      for (i = 0; i < 8; ++i)
      {
         png_ascii_from_fp(png_ptr, ascii, sizeof ascii, vals[i], 5);
         printf("  ascii_from_fp %d '%s'\n", i, ascii);
      }
      for (i = 0; i < 8; ++i)
      {
         png_ascii_from_fixed(png_ptr, ascii, sizeof ascii,
                              (png_fixed_point)(vals[i] * 100000.0));
         printf("  ascii_from_fixed %d '%s'\n", i, ascii);
      }
      printf("  muldiv %d res=%ld\n", png_muldiv(&res, 100000, 45455, 100000), (long)res);
      printf("  reciprocal %ld %ld\n", (long)png_reciprocal(45455),
             (long)png_reciprocal2(45455, 220000));
      printf("  gamma_significant %d %d\n", png_gamma_significant(100000),
             png_gamma_significant(45455));
      printf("  gamma_8bit %d gamma_16bit %d\n", png_gamma_8bit_correct(128, 45455),
             png_gamma_16bit_correct(32768, 45455));
      printf("  fixed %ld ITU %u\n", (long)png_fixed(png_ptr, 1.23456, "t"),
             (unsigned)png_fixed_ITU(png_ptr, 0.65432, "t"));
      {
         int state = 0;
         size_t whereami = 0;
         printf("  fp_number %d state=%d at=%lu  fp_string %d\n",
                png_check_fp_number("12.5e3", 6, &state, &whereami), state,
                (unsigned long)whereami, png_check_fp_string("-0.125", 6));
      }
      {
         png_byte pal8[3 * 256];
         png_color grey[256];
         png_build_grayscale_palette(8, grey);
         memcpy(pal8, grey, sizeof grey > sizeof pal8 ? sizeof pal8 : sizeof grey);
         printf("  grayscale_palette hash=%016lx\n", fnv((unsigned char*)grey, sizeof grey));
      }
      {
         h_png_xy xy;
         h_png_XYZ XYZ;
         xy.whitex = 31270; xy.whitey = 32900;
         xy.redx = 64000; xy.redy = 33000;
         xy.greenx = 30000; xy.greeny = 60000;
         xy.bluex = 15000; xy.bluey = 6000;
         if (png_XYZ_from_xy(&XYZ, &xy) == 0)
         {
            h_png_xy back;
            printf("  XYZ %ld %ld %ld %ld %ld %ld %ld %ld %ld\n",
                   (long)XYZ.red_X, (long)XYZ.red_Y, (long)XYZ.red_Z,
                   (long)XYZ.green_X, (long)XYZ.green_Y, (long)XYZ.green_Z,
                   (long)XYZ.blue_X, (long)XYZ.blue_Y, (long)XYZ.blue_Z);
            if (png_xy_from_XYZ(&back, &XYZ) == 0)
               printf("  xy %ld %ld %ld %ld %ld %ld %ld %ld\n", (long)back.whitex,
                      (long)back.whitey, (long)back.redx, (long)back.redy,
                      (long)back.greenx, (long)back.greeny, (long)back.bluex,
                      (long)back.bluey);
         }
      }
      printf("  option %d %d\n", png_set_option(png_ptr, PNG_MAXIMUM_INFLATE_WINDOW, PNG_OPTION_ON),
             png_set_option(png_ptr, PNG_SKIP_sRGB_CHECK_PROFILE, PNG_OPTION_OFF));
      printf("  safecat %lu\n", (unsigned long)png_safecat(ascii, sizeof ascii, 3, "hello"));
      {
         char nb[24];
         printf("  format_number '%s'\n",
                png_format_number(nb, nb + sizeof nb, PNG_NUMBER_FORMAT_fixed, 1234567));
      }
      png_destroy_write_struct(&png_ptr, NULL);
   }
}

static void error_tests(const unsigned char *good, size_t goodsize)
{
   unsigned char *bad;
   printf("== error paths ==\n");

   /* truncated file */
   read_png(good, goodsize / 2, 0, 0, "truncated");

   /* corrupt CRC */
   bad = malloc(goodsize);
   memcpy(bad, good, goodsize);
   bad[goodsize / 2] ^= 0xff;
   read_png(bad, goodsize, 0, 0, "corrupt");

   /* bad signature */
   memcpy(bad, good, goodsize);
   bad[1] = 'X';
   read_png(bad, goodsize, 0, 0, "badsig");

   /* zero length */
   read_png(bad, 0, 0, 0, "empty");
   free(bad);
}


/* ------------------------------------------------- user callbacks & extras */
static void user_read_transform(png_structp png_ptr, png_row_infop row_info, png_bytep data)
{
   unsigned long h = fnv(data, row_info->rowbytes);
   printf("    user_read_transform w=%u rb=%lu ct=%d bd=%d ch=%d pd=%d hash=%016lx\n",
          (unsigned)row_info->width, (unsigned long)row_info->rowbytes,
          row_info->color_type, row_info->bit_depth, row_info->channels,
          row_info->pixel_depth, h);
   if (row_info->rowbytes > 0) data[0] = (png_byte)(data[0] ^ 0x55);
   (void)png_get_user_transform_ptr(png_ptr);
}

static void user_write_transform(png_structp png_ptr, png_row_infop row_info, png_bytep data)
{
   printf("    user_write_transform rb=%lu hash=%016lx\n", (unsigned long)row_info->rowbytes,
          fnv(data, row_info->rowbytes));
   (void)png_ptr;
}

static int user_chunk_cb(png_structp png_ptr, png_unknown_chunkp chunk)
{
   printf("    user_chunk '%s' size=%lu hash=%016lx ptr=%d\n", (char*)chunk->name,
          (unsigned long)chunk->size, fnv(chunk->data, chunk->size),
          png_get_user_chunk_ptr(png_ptr) != NULL);
   return 0;
}

static void row_status(png_structp png_ptr, png_uint_32 row, int pass)
{
   (void)png_ptr;
   if ((row % 7) == 0) printf("    row_status %u %d\n", (unsigned)row, pass);
}

static void extra_tests(const unsigned char *good, size_t goodsize)
{
   png_structp png_ptr;
   png_infop info_ptr;
   jmp_buf jb;
   rdbuf rb;

   printf("== extras: user callbacks on read ==\n");
   rb.data = good; rb.size = goodsize; rb.pos = 0;
   png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
   info_ptr = png_create_info_struct(png_ptr);
   if (setjmp(jb) == 0)
   {
      unsigned y, height;
      size_t rowbytes;
      unsigned char *row;

      png_set_read_fn(png_ptr, &rb, mem_read);
      png_set_read_user_chunk_fn(png_ptr, (png_voidp)&rb, user_chunk_cb);
      png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_ALWAYS, NULL, 0);
      png_set_read_status_fn(png_ptr, row_status);
      png_set_benign_errors(png_ptr, 1);
      png_permit_mng_features(png_ptr, PNG_ALL_MNG_FEATURES);
      printf("  handle_as_unknown gAMA=%d prVt=%d\n",
             png_handle_as_unknown(png_ptr, (png_const_bytep)"gAMA"),
             png_handle_as_unknown(png_ptr, (png_const_bytep)"prVt"));
      png_read_info(png_ptr, info_ptr);
      png_set_expand(png_ptr);
      png_set_user_transform_info(png_ptr, (png_voidp)&rb, 8, 4);
      png_set_read_user_transform_fn(png_ptr, user_read_transform);
      png_read_update_info(png_ptr, info_ptr);
      printf("  user transform rowbytes=%lu\n",
             (unsigned long)png_get_rowbytes(png_ptr, info_ptr));
      height = png_get_image_height(png_ptr, info_ptr);
      rowbytes = png_get_rowbytes(png_ptr, info_ptr);
      row = malloc(rowbytes + 8);
      for (y = 0; y < height; ++y)
      {
         memset(row, 0, rowbytes + 8);
         png_read_row(png_ptr, row, NULL);
      }
      png_read_end(png_ptr, NULL);
      free(row);
      png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
   }
   else
   {
      printf("  extras read aborted\n");
      png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
   }

   printf("== extras: write with flush, user transform, high level ==\n");
   {
      membuf out;
      unsigned char **rows;
      unsigned y;
      size_t rowbytes;

      out.data = NULL; out.size = out.cap = 0;
      png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
      info_ptr = png_create_info_struct(png_ptr);
      rows = NULL;
      if (setjmp(jb) == 0)
      {
         png_set_write_fn(png_ptr, &out, mem_write, mem_flush);
         png_set_flush(png_ptr, 3);
         png_set_write_status_fn(png_ptr, row_status);
         png_set_write_user_transform_fn(png_ptr, user_write_transform);
         png_set_user_transform_info(png_ptr, NULL, 8, 3);
         png_set_filter(png_ptr, 0, PNG_ALL_FILTERS);
         {
            double weights[3] = { 1.5, 1.25, 1.0 };
            double costs[PNG_FILTER_VALUE_LAST] = { 1.0, 1.0, 1.0, 1.0, 1.0 };
            png_set_filter_heuristics(png_ptr, PNG_FILTER_HEURISTIC_WEIGHTED, 3,
                                      weights, costs);
         }
         png_set_IHDR(png_ptr, info_ptr, 12, 9, 8, PNG_COLOR_TYPE_RGB,
                      PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_DEFAULT,
                      PNG_FILTER_TYPE_DEFAULT);
         png_set_text_compression_level(png_ptr, 3);
         png_set_text_compression_strategy(png_ptr, 0);
         png_set_text_compression_window_bits(png_ptr, 12);
         png_set_text_compression_mem_level(png_ptr, 6);
         png_set_text_compression_method(png_ptr, 8);
         png_write_info(png_ptr, info_ptr);
         rowbytes = png_get_rowbytes(png_ptr, info_ptr);
         rows = malloc(9 * sizeof(unsigned char*));
         for (y = 0; y < 9; ++y)
         {
            rows[y] = malloc(rowbytes);
            fill_row(rows[y], rowbytes, y + 100);
         }
         for (y = 0; y < 9; ++y) png_write_row(png_ptr, rows[y]);
         png_write_flush(png_ptr);
         png_write_end(png_ptr, info_ptr);
         printf("  flushed write %lu bytes hash=%016lx\n", (unsigned long)out.size,
                fnv(out.data, out.size));
         png_destroy_write_struct(&png_ptr, &info_ptr);
         for (y = 0; y < 9; ++y) free(rows[y]);
         free(rows);
      }
      else
      {
         printf("  extras write aborted\n");
         png_destroy_write_struct(&png_ptr, &info_ptr);
      }
      free(out.data);
   }

   printf("== extras: high level write ==\n");
   {
      membuf out;
      unsigned char **rows;
      unsigned y;
      size_t rowbytes;

      out.data = NULL; out.size = out.cap = 0;
      png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
      info_ptr = png_create_info_struct(png_ptr);
      if (setjmp(jb) == 0)
      {
         png_set_write_fn(png_ptr, &out, mem_write, mem_flush);
         png_set_IHDR(png_ptr, info_ptr, 14, 6, 8, PNG_COLOR_TYPE_RGB_ALPHA,
                      PNG_INTERLACE_ADAM7, PNG_COMPRESSION_TYPE_DEFAULT,
                      PNG_FILTER_TYPE_DEFAULT);
         rowbytes = 14 * 4;
         rows = malloc(6 * sizeof(unsigned char*));
         for (y = 0; y < 6; ++y)
         {
            rows[y] = malloc(rowbytes);
            fill_row(rows[y], rowbytes, y + 200);
         }
         png_set_rows(png_ptr, info_ptr, rows);
         png_write_png(png_ptr, info_ptr,
                       PNG_TRANSFORM_BGR | PNG_TRANSFORM_SWAP_ALPHA, NULL);
         printf("  high level write %lu bytes hash=%016lx\n", (unsigned long)out.size,
                fnv(out.data, out.size));
         png_destroy_write_struct(&png_ptr, &info_ptr);
         for (y = 0; y < 6; ++y) free(rows[y]);
         free(rows);
      }
      else
      {
         printf("  high level write aborted\n");
         png_destroy_write_struct(&png_ptr, &info_ptr);
      }
      free(out.data);
   }

   printf("== extras: crc actions on corrupt data ==\n");
   {
      int crit, anc;
      for (crit = PNG_CRC_DEFAULT; crit <= PNG_CRC_NO_CHANGE; ++crit)
      {
         for (anc = PNG_CRC_DEFAULT; anc <= PNG_CRC_NO_CHANGE; ++anc)
         {
            unsigned char *bad = malloc(goodsize);
            rdbuf rb2;
            memcpy(bad, good, goodsize);
            bad[goodsize - 20] ^= 0x5a;
            rb2.data = bad; rb2.size = goodsize; rb2.pos = 0;
            printf("-- crc crit=%d anc=%d\n", crit, anc);
            png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING, &jb, err_fn, warn_fn);
            info_ptr = png_create_info_struct(png_ptr);
            if (setjmp(jb) == 0)
            {
               png_set_read_fn(png_ptr, &rb2, mem_read);
               png_set_crc_action(png_ptr, crit, anc);
               png_read_info(png_ptr, info_ptr);
               printf("  ok %ux%u\n", (unsigned)png_get_image_width(png_ptr, info_ptr),
                      (unsigned)png_get_image_height(png_ptr, info_ptr));
               png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
            }
            else
            {
               printf("  aborted\n");
               png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
            }
            free(bad);
         }
      }
   }
}

int main(void)
{
   membuf out;
   static const struct { int ct, bd, il; } cases[] = {
      { PNG_COLOR_TYPE_GRAY, 1, 0 },
      { PNG_COLOR_TYPE_GRAY, 2, 0 },
      { PNG_COLOR_TYPE_GRAY, 4, 0 },
      { PNG_COLOR_TYPE_GRAY, 8, 0 },
      { PNG_COLOR_TYPE_GRAY, 16, 0 },
      { PNG_COLOR_TYPE_GRAY_ALPHA, 8, 0 },
      { PNG_COLOR_TYPE_GRAY_ALPHA, 16, 1 },
      { PNG_COLOR_TYPE_PALETTE, 1, 0 },
      { PNG_COLOR_TYPE_PALETTE, 2, 1 },
      { PNG_COLOR_TYPE_PALETTE, 4, 0 },
      { PNG_COLOR_TYPE_PALETTE, 8, 0 },
      { PNG_COLOR_TYPE_RGB, 8, 0 },
      { PNG_COLOR_TYPE_RGB, 16, 1 },
      { PNG_COLOR_TYPE_RGB_ALPHA, 8, 1 },
      { PNG_COLOR_TYPE_RGB_ALPHA, 16, 0 }
   };
   unsigned i;
   membuf keep;
   keep.data = NULL; keep.size = keep.cap = 0;

   utility_tests();

   for (i = 0; i < sizeof cases / sizeof cases[0]; ++i)
   {
      int filters[3] = { PNG_ALL_FILTERS, PNG_FILTER_NONE, PNG_FILTER_PAETH };
      unsigned f;
      for (f = 0; f < 3; ++f)
      {
         printf("== write ct=%d bd=%d il=%d filters=%d ==\n", cases[i].ct, cases[i].bd,
                cases[i].il, filters[f]);
         if (write_png(&out, cases[i].ct, cases[i].bd, cases[i].il, filters[f],
                       f == 1 ? 9 : 6, f == 2 ? Z_RLE : Z_DEFAULT_STRATEGY,
                       f == 0 ? 1 : 0, 23, 17) != 0)
         {
            unsigned t;
            static const unsigned tsets[] = {
               0,
               PNG_TRANSFORM_EXPAND,
               PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_STRIP_16,
               PNG_TRANSFORM_GRAY_TO_RGB | PNG_TRANSFORM_BGR,
               PNG_TRANSFORM_PACKING | PNG_TRANSFORM_PACKSWAP,
               PNG_TRANSFORM_INVERT_MONO | PNG_TRANSFORM_SWAP_ENDIAN,
               PNG_TRANSFORM_STRIP_ALPHA | PNG_TRANSFORM_INVERT_ALPHA,
               PNG_TRANSFORM_EXPAND_16 | PNG_TRANSFORM_SWAP_ALPHA,
               PNG_TRANSFORM_SCALE_16,
               0x10000, 0x20000, 0x40000, 0x80000, 0x100000, 0x200000, 0x400000,
               0x800000, 0x1000000, 0x2000000
            };
            for (t = 0; t < sizeof tsets / sizeof tsets[0]; ++t)
            {
               printf("-- read transforms=%x\n", tsets[t]);
               read_png(out.data, out.size, tsets[t], 0, "r");
            }
            printf("-- high level read\n");
            read_png(out.data, out.size, PNG_TRANSFORM_IDENTITY, 1, "hl");
            read_png(out.data, out.size,
                     PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_BGR | PNG_TRANSFORM_PACKING, 1,
                     "hl2");
            printf("-- progressive\n");
            progressive_read(out.data, out.size, 1);
            progressive_read(out.data, out.size, 37);
            progressive_read(out.data, out.size, out.size);

            if (i == 11 && f == 0)
            {
               free(keep.data);
               keep.data = malloc(out.size);
               memcpy(keep.data, out.data, out.size);
               keep.size = out.size;
            }
         }
         free(out.data);
      }
   }

   printf("== simplified ==\n");
   simplified(19, 13, PNG_FORMAT_GRAY);
   simplified(19, 13, PNG_FORMAT_GA);
   simplified(19, 13, PNG_FORMAT_AG);
   simplified(19, 13, PNG_FORMAT_RGB);
   simplified(19, 13, PNG_FORMAT_BGR);
   simplified(19, 13, PNG_FORMAT_RGBA);
   simplified(19, 13, PNG_FORMAT_ARGB);
   simplified(19, 13, PNG_FORMAT_BGRA);
   simplified(19, 13, PNG_FORMAT_ABGR);
   simplified(19, 13, PNG_FORMAT_LINEAR_Y);
   simplified(19, 13, PNG_FORMAT_LINEAR_RGB);
   simplified(19, 13, PNG_FORMAT_LINEAR_RGB_ALPHA);

   if (keep.data != NULL) extra_tests(keep.data, keep.size);
   if (keep.data != NULL) error_tests(keep.data, keep.size);
   free(keep.data);

   printf("== done ==\n");
   return 0;
}
