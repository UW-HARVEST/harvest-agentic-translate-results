/* Differential test harness: exercises a wide slice of the libpng API and
 * dumps every byte it produces to stdout so that the C and Rust builds can be
 * compared byte for byte.
 *
 * Build:  cc -I../../c_src/include harness.c -L<dir> -lpng -lz -lm -o harness
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <math.h>
#include "png.h"

/* Private (but exported) libpng entry points that png.h does not declare. */
typedef struct { png_fixed_point redx, redy, greenx, greeny, bluex, bluey,
    whitex, whitey; } t_png_xy;
typedef struct { png_fixed_point red_X, red_Y, red_Z, green_X, green_Y,
    green_Z, blue_X, blue_Y, blue_Z; } t_png_XYZ;
#define png_xy t_png_xy
#define png_XYZ t_png_XYZ
extern int png_muldiv(png_fixed_point *res, png_fixed_point a, png_int_32 m,
    png_int_32 d);
extern png_fixed_point png_reciprocal(png_fixed_point a);
extern png_fixed_point png_reciprocal2(png_fixed_point a, png_fixed_point b);
extern int png_gamma_significant(png_fixed_point g);
extern png_byte png_gamma_8bit_correct(unsigned int v, png_fixed_point g);
extern png_uint_16 png_gamma_16bit_correct(unsigned int v, png_fixed_point g);
extern void png_ascii_from_fixed(png_const_structrp pp, png_charp ascii,
    size_t size, png_fixed_point fp);
extern void png_ascii_from_fp(png_const_structrp pp, png_charp ascii,
    size_t size, double fp, unsigned int precision);
extern int png_check_fp_number(png_const_charp s, size_t size, int *statep,
    size_t *whereami);
extern int png_check_fp_string(png_const_charp s, size_t size);
extern int png_XYZ_from_xy(png_XYZ *XYZ, const png_xy *xy);
extern int png_xy_from_XYZ(png_xy *xy, const png_XYZ *XYZ);
extern size_t png_safecat(png_charp buffer, size_t bufsize, size_t pos,
    png_const_charp string);
extern void png_do_bgr(png_row_infop row_info, png_bytep row);
extern void png_do_invert(png_row_infop row_info, png_bytep row);
extern void png_do_swap(png_row_infop row_info, png_bytep row);
extern void png_do_packswap(png_row_infop row_info, png_bytep row);
extern void png_do_strip_channel(png_row_infop row_info, png_bytep row,
    int at_start);

static unsigned long g_hash;
static void feed(const void *p, size_t n)
{
   const unsigned char *b = p;
   size_t i;
   for (i = 0; i < n; ++i)
      g_hash = g_hash * 1000003u + b[i];
}

#define OUT(...) printf(__VA_ARGS__)

static void dump(const char *tag, const unsigned char *b, size_t n)
{
   size_t i;
   OUT("%s len=%zu\n", tag, n);
   for (i = 0; i < n; ++i)
   {
      OUT("%02x", b[i]);
      if ((i & 31) == 31) OUT("\n");
   }
   if (n & 31) OUT("\n");
}

/* ------------------------------------------------------------------ */
/* memory writer for the low-level API                                */
typedef struct { unsigned char *buf; size_t len, cap; } membuf;

static void mem_write(png_structp pp, png_bytep data, size_t len)
{
   membuf *m = png_get_io_ptr(pp);
   if (m->len + len > m->cap)
   {
      m->cap = (m->len + len) * 2 + 1024;
      m->buf = realloc(m->buf, m->cap);
   }
   memcpy(m->buf + m->len, data, len);
   m->len += len;
}
static void mem_flush(png_structp pp) { (void)pp; }

typedef struct { const unsigned char *buf; size_t len, pos; } rdbuf;
static void mem_read(png_structp pp, png_bytep data, size_t len)
{
   rdbuf *r = png_get_io_ptr(pp);
   if (r->pos + len > r->len) png_error(pp, "read past end");
   memcpy(data, r->buf + r->pos, len);
   r->pos += len;
}

static void err_fn(png_structp pp, png_const_charp msg)
{
   OUT("ERROR: %s\n", msg);
   longjmp(png_jmpbuf(pp), 1);
}
static void warn_fn(png_structp pp, png_const_charp msg)
{
   (void)pp;
   OUT("WARN: %s\n", msg);
}

/* ------------------------------------------------------------------ */
static void fill_row(unsigned char *row, size_t nbytes, unsigned seed)
{
   size_t i;
   unsigned x = seed * 2654435761u + 1;
   for (i = 0; i < nbytes; ++i)
   {
      x = x * 1103515245u + 12345u;
      row[i] = (unsigned char)((x >> 16) ^ (i * 7));
   }
}

