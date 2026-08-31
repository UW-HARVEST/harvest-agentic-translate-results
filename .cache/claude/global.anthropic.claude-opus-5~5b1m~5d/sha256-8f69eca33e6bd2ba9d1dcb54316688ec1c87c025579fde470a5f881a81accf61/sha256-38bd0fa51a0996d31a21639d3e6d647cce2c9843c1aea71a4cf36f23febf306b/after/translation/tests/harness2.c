/* Second differential harness: covers the API surface harness.c does not
 * (png_write_png, file/stdio I/O, png_set_rows, user limits, error paths,
 * custom text compression, progressive_combine_row, time conversion, ...).
 *
 * All image widths are multiples of 8 so that no row ever has undefined
 * padding bits (libpng leaves those at whatever the caller's buffer held).
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <time.h>
#include "png.h"

static unsigned long g_hash;
static void feed(const void *p, size_t n)
{
   const unsigned char *b = p;
   size_t i;
   for (i = 0; i < n; ++i) g_hash = g_hash * 1000003u + b[i];
}
#define OUT(...) printf(__VA_ARGS__)
static void dump(const char *tag, const unsigned char *b, size_t n)
{
   size_t i;
   OUT("%s len=%zu\n", tag, n);
   for (i = 0; i < n; ++i) { OUT("%02x", b[i]); if ((i & 31) == 31) OUT("\n"); }
   if (n & 31) OUT("\n");
}

typedef struct { unsigned char *buf; size_t len, cap; } membuf;
static void mem_write(png_structp pp, png_bytep d, size_t n)
{
   membuf *m = png_get_io_ptr(pp);
   if (m->len + n > m->cap) { m->cap = (m->len+n)*2+1024; m->buf = realloc(m->buf, m->cap); }
   memcpy(m->buf+m->len, d, n); m->len += n;
}
static void mem_flush(png_structp pp) { (void)pp; }
typedef struct { const unsigned char *buf; size_t len, pos; } rdbuf;
static void mem_read(png_structp pp, png_bytep d, size_t n)
{
   rdbuf *r = png_get_io_ptr(pp);
   if (r->pos + n > r->len) png_error(pp, "eof");
   memcpy(d, r->buf + r->pos, n); r->pos += n;
}
static void err_fn(png_structp pp, png_const_charp m)
{ OUT("ERR %s\n", m); longjmp(png_jmpbuf(pp), 1); }
static void warn_fn(png_structp pp, png_const_charp m)
{ (void)pp; OUT("WARN %s\n", m); }

static void fill(unsigned char *p, size_t n, unsigned seed)
{
   size_t i; unsigned x = seed*2654435761u+1;
   for (i = 0; i < n; ++i) { x = x*1103515245u+12345u; p[i] = (unsigned char)((x>>16)^(i*11)); }
}

/* ---------------- png_write_png with row_pointers ---------------- */
static void test_write_png(int ct, int bd, int il, int transforms, unsigned seed)
{
   png_structp pp;
   png_infop ip;
   membuf m;
   png_uint_32 w = 32, h = 9, y;
   size_t rowbytes;
   png_bytep *rows;

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   OUT("--- write_png ct=%d bd=%d il=%d tr=%x\n", ct, bd, il, transforms);
   if (setjmp(png_jmpbuf(pp)))
   { OUT("wp longjmp\n"); png_destroy_write_struct(&pp, &ip); free(m.buf); return; }
   png_set_write_fn(pp, &m, mem_write, mem_flush);
   png_set_IHDR(pp, ip, w, h, bd, ct, il, 0, 0);
   if ((ct & PNG_COLOR_MASK_PALETTE) != 0)
   {
      png_color pal[256]; int i, n = 1 << bd;
      for (i = 0; i < n; ++i)
      { pal[i].red=(png_byte)(i*7); pal[i].green=(png_byte)(i*9); pal[i].blue=(png_byte)(i*3); }
      png_set_PLTE(pp, ip, pal, n);
   }
   png_set_gAMA_fixed(pp, ip, 45455);
   /* Over-allocate: write transforms such as PACKING / STRIP_FILLER make
    * libpng read up to one byte (or two for 16-bit) per channel per pixel,
    * plus a possible extra filler channel.  Fill the whole buffer so that no
    * uninitialised byte is ever read.
    */
   rowbytes = (size_t)w * 5 * 2 + 16;
   rows = malloc(h * sizeof(png_bytep));
   for (y = 0; y < h; ++y) { rows[y] = malloc(rowbytes); fill(rows[y], rowbytes, seed + y); }
   png_set_rows(pp, ip, rows);
   png_write_png(pp, ip, transforms, NULL);
   dump("WPNG", m.buf, m.len);
   feed(m.buf, m.len);
   for (y = 0; y < h; ++y) free(rows[y]);
   free(rows);
   png_destroy_write_struct(&pp, &ip);
   free(m.buf);
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- file / stdio round-trip ---------------- */
static void test_file_io(void)
{
   png_structp pp; png_infop ip;
   FILE *f;
   png_uint_32 w = 40, h = 7, y;
   size_t rowbytes;
   png_bytep *rows;
   const char *path = "tmp_h2.png";

   OUT("--- file_io\n");
   f = fopen(path, "wb");
   if (f == NULL) { OUT("fopen failed\n"); return; }
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   if (setjmp(png_jmpbuf(pp)))
   { OUT("f longjmp\n"); png_destroy_write_struct(&pp, &ip); fclose(f); return; }
   png_init_io(pp, f);
   png_set_IHDR(pp, ip, w, h, 8, PNG_COLOR_TYPE_RGBA, 0, 0, 0);
   png_set_flush(pp, 2);
   png_write_info(pp, ip);
   rowbytes = png_get_rowbytes(pp, ip);
   rows = malloc(h * sizeof(png_bytep));
   for (y = 0; y < h; ++y) { rows[y] = malloc(rowbytes); fill(rows[y], rowbytes, 5 + y); }
   for (y = 0; y < h; ++y) { png_write_row(pp, rows[y]); png_write_flush(pp); }
   png_write_end(pp, ip);
   png_destroy_write_struct(&pp, &ip);
   fclose(f);

   /* read back with stdio */
   f = fopen(path, "rb");
   {
      png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
      png_infop rip = png_create_info_struct(rp);
      png_byte sig[8];
      if (setjmp(png_jmpbuf(rp)))
      { OUT("fr longjmp\n"); png_destroy_read_struct(&rp, &rip, NULL); fclose(f); return; }
      if (fread(sig, 1, 8, f) != 8) { OUT("short sig\n"); }
      OUT("sig_cmp=%d\n", png_sig_cmp(sig, 0, 8));
      png_init_io(rp, f);
      png_set_sig_bytes(rp, 8);
      png_read_info(rp, rip);
      OUT("file %u %u %d %d io_state=%u chunk=%08x\n",
          png_get_image_width(rp, rip), png_get_image_height(rp, rip),
          png_get_bit_depth(rp, rip), png_get_color_type(rp, rip),
          (unsigned)png_get_io_state(rp), (unsigned)png_get_io_chunk_type(rp));
      {
         size_t rb = png_get_rowbytes(rp, rip);
         png_bytep r = malloc(rb);
         for (y = 0; y < h; ++y)
         {
            memset(r, 0, rb);
            png_read_row(rp, r, NULL);
            feed(r, rb);
            if (memcmp(r, rows[y], rb) != 0) OUT("file row %u mismatch\n", y);
         }
         free(r);
      }
      png_read_end(rp, NULL);
      png_destroy_read_struct(&rp, &rip, NULL);
   }
   fclose(f);
   for (y = 0; y < h; ++y) free(rows[y]);
   free(rows);
   remove(path);
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- simplified API to/from file ---------------- */
static void test_simplified_file(png_uint_32 fmt, int c8)
{
   png_image im;
   unsigned char *buf;
   size_t sz;
   const char *path = "tmp_h2s.png";

   memset(&im, 0, sizeof im);
   im.version = PNG_IMAGE_VERSION;
   im.width = 16; im.height = 5; im.format = fmt;
   sz = PNG_IMAGE_SIZE(im);
   buf = malloc(sz);
   fill(buf, sz, fmt + c8);
   OUT("--- simpl_file fmt=%u c8=%d\n", fmt, c8);
   if (png_image_write_to_file(&im, path, c8, buf, 0, NULL))
   {
      FILE *f = fopen(path, "rb");
      if (f != NULL)
      {
         unsigned char tmp[65536];
         size_t n = fread(tmp, 1, sizeof tmp, f);
         fclose(f);
         dump("SFILE", tmp, n);
         feed(tmp, n);
      }
      {
         png_image r;
         memset(&r, 0, sizeof r);
         r.version = PNG_IMAGE_VERSION;
         if (png_image_begin_read_from_file(&r, path))
         {
            unsigned char *rb;
            size_t rs;
            r.format = fmt;
            rs = PNG_IMAGE_SIZE(r);
            rb = malloc(rs);
            memset(rb, 0, rs);
            if (png_image_finish_read(&r, NULL, rb, 0, NULL))
            { dump("SFREAD", rb, rs); feed(rb, rs); }
            else OUT("sf read fail %s\n", r.message);
            free(rb);
         }
         else OUT("sf begin fail %s\n", r.message);
         png_image_free(&r);
      }
      remove(path);
   }
   else OUT("write_to_file failed %s\n", im.message);
   free(buf);
   png_image_free(&im);
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- custom text compression ---------------- */
static void test_text_compression(int level, int strategy, int wbits, int mem)
{
   png_structp pp; png_infop ip; membuf m;
   png_text txt[2];
   png_uint_32 w = 8, h = 2, y;
   png_bytep row;

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   OUT("--- textcomp l=%d s=%d w=%d m=%d\n", level, strategy, wbits, mem);
   if (setjmp(png_jmpbuf(pp)))
   { OUT("tc longjmp\n"); png_destroy_write_struct(&pp, &ip); free(m.buf); return; }
   png_set_write_fn(pp, &m, mem_write, mem_flush);
   png_set_text_compression_level(pp, level);
   png_set_text_compression_strategy(pp, strategy);
   png_set_text_compression_window_bits(pp, wbits);
   png_set_text_compression_mem_level(pp, mem);
   png_set_IHDR(pp, ip, w, h, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
   memset(txt, 0, sizeof txt);
   txt[0].compression = PNG_TEXT_COMPRESSION_zTXt;
   txt[0].key = (png_charp)"Software";
   txt[0].text = (png_charp)
     "repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat "
     "repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat";
   txt[1].compression = PNG_ITXT_COMPRESSION_zTXt;
   txt[1].key = (png_charp)"Author";
   txt[1].lang = (png_charp)"en-GB";
   txt[1].lang_key = (png_charp)"Author";
   txt[1].text = (png_charp)
     "another long international text another long international text";
   png_set_text(pp, ip, txt, 2);
   png_write_info(pp, ip);
   row = malloc(w);
   for (y = 0; y < h; ++y) { fill(row, w, y + 1); png_write_row(pp, row); }
   free(row);
   png_write_end(pp, ip);
   dump("TCPNG", m.buf, m.len);
   feed(m.buf, m.len);
   png_destroy_write_struct(&pp, &ip);
   free(m.buf);
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- user limits / error paths ---------------- */
static unsigned char *mk_simple(size_t *lenp, png_uint_32 w, png_uint_32 h,
    int ct, int bd, int il, int nchunks)
{
   png_structp pp; png_infop ip; membuf m;
   png_uint_32 y;
   size_t rowbytes;
   png_bytep row;

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   if (setjmp(png_jmpbuf(pp)))
   { png_destroy_write_struct(&pp, &ip); *lenp = 0; return NULL; }
   png_set_write_fn(pp, &m, mem_write, mem_flush);
   png_set_IHDR(pp, ip, w, h, bd, ct, il, 0, 0);
   if ((ct & PNG_COLOR_MASK_PALETTE) != 0)
   {
      png_color pal[256]; int i, n = 1 << bd;
      for (i = 0; i < n; ++i)
      { pal[i].red=(png_byte)i; pal[i].green=(png_byte)(i*2); pal[i].blue=(png_byte)(i*3); }
      png_set_PLTE(pp, ip, pal, n);
   }
   if (nchunks > 0)
   {
      int i;
      png_text txt[1];
      memset(txt, 0, sizeof txt);
      txt[0].compression = PNG_TEXT_COMPRESSION_NONE;
      txt[0].key = (png_charp)"K";
      txt[0].text = (png_charp)"v";
      for (i = 0; i < nchunks; ++i) png_set_text(pp, ip, txt, 1);
   }
   png_write_info(pp, ip);
   rowbytes = png_get_rowbytes(pp, ip);
   row = malloc(rowbytes);
   for (y = 0; y < h; ++y) { fill(row, rowbytes, y*3+1); png_write_row(pp, row); }
   free(row);
   png_write_end(pp, ip);
   png_destroy_write_struct(&pp, &ip);
   *lenp = m.len;
   return m.buf;
}

static void test_limits(const unsigned char *png, size_t len,
    png_uint_32 wmax, png_uint_32 hmax, png_uint_32 cache, png_alloc_size_t mm)
{
   png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   png_infop ip = png_create_info_struct(rp);
   rdbuf rb;
   rb.buf = png; rb.len = len; rb.pos = 0;
   OUT("--- limits w=%u h=%u cache=%u mm=%zu\n", wmax, hmax, cache, mm);
   if (setjmp(png_jmpbuf(rp)))
   { OUT("lim longjmp\n"); png_destroy_read_struct(&rp, &ip, NULL); return; }
   png_set_read_fn(rp, &rb, mem_read);
   png_set_user_limits(rp, wmax, hmax);
   png_set_chunk_cache_max(rp, cache);
   png_set_chunk_malloc_max(rp, mm);
   OUT("get %u %u %u %zu\n", png_get_user_width_max(rp),
       png_get_user_height_max(rp), png_get_chunk_cache_max(rp),
       (size_t)png_get_chunk_malloc_max(rp));
   png_read_info(rp, ip);
   OUT("ok %u %u\n", png_get_image_width(rp, ip), png_get_image_height(rp, ip));
   {
      png_textp tp; int n = 0;
      png_get_text(rp, ip, &tp, &n);
      OUT("ntext=%d\n", n);
   }
   png_read_end(rp, NULL);
   png_destroy_read_struct(&rp, &ip, NULL);
   OUT("hash=%lu\n", g_hash);
}

/* Corrupt every single byte position in a small PNG in turn and report. */
static void test_fuzz_bytes(const unsigned char *png, size_t len)
{
   unsigned char *copy = malloc(len);
   size_t i;
   OUT("--- fuzz bytes len=%zu\n", len);
   for (i = 0; i < len; ++i)
   {
      png_structp rp;
      png_infop ip;
      rdbuf rb;
      memcpy(copy, png, len);
      copy[i] ^= 0x5a;
      rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
      ip = png_create_info_struct(rp);
      rb.buf = copy; rb.len = len; rb.pos = 0;
      OUT("[%zu] ", i);
      if (setjmp(png_jmpbuf(rp)))
         OUT("fail\n");
      else
      {
         png_set_read_fn(rp, &rb, mem_read);
         png_set_crc_action(rp, PNG_CRC_DEFAULT, PNG_CRC_DEFAULT);
         png_read_info(rp, ip);
         {
            png_uint_32 h = png_get_image_height(rp, ip);
            size_t rbs = png_get_rowbytes(rp, ip);
            png_bytep r = malloc(rbs ? rbs : 1);
            png_uint_32 y;
            int npass = png_get_interlace_type(rp, ip) == PNG_INTERLACE_ADAM7
                ? png_set_interlace_handling(rp) : 1;
            int p;
            png_read_update_info(rp, ip);
            rbs = png_get_rowbytes(rp, ip);
            r = realloc(r, rbs ? rbs : 1);
            for (p = 0; p < npass; ++p)
               for (y = 0; y < h; ++y)
               { memset(r, 0, rbs); png_read_row(rp, r, NULL); feed(r, rbs); }
            free(r);
            png_read_end(rp, NULL);
         }
         OUT("ok h=%lu\n", g_hash);
      }
      png_destroy_read_struct(&rp, &ip, NULL);
   }
   free(copy);
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- progressive with combine_row ---------------- */
static png_bytep *g_prows;
static size_t g_prowbytes;
static png_uint_32 g_ph;
static void p2_info(png_structp pp, png_infop ip)
{
   png_uint_32 y;
   if (png_get_interlace_type(pp, ip) == PNG_INTERLACE_ADAM7)
      png_set_interlace_handling(pp);
   png_read_update_info(pp, ip);
   g_prowbytes = png_get_rowbytes(pp, ip);
   g_ph = png_get_image_height(pp, ip);
   g_prows = malloc(g_ph * sizeof(png_bytep));
   for (y = 0; y < g_ph; ++y)
   { g_prows[y] = malloc(g_prowbytes); memset(g_prows[y], 0, g_prowbytes); }
   OUT("p2 info rb=%zu h=%u\n", g_prowbytes, g_ph);
}
static void p2_row(png_structp pp, png_bytep newrow, png_uint_32 rownum, int pass)
{
   (void)pass;
   if (rownum < g_ph)
      png_progressive_combine_row(pp, g_prows[rownum], newrow);
}
static void p2_end(png_structp pp, png_infop ip) { (void)pp; (void)ip; OUT("p2 end\n"); }

static void test_progressive_combine(const unsigned char *png, size_t len,
    size_t chunk)
{
   png_structp pp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   png_infop ip = png_create_info_struct(pp);
   size_t pos = 0;
   png_uint_32 y;
   g_prows = NULL; g_ph = 0;
   OUT("--- prog_combine chunk=%zu\n", chunk);
   if (setjmp(png_jmpbuf(pp)))
   { OUT("p2 longjmp\n"); png_destroy_read_struct(&pp, &ip, NULL); return; }
   png_set_progressive_read_fn(pp, NULL, p2_info, p2_row, p2_end);
   while (pos < len)
   {
      size_t n = len - pos < chunk ? len - pos : chunk;
      png_process_data(pp, ip, (png_bytep)(png + pos), n);
      pos += n;
      if (chunk == 3)
      {
         size_t rem = png_process_data_pause(pp, 0);
         if (rem != 0) { pos -= rem; }
      }
   }
   OUT("skip=%u\n", (unsigned)png_process_data_skip(pp));
   for (y = 0; y < g_ph; ++y) { feed(g_prows[y], g_prowbytes); if (y < 2) dump("prow", g_prows[y], g_prowbytes); }
   for (y = 0; y < g_ph; ++y) free(g_prows[y]);
   free(g_prows);
   png_destroy_read_struct(&pp, &ip, NULL);
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- time conversions ---------------- */
static void test_time(void)
{
   png_time t;
   struct tm tm;
   time_t tt;
   char buf[29];
   int i;
   OUT("--- time\n");
   memset(&tm, 0, sizeof tm);
   tm.tm_year = 100; tm.tm_mon = 5; tm.tm_mday = 15;
   tm.tm_hour = 1; tm.tm_min = 2; tm.tm_sec = 3;
   png_convert_from_struct_tm(&t, &tm);
   OUT("from_tm %u %u %u %u %u %u\n", t.year, t.month, t.day, t.hour, t.minute, t.second);
   for (i = 0; i < 5; ++i)
   {
      tt = (time_t)(i * 100000000);
      png_convert_from_time_t(&t, tt);
      OUT("from_t %d -> %u %u %u %u %u %u\n", i, t.year, t.month, t.day,
          t.hour, t.minute, t.second);
      if (png_convert_to_rfc1123_buffer(buf, &t)) OUT("  %s\n", buf);
   }
   OUT("hash=%lu\n", g_hash);
}

/* ---------------- benign errors / crc actions ---------------- */
static void test_crc_actions(const unsigned char *png, size_t len, int crit,
    int anc, int benign)
{
   png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   png_infop ip = png_create_info_struct(rp);
   rdbuf rb;
   rb.buf = png; rb.len = len; rb.pos = 0;
   OUT("--- crc crit=%d anc=%d benign=%d\n", crit, anc, benign);
   if (setjmp(png_jmpbuf(rp)))
   { OUT("crc longjmp\n"); png_destroy_read_struct(&rp, &ip, NULL); return; }
   png_set_read_fn(rp, &rb, mem_read);
   png_set_crc_action(rp, crit, anc);
   png_set_benign_errors(rp, benign);
   png_set_check_for_invalid_index(rp, 1);
   png_read_info(rp, ip);
   {
      png_uint_32 h = png_get_image_height(rp, ip), y;
      size_t rbs;
      png_bytep r;
      int npass = png_get_interlace_type(rp, ip) == PNG_INTERLACE_ADAM7
          ? png_set_interlace_handling(rp) : 1;
      int p;
      png_read_update_info(rp, ip);
      rbs = png_get_rowbytes(rp, ip);
      r = malloc(rbs);
      for (p = 0; p < npass; ++p)
         for (y = 0; y < h; ++y)
         { memset(r, 0, rbs); png_read_row(rp, r, NULL); feed(r, rbs); }
      free(r);
      png_read_end(rp, NULL);
      OUT("palette_max=%d\n", png_get_palette_max(rp, ip));
   }
   png_destroy_read_struct(&rp, &ip, NULL);
   OUT("hash=%lu\n", g_hash);
}

int main(void)
{
   static const int cts[] = { 0, 3, 2, 4, 6 };
   static const int bds[] = { 1, 2, 4, 8, 16 };
   size_t ci, bi;
   int il;
   size_t len;
   unsigned char *png;

   g_hash = 5381;

   test_time();

   for (ci = 0; ci < 5; ++ci)
      for (bi = 0; bi < 5; ++bi)
      {
         int ct = cts[ci], bd = bds[bi];
         if (ct == 3 && bd == 16) continue;
         if (ct != 0 && ct != 3 && bd < 8) continue;
         for (il = 0; il < 2; ++il)
         {
            test_write_png(ct, bd, il, PNG_TRANSFORM_IDENTITY, (unsigned)(ci*5+bi));
            test_write_png(ct, bd, il, PNG_TRANSFORM_BGR|PNG_TRANSFORM_PACKING,
                (unsigned)(ci*7+bi));
            test_write_png(ct, bd, il,
                PNG_TRANSFORM_INVERT_MONO|PNG_TRANSFORM_PACKSWAP|
                PNG_TRANSFORM_SWAP_ENDIAN|PNG_TRANSFORM_INVERT_ALPHA|
                PNG_TRANSFORM_SWAP_ALPHA|PNG_TRANSFORM_STRIP_FILLER_AFTER,
                (unsigned)(ci*11+bi));
         }
      }

   test_file_io();

   {
      static const png_uint_32 fmts[] = {
         PNG_FORMAT_GRAY, PNG_FORMAT_GA, PNG_FORMAT_RGB, PNG_FORMAT_RGBA,
         PNG_FORMAT_BGRA, PNG_FORMAT_ABGR, PNG_FORMAT_LINEAR_Y,
         PNG_FORMAT_LINEAR_RGB_ALPHA
      };
      size_t i;
      for (i = 0; i < sizeof fmts / sizeof fmts[0]; ++i)
      { test_simplified_file(fmts[i], 0); test_simplified_file(fmts[i], 1); }
   }

   {
      int l, s;
      for (l = -1; l <= 9; l += 5)
         for (s = 0; s <= 3; ++s)
            test_text_compression(l, s, 15, 8);
      test_text_compression(9, 0, 9, 1);
      test_text_compression(1, 2, 12, 5);
   }

   /* user limits + error paths */
   png = mk_simple(&len, 32, 8, PNG_COLOR_TYPE_RGB, 8, 0, 5);
   if (png != NULL)
   {
      test_limits(png, len, 1000000, 1000000, 1000, 8000000);
      test_limits(png, len, 16, 1000000, 1000, 8000000);
      test_limits(png, len, 1000000, 4, 1000, 8000000);
      test_limits(png, len, 1000000, 1000000, 2, 8000000);
      test_limits(png, len, 1000000, 1000000, 1000, 4);
      test_crc_actions(png, len, PNG_CRC_DEFAULT, PNG_CRC_DEFAULT, 0);
      test_crc_actions(png, len, PNG_CRC_ERROR_QUIT, PNG_CRC_ERROR_QUIT, 0);
      test_crc_actions(png, len, PNG_CRC_WARN_USE, PNG_CRC_WARN_DISCARD, 1);
      test_crc_actions(png, len, PNG_CRC_QUIET_USE, PNG_CRC_QUIET_USE, 1);
      test_crc_actions(png, len, PNG_CRC_NO_CHANGE, PNG_CRC_NO_CHANGE, 0);
      free(png);
   }

   /* fuzz every byte of a tiny image */
   png = mk_simple(&len, 8, 2, PNG_COLOR_TYPE_RGB, 8, 0, 0);
   if (png != NULL) { test_fuzz_bytes(png, len); free(png); }
   png = mk_simple(&len, 8, 2, PNG_COLOR_TYPE_PALETTE, 4, 1, 1);
   if (png != NULL) { test_fuzz_bytes(png, len); free(png); }

   /* progressive with combine_row over all formats */
   for (ci = 0; ci < 5; ++ci)
      for (bi = 0; bi < 5; ++bi)
      {
         int ct = cts[ci], bd = bds[bi];
         if (ct == 3 && bd == 16) continue;
         if (ct != 0 && ct != 3 && bd < 8) continue;
         for (il = 0; il < 2; ++il)
         {
            png = mk_simple(&len, 32, 9, ct, bd, il, 0);
            if (png == NULL) continue;
            OUT("=== PC ct=%d bd=%d il=%d\n", ct, bd, il);
            test_progressive_combine(png, len, 1);
            test_progressive_combine(png, len, 5);
            test_progressive_combine(png, len, 3);
            test_progressive_combine(png, len, 100000);
            free(png);
         }
      }

   OUT("FINAL hash=%lu\n", g_hash);
   return 0;
}