/* Write an image with the low-level API using an exhaustive set of options. */
static void test_write(int color_type, int bit_depth, int interlace,
    int filters, int level, int strategy, int with_chunks, unsigned seed)
{
   png_structp pp;
   png_infop ip;
   membuf m;
   png_uint_32 w = 23, h = 17;
   png_bytep *rows;
   size_t rowbytes;
   png_uint_32 y;
   int channels;
   png_color pal[256];
   png_byte trans[256];

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   if (pp == NULL) { OUT("no write struct\n"); return; }
   ip = png_create_info_struct(pp);
   if (setjmp(png_jmpbuf(pp)))
   {
      OUT("write longjmp\n");
      png_destroy_write_struct(&pp, &ip);
      free(m.buf);
      return;
   }
   png_set_write_fn(pp, &m, mem_write, mem_flush);

   png_set_IHDR(pp, ip, w, h, bit_depth, color_type, interlace,
       PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
   png_set_filter(pp, 0, filters);
   png_set_compression_level(pp, level);
   png_set_compression_strategy(pp, strategy);
   png_set_compression_window_bits(pp, 15);
   png_set_compression_mem_level(pp, 8);

   if ((color_type & PNG_COLOR_MASK_PALETTE) != 0)
   {
      int i, n = 1 << bit_depth;
      for (i = 0; i < n; ++i)
      {
         pal[i].red   = (png_byte)(i * 3 + 1);
         pal[i].green = (png_byte)(i * 5 + 2);
         pal[i].blue  = (png_byte)(i * 7 + 3);
         trans[i] = (png_byte)(255 - i);
      }
      png_set_PLTE(pp, ip, pal, n);
      if (with_chunks)
         png_set_tRNS(pp, ip, trans, n, NULL);
   }
   else if (with_chunks)
   {
      png_color_16 t;
      memset(&t, 0, sizeof t);
      t.red = t.green = t.blue = t.gray = (png_uint_16)(bit_depth == 16 ? 1000 : 5);
      if ((color_type & PNG_COLOR_MASK_ALPHA) == 0)
         png_set_tRNS(pp, ip, NULL, 0, &t);
   }

   if (with_chunks)
   {
      png_color_16 bg;
      png_color_8 sbit;
      png_time t;
      png_text txt[3];
      png_uint_16 hist[256];
      png_sPLT_t splt;
      png_sPLT_entry sent[4];
      png_unknown_chunk unk[1];
      int i;

      memset(&bg, 0, sizeof bg);
      bg.index = 1; bg.red = 3; bg.green = 4; bg.blue = 5; bg.gray = 6;
      png_set_bKGD(pp, ip, &bg);

      memset(&sbit, 0, sizeof sbit);
      sbit.red = sbit.green = sbit.blue = sbit.gray = (png_byte)bit_depth;
      sbit.alpha = (png_byte)bit_depth;
      png_set_sBIT(pp, ip, &sbit);

      png_set_gAMA_fixed(pp, ip, 45455);
      png_set_cHRM_fixed(pp, ip, 31270, 32900, 64000, 33000, 30000, 60000,
          15000, 6000);
      png_set_sRGB(pp, ip, PNG_sRGB_INTENT_PERCEPTUAL);
      png_set_cICP(pp, ip, 9, 16, 0, 1);
      png_set_cLLI_fixed(pp, ip, 10000000, 4000000);
      png_set_mDCV_fixed(pp, ip, 15635, 16450, 34000, 16000, 13250, 34500,
          7500, 3000, 10000000, 500);
      png_set_pHYs(pp, ip, 3000, 2000, PNG_RESOLUTION_METER);
      png_set_oFFs(pp, ip, -5, 7, PNG_OFFSET_PIXEL);
      png_set_sCAL_s(pp, ip, PNG_SCALE_METER, "1.5", "2.5e-1");
      memset(&t, 0, sizeof t);
      t.year = 2024; t.month = 6; t.day = 15;
      t.hour = 12; t.minute = 34; t.second = 56;
      png_set_tIME(pp, ip, &t);

      memset(txt, 0, sizeof txt);
      txt[0].compression = PNG_TEXT_COMPRESSION_NONE;
      txt[0].key = (png_charp)"Title";
      txt[0].text = (png_charp)"a plain tEXt chunk";
      txt[1].compression = PNG_TEXT_COMPRESSION_zTXt;
      txt[1].key = (png_charp)"Description";
      txt[1].text = (png_charp)
          "a compressed zTXt chunk with enough text to actually compress "
          "a compressed zTXt chunk with enough text to actually compress";
      txt[2].compression = PNG_ITXT_COMPRESSION_zTXt;
      txt[2].key = (png_charp)"Comment";
      txt[2].lang = (png_charp)"en";
      txt[2].lang_key = (png_charp)"Comment";
      txt[2].text = (png_charp)
          "an international text chunk, compressed, repeated repeated repeated";
      png_set_text(pp, ip, txt, 3);

      if ((color_type & PNG_COLOR_MASK_PALETTE) != 0)
      {
         int n = 1 << bit_depth;
         for (i = 0; i < n; ++i) hist[i] = (png_uint_16)(n - i);
         png_set_hIST(pp, ip, hist);
      }

      for (i = 0; i < 4; ++i)
      {
         sent[i].red = (png_uint_16)(i * 1000);
         sent[i].green = (png_uint_16)(i * 1100);
         sent[i].blue = (png_uint_16)(i * 1200);
         sent[i].alpha = (png_uint_16)(i * 1300);
         sent[i].frequency = (png_uint_16)(4 - i);
      }
      splt.name = (png_charp)"spltname";
      splt.depth = 16;
      splt.entries = sent;
      splt.nentries = 4;
      png_set_sPLT(pp, ip, &splt, 1);

      png_set_pCAL(pp, ip, "pcal purpose", -100, 100, PNG_EQUATION_LINEAR,
          2, "units", (png_charpp)(char*[]){ (char*)"1.0", (char*)"2.0" });

      {
         static png_byte exif[] = { 'M','M',0,42,0,0,0,8 };
         png_set_eXIf_1(pp, ip, (png_uint_32)sizeof exif, exif);
      }

      memset(unk, 0, sizeof unk);
      memcpy(unk[0].name, "prVt", 5);
      unk[0].data = (png_bytep)"unknown-chunk-payload";
      unk[0].size = 21;
      unk[0].location = PNG_HAVE_IHDR;
      png_set_unknown_chunks(pp, ip, unk, 1);
      png_set_keep_unknown_chunks(pp, PNG_HANDLE_CHUNK_ALWAYS, NULL, 0);
   }

   png_write_info(pp, ip);

   channels = png_get_channels(pp, ip);
   rowbytes = png_get_rowbytes(pp, ip);
   OUT("write ct=%d bd=%d il=%d ch=%d rb=%zu\n", color_type, bit_depth,
       interlace, channels, rowbytes);

   rows = malloc(h * sizeof(png_bytep));
   for (y = 0; y < h; ++y)
   {
      rows[y] = malloc(rowbytes);
      fill_row(rows[y], rowbytes, seed + y);
   }

   png_write_image(pp, rows);
   png_write_end(pp, ip);

   dump("PNG", m.buf, m.len);
   feed(m.buf, m.len);

   /* -------- now read it back with the low level API -------- */
   {
      png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL,
          err_fn, warn_fn);
      png_infop rip = png_create_info_struct(rp);
      png_infop eip = png_create_info_struct(rp);
      rdbuf rb;
      rb.buf = m.buf; rb.len = m.len; rb.pos = 0;

      if (setjmp(png_jmpbuf(rp)))
      {
         OUT("read longjmp\n");
         png_destroy_read_struct(&rp, &rip, &eip);
      }
      else
      {
         png_uint_32 rw, rh;
         int rbd, rct, ril, rcm, rfm, npass, pass;
         png_bytep *rrows;
         size_t rrb;

         png_set_read_fn(rp, &rb, mem_read);
         png_set_keep_unknown_chunks(rp, PNG_HANDLE_CHUNK_ALWAYS, NULL, 0);
         png_read_info(rp, rip);
         png_get_IHDR(rp, rip, &rw, &rh, &rbd, &rct, &ril, &rcm, &rfm);
         OUT("read IHDR %u %u %d %d %d %d %d\n", rw, rh, rbd, rct, ril, rcm,
             rfm);
         OUT("valid=%08x\n", (unsigned)png_get_valid(rp, rip, 0xffffffffu));

         /* dump ancillary info */
         {
            png_fixed_point g;
            if (png_get_gAMA_fixed(rp, rip, &g)) OUT("gAMA %d\n", g);
         }
         {
            png_fixed_point wx, wy, rx, ry, gx, gy, bx, by;
            if (png_get_cHRM_fixed(rp, rip, &wx, &wy, &rx, &ry, &gx, &gy,
                &bx, &by))
               OUT("cHRM %d %d %d %d %d %d %d %d\n", wx, wy, rx, ry, gx, gy,
                   bx, by);
         }
         {
            int intent;
            if (png_get_sRGB(rp, rip, &intent)) OUT("sRGB %d\n", intent);
         }
         {
            png_byte a, b, c, d;
            if (png_get_cICP(rp, rip, &a, &b, &c, &d))
               OUT("cICP %d %d %d %d\n", a, b, c, d);
         }
         {
            png_uint_32 a, b;
            if (png_get_cLLI_fixed(rp, rip, &a, &b)) OUT("cLLI %u %u\n", a, b);
         }
         {
            png_fixed_point wx, wy, rx, ry, gx, gy, bx, by;
            png_uint_32 mx, mn;
            if (png_get_mDCV_fixed(rp, rip, &wx, &wy, &rx, &ry, &gx, &gy,
                &bx, &by, &mx, &mn))
               OUT("mDCV %d %d %d %d %d %d %d %d %u %u\n", wx, wy, rx, ry,
                   gx, gy, bx, by, mx, mn);
         }
         {
            png_uint_32 rx, ry; int ut;
            if (png_get_pHYs(rp, rip, &rx, &ry, &ut))
               OUT("pHYs %u %u %d\n", rx, ry, ut);
         }
         {
            png_int_32 ox, oy; int ut;
            if (png_get_oFFs(rp, rip, &ox, &oy, &ut))
               OUT("oFFs %d %d %d\n", ox, oy, ut);
         }
         {
            int unit; png_charp sw, sh;
            if (png_get_sCAL_s(rp, rip, &unit, &sw, &sh))
               OUT("sCAL %d %s %s\n", unit, sw, sh);
         }
         {
            png_timep tp;
            if (png_get_tIME(rp, rip, &tp))
               OUT("tIME %u %u %u %u %u %u\n", tp->year, tp->month, tp->day,
                   tp->hour, tp->minute, tp->second);
         }
         {
            png_color_16p bg;
            if (png_get_bKGD(rp, rip, &bg))
               OUT("bKGD %u %u %u %u %u\n", bg->index, bg->red, bg->green,
                   bg->blue, bg->gray);
         }
         {
            png_color_8p sb;
            if (png_get_sBIT(rp, rip, &sb))
               OUT("sBIT %u %u %u %u %u\n", sb->red, sb->green, sb->blue,
                   sb->gray, sb->alpha);
         }
         {
            png_charp purpose, units; png_int_32 X0, X1; int type, nparams;
            png_charpp params;
            if (png_get_pCAL(rp, rip, &purpose, &X0, &X1, &type, &nparams,
                &units, &params))
            {
               int i;
               OUT("pCAL %s %d %d %d %d %s", purpose, X0, X1, type, nparams,
                   units);
               for (i = 0; i < nparams; ++i) OUT(" %s", params[i]);
               OUT("\n");
            }
         }
         {
            png_textp tp; int ntext;
            if (png_get_text(rp, rip, &tp, &ntext) > 0)
            {
               int i;
               for (i = 0; i < ntext; ++i)
                  OUT("text[%d] %d %s | %s | %s | %s\n", i, tp[i].compression,
                      tp[i].key ? tp[i].key : "-",
                      tp[i].text ? tp[i].text : "-",
                      tp[i].lang ? tp[i].lang : "-",
                      tp[i].lang_key ? tp[i].lang_key : "-");
            }
         }
         {
            png_sPLT_tp sp; int n = png_get_sPLT(rp, rip, &sp);
            int i, j;
            for (i = 0; i < n; ++i)
            {
               OUT("sPLT %s d=%d n=%d:", sp[i].name, sp[i].depth,
                   (int)sp[i].nentries);
               for (j = 0; j < sp[i].nentries; ++j)
                  OUT(" %u/%u/%u/%u/%u", sp[i].entries[j].red,
                      sp[i].entries[j].green, sp[i].entries[j].blue,
                      sp[i].entries[j].alpha, sp[i].entries[j].frequency);
               OUT("\n");
            }
         }
         {
            png_uint_16p hist;
            if (png_get_hIST(rp, rip, &hist))
            {
               int i, n; png_colorp p;
               png_get_PLTE(rp, rip, &p, &n);
               OUT("hIST:");
               for (i = 0; i < n; ++i) OUT(" %u", hist[i]);
               OUT("\n");
            }
         }
         {
            png_uint_32 nexif; png_bytep exif;
            if (png_get_eXIf_1(rp, rip, &nexif, &exif))
               dump("eXIf", exif, nexif);
         }
         {
            png_unknown_chunkp uc;
            int n = png_get_unknown_chunks(rp, rip, &uc);
            int i;
            for (i = 0; i < n; ++i)
            {
               OUT("unknown %s loc=%u ", (char*)uc[i].name, uc[i].location);
               dump("data", uc[i].data, uc[i].size);
            }
         }

         npass = 1;
         if (ril == PNG_INTERLACE_ADAM7)
            npass = png_set_interlace_handling(rp);
         png_read_update_info(rp, rip);
         rrb = png_get_rowbytes(rp, rip);
         OUT("read rowbytes=%zu passes=%d\n", rrb, npass);

         rrows = malloc(rh * sizeof(png_bytep));
         for (y = 0; y < rh; ++y)
         {
            rrows[y] = malloc(rrb);
            memset(rrows[y], 0, rrb);
         }
         for (pass = 0; pass < npass; ++pass)
            for (y = 0; y < rh; ++y)
               png_read_row(rp, rrows[y], NULL);

         png_read_end(rp, eip);

         for (y = 0; y < rh; ++y)
         {
            feed(rrows[y], rrb);
            if (y < 3) dump("row", rrows[y], rrb);
            /* compare with what we wrote */
            if (rrb == rowbytes && memcmp(rrows[y], rows[y], rrb) != 0)
               OUT("ROW MISMATCH at %u\n", y);
         }
         for (y = 0; y < rh; ++y) free(rrows[y]);
         free(rrows);
         png_destroy_read_struct(&rp, &rip, &eip);
      }
   }

   for (y = 0; y < h; ++y) free(rows[y]);
   free(rows);
   png_destroy_write_struct(&pp, &ip);
   free(m.buf);
   OUT("hash=%lu\n", g_hash);
}

/* ------------------------------------------------------------------ */
/* Read with transformations                                          */
static void test_read_transforms(const unsigned char *png, size_t len,
    int transforms, const char *tag)
{
   png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL,
       err_fn, warn_fn);
   png_infop ip = png_create_info_struct(rp);
   rdbuf rb;
   rb.buf = png; rb.len = len; rb.pos = 0;

   OUT("--- read_png %s transforms=%x\n", tag, transforms);
   if (setjmp(png_jmpbuf(rp)))
   {
      OUT("read_png longjmp\n");
      png_destroy_read_struct(&rp, &ip, NULL);
      return;
   }
   png_set_read_fn(rp, &rb, mem_read);
   png_read_png(rp, ip, transforms, NULL);
   {
      png_bytepp rows = png_get_rows(rp, ip);
      png_uint_32 h = png_get_image_height(rp, ip);
      size_t rb2 = png_get_rowbytes(rp, ip);
      png_uint_32 y;
      OUT("rows h=%u rb=%zu ct=%d bd=%d\n", h, rb2,
          png_get_color_type(rp, ip), png_get_bit_depth(rp, ip));
      for (y = 0; y < h; ++y)
      {
         feed(rows[y], rb2);
         if (y < 2) dump("trow", rows[y], rb2);
      }
   }
   png_destroy_read_struct(&rp, &ip, NULL);
   OUT("hash=%lu\n", g_hash);
}

/* Manual transform set applied through the low level API. */
static void test_read_manual(const unsigned char *png, size_t len, int which)
{
   png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL,
       err_fn, warn_fn);
   png_infop ip = png_create_info_struct(rp);
   rdbuf rb;
   png_uint_32 y, h;
   size_t rowbytes;
   png_bytep row;

   rb.buf = png; rb.len = len; rb.pos = 0;
   OUT("--- read_manual which=%d\n", which);
   if (setjmp(png_jmpbuf(rp)))
   {
      OUT("read_manual longjmp\n");
      png_destroy_read_struct(&rp, &ip, NULL);
      return;
   }
   png_set_read_fn(rp, &rb, mem_read);
   png_read_info(rp, ip);

   switch (which)
   {
      case 0:
         png_set_expand(rp);
         png_set_gray_to_rgb(rp);
         png_set_gamma_fixed(rp, 220000, 45455);
         break;
      case 1:
      {
         png_color_16 bg;
         memset(&bg, 0, sizeof bg);
         bg.red = 200; bg.green = 100; bg.blue = 50; bg.gray = 128;
         png_set_expand(rp);
         png_set_background_fixed(rp, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0,
             100000);
         png_set_gamma_fixed(rp, 220000, 45455);
         break;
      }
      case 2:
         png_set_expand(rp);
         png_set_rgb_to_gray_fixed(rp, PNG_ERROR_ACTION_NONE, -1, -1);
         png_set_gamma_fixed(rp, 220000, 45455);
         break;
      case 3:
         png_set_expand(rp);
         png_set_strip_16(rp);
         png_set_packing(rp);
         png_set_bgr(rp);
         png_set_swap_alpha(rp);
         png_set_invert_alpha(rp);
         break;
      case 4:
         png_set_expand(rp);
         png_set_scale_16(rp);
         png_set_filler(rp, 0x8000, PNG_FILLER_AFTER);
         break;
      case 5:
         png_set_expand_16(rp);
         png_set_alpha_mode_fixed(rp, PNG_ALPHA_STANDARD, PNG_GAMMA_LINEAR);
         break;
      case 6:
      {
         png_color pal[256];
         png_uint_16 hist[256];
         int i;
         for (i = 0; i < 256; ++i)
         {
            pal[i].red = (png_byte)i;
            pal[i].green = (png_byte)(255 - i);
            pal[i].blue = (png_byte)(i * 3);
            hist[i] = (png_uint_16)(256 - i);
         }
         png_set_expand(rp);
         png_set_quantize(rp, pal, 256, 17, hist, 1);
         break;
      }
      case 7:
      {
         png_color_8 sh;
         memset(&sh, 0, sizeof sh);
         sh.red = sh.green = sh.blue = sh.gray = 4;
         sh.alpha = 4;
         png_set_expand(rp);
         png_set_shift(rp, &sh);
         png_set_invert_mono(rp);
         png_set_packswap(rp);
         break;
      }
      default:
         break;
   }

   if (png_get_interlace_type(rp, ip) == PNG_INTERLACE_ADAM7)
      png_set_interlace_handling(rp);
   png_read_update_info(rp, ip);
   h = png_get_image_height(rp, ip);
   rowbytes = png_get_rowbytes(rp, ip);
   OUT("manual rb=%zu ct=%d bd=%d ch=%d\n", rowbytes,
       png_get_color_type(rp, ip), png_get_bit_depth(rp, ip),
       png_get_channels(rp, ip));
   row = malloc(rowbytes);
   {
      int npass = png_get_interlace_type(rp, ip) == PNG_INTERLACE_ADAM7 ? 7 : 1;
      int pass;
      for (pass = 0; pass < npass; ++pass)
         for (y = 0; y < h; ++y)
         {
            memset(row, 0, rowbytes);
            png_read_row(rp, row, NULL);
            feed(row, rowbytes);
            if (pass == npass - 1 && y < 2) dump("mrow", row, rowbytes);
         }
   }
   free(row);
   png_read_end(rp, NULL);
   png_destroy_read_struct(&rp, &ip, NULL);
   OUT("hash=%lu\n", g_hash);
}

/* ------------------------------------------------------------------ */
/* progressive reader                                                 */
static void prog_info(png_structp pp, png_infop ip)
{
   OUT("prog info %u %u %d %d\n", png_get_image_width(pp, ip),
       png_get_image_height(pp, ip), png_get_bit_depth(pp, ip),
       png_get_color_type(pp, ip));
   if (png_get_interlace_type(pp, ip) == PNG_INTERLACE_ADAM7)
      png_set_interlace_handling(pp);
   png_read_update_info(pp, ip);
}
static void prog_row(png_structp pp, png_bytep newrow, png_uint_32 rownum,
    int pass)
{
   (void)pp;
   if (newrow != NULL)
   {
      OUT("prog row %u %d\n", rownum, pass);
      /* the row length is not directly available; hash a fixed prefix */
      feed(newrow, 8);
   }
}
static void prog_end(png_structp pp, png_infop ip)
{
   (void)pp; (void)ip;
   OUT("prog end\n");
}

static void test_progressive(const unsigned char *png, size_t len, size_t chunk)
{
   png_structp pp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL,
       err_fn, warn_fn);
   png_infop ip = png_create_info_struct(pp);
   size_t pos = 0;

   OUT("--- progressive chunk=%zu\n", chunk);
   if (setjmp(png_jmpbuf(pp)))
   {
      OUT("prog longjmp\n");
      png_destroy_read_struct(&pp, &ip, NULL);
      return;
   }
   png_set_progressive_read_fn(pp, NULL, prog_info, prog_row, prog_end);
   while (pos < len)
   {
      size_t n = len - pos < chunk ? len - pos : chunk;
      png_process_data(pp, ip, (png_bytep)(png + pos), n);
      pos += n;
   }
   png_destroy_read_struct(&pp, &ip, NULL);
   OUT("hash=%lu\n", g_hash);
}

/* ------------------------------------------------------------------ */
/* simplified API                                                     */
static void test_simplified(png_uint_32 fmt, int convert8, unsigned seed)
{
   png_image im;
   size_t nbytes = 0;
   void *mem;
   unsigned char *buf;
   png_uint_32 w = 19, h = 13;
   size_t bufsize;

   memset(&im, 0, sizeof im);
   im.version = PNG_IMAGE_VERSION;
   im.width = w;
   im.height = h;
   im.format = fmt;
   im.flags = 0;
   im.colormap_entries = 0;

   bufsize = PNG_IMAGE_SIZE(im);
   buf = malloc(bufsize);
   fill_row(buf, bufsize, seed);

   OUT("--- simplified fmt=%u c8=%d size=%zu\n", fmt, convert8, bufsize);

   if (png_image_write_to_memory(&im, NULL, &nbytes, convert8, buf, 0, NULL))
   {
      OUT("needed=%zu\n", nbytes);
      mem = malloc(nbytes);
      if (png_image_write_to_memory(&im, mem, &nbytes, convert8, buf, 0, NULL))
      {
         dump("SPNG", mem, nbytes);
         feed(mem, nbytes);

         /* read it back */
         {
            png_image r;
            memset(&r, 0, sizeof r);
            r.version = PNG_IMAGE_VERSION;
            if (png_image_begin_read_from_memory(&r, mem, nbytes))
            {
               unsigned char *rb;
               size_t rs;
               r.format = fmt;
               rs = PNG_IMAGE_SIZE(r);
               rb = malloc(rs);
               memset(rb, 0, rs);
               if (png_image_finish_read(&r, NULL, rb, 0, NULL))
               {
                  dump("SREAD", rb, rs < 96 ? rs : 96);
                  feed(rb, rs);
               }
               else
                  OUT("finish_read failed: %s\n", r.message);
               free(rb);
            }
            else
               OUT("begin_read failed: %s\n", r.message);
            png_image_free(&r);
         }
         /* and as a colour-mapped image */
         {
            png_image r;
            memset(&r, 0, sizeof r);
            r.version = PNG_IMAGE_VERSION;
            if (png_image_begin_read_from_memory(&r, mem, nbytes))
            {
               unsigned char *rb, *cm;
               size_t rs, cs;
               r.format = (fmt & ~PNG_FORMAT_FLAG_LINEAR)
                   | PNG_FORMAT_FLAG_COLORMAP;
               rs = PNG_IMAGE_SIZE(r);
               cs = PNG_IMAGE_COLORMAP_SIZE(r);
               rb = malloc(rs ? rs : 1);
               cm = malloc(cs ? cs : 1);
               memset(rb, 0, rs ? rs : 1);
               memset(cm, 0, cs ? cs : 1);
               if (png_image_finish_read(&r, NULL, rb, 0, cm))
               {
                  OUT("cmap entries=%u\n", r.colormap_entries);
                  dump("CREAD", rb, rs < 64 ? rs : 64);
                  dump("CMAP", cm, cs < 64 ? cs : 64);
                  feed(rb, rs);
                  feed(cm, cs);
               }
               else
                  OUT("cmap finish_read failed: %s\n", r.message);
               free(rb); free(cm);
            }
            png_image_free(&r);
         }
      }
      free(mem);
   }
   else
      OUT("write_to_memory failed: %s\n", im.message);
   free(buf);
   png_image_free(&im);
   OUT("hash=%lu\n", g_hash);
}

/* ------------------------------------------------------------------ */
/* misc utility APIs                                                  */
static void test_misc(void)
{
   png_byte b[8];
   png_time t;
   char buf[29];
   int i;

   OUT("--- misc\n");
   OUT("version=%u\n", (unsigned)png_access_version_number());
   OUT("libpng_ver=%s\n", png_get_libpng_ver(NULL));
   OUT("header_ver=%s\n", png_get_header_ver(NULL));
   OUT("header_version=%s\n", png_get_header_version(NULL));
   OUT("copyright=%s\n", png_get_copyright(NULL));

   for (i = 0; i < 8; ++i) b[i] = (png_byte)(i * 37 + 11);
   OUT("u32=%u u16=%u i32=%d\n", (unsigned)png_get_uint_32(b),
       (unsigned)png_get_uint_16(b), (int)png_get_int_32(b));
   b[0] = 0x80;
   OUT("i32neg=%d\n", (int)png_get_int_32(b));
   png_save_uint_32(b, 0xdeadbeefu);
   dump("save32", b, 4);
   png_save_uint_16(b, 0xbeefu);
   dump("save16", b, 2);
   png_save_int_32(b, -12345678);
   dump("saveint32", b, 4);

   memset(&t, 0, sizeof t);
   t.year = 1999; t.month = 12; t.day = 31;
   t.hour = 23; t.minute = 59; t.second = 60;
   if (png_convert_to_rfc1123_buffer(buf, &t))
      OUT("rfc1123=%s\n", buf);

   {
      png_color pal[256];
      int d;
      for (d = 1; d <= 8; d <<= 1)
      {
         png_build_grayscale_palette(d, pal);
         OUT("graypal %d:", d);
         for (i = 0; i < (1 << d) && i < 16; ++i)
            OUT(" %u/%u/%u", pal[i].red, pal[i].green, pal[i].blue);
         OUT("\n");
      }
   }

   {
      static const png_byte sig[8] = { 137, 80, 78, 71, 13, 10, 26, 10 };
      OUT("sigcmp %d %d %d\n", png_sig_cmp(sig, 0, 8), png_sig_cmp(sig, 1, 7),
          png_sig_cmp((png_const_bytep)"notapng!", 0, 8));
   }

   /* fixed point / gamma helpers */
   {
      png_fixed_point r;
      OUT("muldiv %d %d\n", png_muldiv(&r, 100000, 3, 7), r);
      OUT("recip %d %d\n", png_reciprocal(45455), png_reciprocal2(45455, 220000));
      OUT("gsig %d %d\n", png_gamma_significant(100000),
          png_gamma_significant(45455));
      OUT("g8 %u %u\n", png_gamma_8bit_correct(128, 45455),
          png_gamma_8bit_correct(200, 220000));
      OUT("g16 %u %u\n", png_gamma_16bit_correct(30000, 45455),
          png_gamma_16bit_correct(60000, 220000));
   }

   /* ascii <-> fp */
   {
      char a[64];
      png_ascii_from_fixed(NULL, a, sizeof a, 123456);
      OUT("afx=%s\n", a);
      png_ascii_from_fixed(NULL, a, sizeof a, -1);
      OUT("afx2=%s\n", a);
      png_ascii_from_fp(NULL, a, sizeof a, 3.14159265358979, 5);
      OUT("affp=%s\n", a);
      png_ascii_from_fp(NULL, a, sizeof a, 1e-20, 8);
      OUT("affp2=%s\n", a);
      png_ascii_from_fp(NULL, a, sizeof a, 1e20, 8);
      OUT("affp3=%s\n", a);
      png_ascii_from_fp(NULL, a, sizeof a, 0.0, 5);
      OUT("affp4=%s\n", a);
      png_ascii_from_fp(NULL, a, sizeof a, -12345.6789, 6);
      OUT("affp5=%s\n", a);
   }

   /* fp number parsing */
   {
      static const char *tests[] = {
         "1.0", "-1.5e10", ".5", "1.", "1e", "+3.0E-4", "abc", "0", "-0",
         "1e400", "12345678901234567890"
      };
      size_t i2;
      for (i2 = 0; i2 < sizeof tests / sizeof tests[0]; ++i2)
      {
         int state = 0;
         size_t where = 0;
         int ok = png_check_fp_number(tests[i2], strlen(tests[i2]), &state,
             &where);
         OUT("fp %-22s ok=%d state=%d where=%zu str=%d\n", tests[i2], ok,
             state, where, png_check_fp_string(tests[i2], strlen(tests[i2])));
      }
   }

   /* colorspace conversions */
   {
      png_xy xy;
      png_XYZ XYZ;
      xy.redx = 64000; xy.redy = 33000;
      xy.greenx = 30000; xy.greeny = 60000;
      xy.bluex = 15000; xy.bluey = 6000;
      xy.whitex = 31270; xy.whitey = 32900;
      OUT("XYZ_from_xy=%d\n", png_XYZ_from_xy(&XYZ, &xy));
      OUT("XYZ %d %d %d %d %d %d %d %d %d\n", XYZ.red_X, XYZ.red_Y, XYZ.red_Z,
          XYZ.green_X, XYZ.green_Y, XYZ.green_Z, XYZ.blue_X, XYZ.blue_Y,
          XYZ.blue_Z);
      OUT("xy_from_XYZ=%d\n", png_xy_from_XYZ(&xy, &XYZ));
      OUT("xy %d %d %d %d %d %d %d %d\n", xy.redx, xy.redy, xy.greenx,
          xy.greeny, xy.bluex, xy.bluey, xy.whitex, xy.whitey);
   }

   /* safecat / format_number */
   {
      char sb[16];
      size_t pos = png_safecat(sb, sizeof sb, 0, "hello");
      pos = png_safecat(sb, sizeof sb, pos, " world and more");
      OUT("safecat %zu %s\n", pos, sb);
   }

   /* row transformation primitives */
   {
      png_row_info ri;
      png_byte row[64];
      int i2;
      for (i2 = 0; i2 < 64; ++i2) row[i2] = (png_byte)(i2 * 5 + 1);
      ri.width = 8; ri.rowbytes = 32; ri.color_type = PNG_COLOR_TYPE_RGBA;
      ri.bit_depth = 8; ri.channels = 4; ri.pixel_depth = 32;
      png_do_bgr(&ri, row);
      dump("bgr", row, 32);
      png_do_invert(&ri, row);
      dump("invert", row, 32);
      png_do_swap(&ri, row);
      dump("swap-nop", row, 32);
      ri.bit_depth = 1; ri.channels = 1; ri.pixel_depth = 1;
      ri.color_type = PNG_COLOR_TYPE_GRAY; ri.rowbytes = 8; ri.width = 64;
      png_do_packswap(&ri, row);
      dump("packswap", row, 8);
      ri.bit_depth = 8; ri.channels = 4; ri.pixel_depth = 32;
      ri.color_type = PNG_COLOR_TYPE_RGBA; ri.rowbytes = 32; ri.width = 8;
      png_do_strip_channel(&ri, row, 1);
      dump("strip", row, 24);
   }

   /* option setting */
   for (i = 0; i < 18; i += 2)
      OUT("opt %d -> %d\n", i, png_set_option(NULL, i, 1));

   OUT("hash=%lu\n", g_hash);
}

/* ------------------------------------------------------------------ */
static unsigned char *make_reference_png(size_t *lenp, int ct, int bd, int il)
{
   png_structp pp;
   png_infop ip;
   membuf m;
   png_uint_32 w = 21, h = 15, y;
   png_bytep *rows;
   size_t rowbytes;

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   if (setjmp(png_jmpbuf(pp)))
   {
      png_destroy_write_struct(&pp, &ip);
      *lenp = 0;
      return NULL;
   }
   png_set_write_fn(pp, &m, mem_write, mem_flush);
   png_set_IHDR(pp, ip, w, h, bd, ct, il, PNG_COMPRESSION_TYPE_DEFAULT,
       PNG_FILTER_TYPE_DEFAULT);
   if ((ct & PNG_COLOR_MASK_PALETTE) != 0)
   {
      png_color pal[256];
      png_byte tr[256];
      int i, n = 1 << bd;
      for (i = 0; i < n; ++i)
      {
         pal[i].red = (png_byte)(i * 11);
         pal[i].green = (png_byte)(i * 13);
         pal[i].blue = (png_byte)(i * 17);
         tr[i] = (png_byte)(i * 3);
      }
      png_set_PLTE(pp, ip, pal, n);
      png_set_tRNS(pp, ip, tr, n, NULL);
   }
   else if ((ct & PNG_COLOR_MASK_ALPHA) == 0)
   {
      png_color_16 t;
      memset(&t, 0, sizeof t);
      t.red = t.green = t.blue = t.gray = 3;
      png_set_tRNS(pp, ip, NULL, 0, &t);
   }
   png_set_gAMA_fixed(pp, ip, 50000);
   {
      png_color_16 bg;
      memset(&bg, 0, sizeof bg);
      bg.red = 1; bg.green = 2; bg.blue = 3; bg.gray = 2; bg.index = 1;
      png_set_bKGD(pp, ip, &bg);
   }
   png_write_info(pp, ip);
   rowbytes = png_get_rowbytes(pp, ip);
   rows = malloc(h * sizeof(png_bytep));
   for (y = 0; y < h; ++y)
   {
      rows[y] = malloc(rowbytes);
      fill_row(rows[y], rowbytes, 99 + y * 3);
   }
   png_write_image(pp, rows);
   png_write_end(pp, ip);
   for (y = 0; y < h; ++y) free(rows[y]);
   free(rows);
   png_destroy_write_struct(&pp, &ip);
   *lenp = m.len;
   return m.buf;
}

int main(void)
{
   static const int cts[] = {
      PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_PALETTE, PNG_COLOR_TYPE_RGB,
      PNG_COLOR_TYPE_GRAY_ALPHA, PNG_COLOR_TYPE_RGB_ALPHA
   };
   static const int bds[] = { 1, 2, 4, 8, 16 };
   size_t ci, bi;
   int il, wc;

   g_hash = 5381;

   test_misc();

   for (ci = 0; ci < sizeof cts / sizeof cts[0]; ++ci)
      for (bi = 0; bi < sizeof bds / sizeof bds[0]; ++bi)
      {
         int ct = cts[ci], bd = bds[bi];
         if (ct == PNG_COLOR_TYPE_PALETTE && bd == 16) continue;
         if (ct != PNG_COLOR_TYPE_GRAY && ct != PNG_COLOR_TYPE_PALETTE &&
             bd < 8) continue;
         for (il = 0; il < 2; ++il)
            for (wc = 0; wc < 2; ++wc)
            {
               OUT("=== W ct=%d bd=%d il=%d wc=%d\n", ct, bd, il, wc);
               test_write(ct, bd, il, PNG_ALL_FILTERS, 6,
                   PNG_Z_DEFAULT_STRATEGY, wc, (unsigned)(ci * 31 + bi * 7));
            }
      }

   /* filter / compression matrix on a single format */
   {
      static const int filts[] = {
         PNG_NO_FILTERS, PNG_FILTER_NONE, PNG_FILTER_SUB, PNG_FILTER_UP,
         PNG_FILTER_AVG, PNG_FILTER_PAETH, PNG_FAST_FILTERS, PNG_ALL_FILTERS
      };
      size_t fi;
      int lvl, strat;
      for (fi = 0; fi < sizeof filts / sizeof filts[0]; ++fi)
         for (lvl = 0; lvl <= 9; lvl += 3)
            for (strat = 0; strat <= 3; ++strat)
            {
               OUT("=== F f=%d l=%d s=%d\n", filts[fi], lvl, strat);
               test_write(PNG_COLOR_TYPE_RGB_ALPHA, 8, 0, filts[fi], lvl,
                   strat, 0, (unsigned)(fi * 13 + lvl + strat));
            }
   }

   /* transformations and progressive reads over reference images */
   for (ci = 0; ci < sizeof cts / sizeof cts[0]; ++ci)
      for (bi = 0; bi < sizeof bds / sizeof bds[0]; ++bi)
      {
         int ct = cts[ci], bd = bds[bi];
         size_t len;
         unsigned char *png;
         if (ct == PNG_COLOR_TYPE_PALETTE && bd == 16) continue;
         if (ct != PNG_COLOR_TYPE_GRAY && ct != PNG_COLOR_TYPE_PALETTE &&
             bd < 8) continue;
         for (il = 0; il < 2; ++il)
         {
            int t;
            png = make_reference_png(&len, ct, bd, il);
            if (png == NULL) continue;
            OUT("=== R ct=%d bd=%d il=%d len=%zu\n", ct, bd, il, len);
            for (t = 0; t <= 8; ++t)
               test_read_manual(png, len, t);
            test_read_transforms(png, len, PNG_TRANSFORM_IDENTITY, "id");
            test_read_transforms(png, len,
                PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_PACKING |
                PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_BGR, "mix");
            test_read_transforms(png, len,
                PNG_TRANSFORM_GRAY_TO_RGB | PNG_TRANSFORM_EXPAND_16 |
                PNG_TRANSFORM_SWAP_ENDIAN | PNG_TRANSFORM_INVERT_ALPHA, "mix2");
            test_read_transforms(png, len,
                PNG_TRANSFORM_STRIP_ALPHA | PNG_TRANSFORM_INVERT_MONO |
                PNG_TRANSFORM_PACKSWAP | PNG_TRANSFORM_SHIFT |
                PNG_TRANSFORM_SWAP_ALPHA | PNG_TRANSFORM_SCALE_16, "mix3");
            test_progressive(png, len, 1);
            test_progressive(png, len, 7);
            test_progressive(png, len, 1000);
            free(png);
         }
      }

   /* simplified API over every format */
   {
      static const png_uint_32 fmts[] = {
         PNG_FORMAT_GRAY, PNG_FORMAT_GA, PNG_FORMAT_AG, PNG_FORMAT_RGB,
         PNG_FORMAT_BGR, PNG_FORMAT_RGBA, PNG_FORMAT_ARGB, PNG_FORMAT_BGRA,
         PNG_FORMAT_ABGR, PNG_FORMAT_LINEAR_Y, PNG_FORMAT_LINEAR_Y_ALPHA,
         PNG_FORMAT_LINEAR_RGB, PNG_FORMAT_LINEAR_RGB_ALPHA
      };
      size_t fi;
      for (fi = 0; fi < sizeof fmts / sizeof fmts[0]; ++fi)
      {
         test_simplified(fmts[fi], 0, (unsigned)(fi * 17 + 3));
         test_simplified(fmts[fi], 1, (unsigned)(fi * 19 + 5));
      }
   }

   /* error paths: truncated / corrupt data */
   {
      size_t len;
      unsigned char *png = make_reference_png(&len, PNG_COLOR_TYPE_RGB, 8, 0);
      size_t cut;
      if (png != NULL)
      {
         for (cut = 8; cut < len; cut += len / 5 + 1)
         {
            OUT("=== TRUNC %zu/%zu\n", cut, len);
            test_read_transforms(png, cut, PNG_TRANSFORM_IDENTITY, "trunc");
            test_progressive(png, cut, 13);
         }
         /* corrupt a byte in the IDAT */
         if (len > 100)
         {
            unsigned char save = png[len - 20];
            png[len - 20] ^= 0xff;
            OUT("=== CORRUPT\n");
            test_read_transforms(png, len, PNG_TRANSFORM_IDENTITY, "corrupt");
            png[len - 20] = save;
         }
         free(png);
      }
   }

   OUT("FINAL hash=%lu\n", g_hash);
   return 0;
}
