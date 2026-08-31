/* Third differential harness: the floating-point API variants, user
 * callbacks, and the remaining getters/setters. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include "png.h"

static unsigned long g_hash;
static void feed(const void *p, size_t n)
{
   const unsigned char *b = p;
   size_t i;
   for (i = 0; i < n; ++i) g_hash = g_hash * 1000003u + b[i];
}
#define OUT(...) printf(__VA_ARGS__)
static void dump(const char *t, const unsigned char *b, size_t n)
{
   size_t i;
   OUT("%s len=%zu\n", t, n);
   for (i = 0; i < n; ++i) { OUT("%02x", b[i]); if ((i & 31) == 31) OUT("\n"); }
   if (n & 31) OUT("\n");
}
/* print a double in a byte-exact way */
static void pd(const char *t, double d)
{
   unsigned char b[sizeof d];
   memcpy(b, &d, sizeof d);
   OUT("%s=", t);
   { size_t i; for (i = 0; i < sizeof d; ++i) OUT("%02x", b[i]); }
   OUT(" (%.17g)\n", d);
}
static void pf(const char *t, float f)
{
   unsigned char b[sizeof f];
   memcpy(b, &f, sizeof f);
   OUT("%s=", t);
   { size_t i; for (i = 0; i < sizeof f; ++i) OUT("%02x", b[i]); }
   OUT(" (%.9g)\n", (double)f);
}

typedef struct { unsigned char *buf; size_t len, cap; } membuf;
static void mem_write(png_structp pp, png_bytep d, size_t n)
{
   membuf *m = png_get_io_ptr(pp);
   if (m->len + n > m->cap) { m->cap=(m->len+n)*2+1024; m->buf=realloc(m->buf,m->cap); }
   memcpy(m->buf+m->len, d, n); m->len += n;
}
static void mem_flush(png_structp pp) { (void)pp; }
typedef struct { const unsigned char *buf; size_t len, pos; } rdbuf;
static void mem_read(png_structp pp, png_bytep d, size_t n)
{
   rdbuf *r = png_get_io_ptr(pp);
   if (r->pos+n > r->len) png_error(pp, "eof");
   memcpy(d, r->buf+r->pos, n); r->pos += n;
}
static void err_fn(png_structp pp, png_const_charp m)
{ OUT("ERR %s\n", m); longjmp(png_jmpbuf(pp), 1); }
static void warn_fn(png_structp pp, png_const_charp m)
{ (void)pp; OUT("WARN %s\n", m); }
static void fill(unsigned char *p, size_t n, unsigned seed)
{
   size_t i; unsigned x = seed*2654435761u+1;
   for (i = 0; i < n; ++i) { x = x*1103515245u+12345u; p[i]=(unsigned char)((x>>16)^(i*13)); }
}

static void rstat(png_structp pp, png_uint_32 row, int pass)
{ (void)pp; OUT("rstat %u %d\n", row, pass); }
static void wstat(png_structp pp, png_uint_32 row, int pass)
{ (void)pp; OUT("wstat %u %d\n", row, pass); }

static void utrans(png_structp pp, png_row_infop ri, png_bytep row)
{
   size_t i;
   OUT("utrans w=%u rb=%zu ct=%d bd=%d ch=%d pd=%d row=%u pass=%d ptr=%p\n",
       ri->width, ri->rowbytes, ri->color_type, ri->bit_depth, ri->channels,
       ri->pixel_depth, png_get_current_row_number(pp),
       png_get_current_pass_number(pp),
       png_get_user_transform_ptr(pp));
   for (i = 0; i < ri->rowbytes; ++i) row[i] = (png_byte)(row[i] ^ 0x33);
}

static int uchunk(png_structp pp, png_unknown_chunkp c)
{
   OUT("uchunk %s size=%zu loc=%u ptr=%p\n", (char*)c->name, c->size,
       c->location, png_get_user_chunk_ptr(pp));
   feed(c->data, c->size);
   return 1;
}

/* ---------------------------------------------------------------- */
static void test_float_setters(void)
{
   png_structp pp; png_infop ip; membuf m;
   png_uint_32 w = 16, h = 4, y;
   png_bytep row;
   size_t rowbytes;

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   OUT("--- float_setters\n");
   if (setjmp(png_jmpbuf(pp)))
   { OUT("fs longjmp\n"); png_destroy_write_struct(&pp,&ip); free(m.buf); return; }
   png_set_write_fn(pp, &m, mem_write, mem_flush);
   png_set_IHDR(pp, ip, w, h, 16, PNG_COLOR_TYPE_RGBA, 0, 0, 0);
   png_set_gAMA(pp, ip, 0.45455);
   png_set_cHRM(pp, ip, 0.3127, 0.3290, 0.64, 0.33, 0.30, 0.60, 0.15, 0.06);
   png_set_cHRM_XYZ(pp, ip, 0.4124, 0.2126, 0.0193, 0.3576, 0.7152, 0.1192,
       0.1805, 0.0722, 0.9505);
   png_set_cLLI(pp, ip, 1000.0, 400.0);
   png_set_mDCV(pp, ip, 0.3127, 0.3290, 0.708, 0.292, 0.170, 0.797, 0.131,
       0.046, 1000.0, 0.005);
   png_set_sCAL(pp, ip, PNG_SCALE_METER, 1.5, 0.25);
   png_set_write_status_fn(pp, wstat);
   png_write_info(pp, ip);
   rowbytes = png_get_rowbytes(pp, ip);
   row = malloc(rowbytes);
   for (y = 0; y < h; ++y) { fill(row, rowbytes, y+9); png_write_row(pp, row); }
   free(row);
   png_write_end(pp, ip);
   dump("FPNG", m.buf, m.len);
   feed(m.buf, m.len);

   /* read back, dump the float getters */
   {
      png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
      png_infop rip = png_create_info_struct(rp);
      rdbuf rb;
      rb.buf = m.buf; rb.len = m.len; rb.pos = 0;
      if (setjmp(png_jmpbuf(rp)))
      { OUT("fsr longjmp\n"); png_destroy_read_struct(&rp,&rip,NULL); goto done; }
      png_set_read_fn(rp, &rb, mem_read);
      png_read_info(rp, rip);
      {
         double g;
         if (png_get_gAMA(rp, rip, &g)) pd("gAMA", g);
      }
      {
         double wx,wy,rx,ry,gx,gy,bx,by;
         if (png_get_cHRM(rp, rip, &wx,&wy,&rx,&ry,&gx,&gy,&bx,&by))
         { pd("wx",wx); pd("wy",wy); pd("rx",rx); pd("ry",ry);
           pd("gx",gx); pd("gy",gy); pd("bx",bx); pd("by",by); }
      }
      {
         double rX,rY,rZ,gX,gY,gZ,bX,bY,bZ;
         if (png_get_cHRM_XYZ(rp, rip, &rX,&rY,&rZ,&gX,&gY,&gZ,&bX,&bY,&bZ))
         { pd("rX",rX); pd("rY",rY); pd("rZ",rZ); pd("gX",gX); pd("gY",gY);
           pd("gZ",gZ); pd("bX",bX); pd("bY",bY); pd("bZ",bZ); }
      }
      {
         png_fixed_point rX,rY,rZ,gX,gY,gZ,bX,bY,bZ;
         if (png_get_cHRM_XYZ_fixed(rp, rip, &rX,&rY,&rZ,&gX,&gY,&gZ,&bX,&bY,&bZ))
            OUT("XYZfix %d %d %d %d %d %d %d %d %d\n", rX,rY,rZ,gX,gY,gZ,bX,bY,bZ);
      }
      {
         double a, b;
         if (png_get_cLLI(rp, rip, &a, &b)) { pd("maxCLL",a); pd("maxFALL",b); }
      }
      {
         double wx,wy,rx,ry,gx,gy,bx,by,mx,mn;
         if (png_get_mDCV(rp, rip, &wx,&wy,&rx,&ry,&gx,&gy,&bx,&by,&mx,&mn))
         { pd("mwx",wx); pd("mwy",wy); pd("mrx",rx); pd("mry",ry);
           pd("mgx",gx); pd("mgy",gy); pd("mbx",bx); pd("mby",by);
           pd("mmax",mx); pd("mmin",mn); }
      }
      {
         int unit; double sw, sh;
         if (png_get_sCAL(rp, rip, &unit, &sw, &sh))
         { OUT("sCALunit=%d\n", unit); pd("sw",sw); pd("sh",sh); }
      }
      {
         int unit; png_fixed_point sw, sh;
         if (png_get_sCAL_fixed(rp, rip, &unit, &sw, &sh))
            OUT("sCALfix %d %d %d\n", unit, sw, sh);
      }
      png_destroy_read_struct(&rp, &rip, NULL);
   }
done:
   png_destroy_write_struct(&pp, &ip);
   free(m.buf);
   OUT("hash=%lu\n", g_hash);
}

/* pHYs / oFFs derived getters */
static void test_easy_access(void)
{
   png_structp pp; png_infop ip; membuf m;
   png_uint_32 res[3][2] = { {1,1}, {2835,2835}, {100000,1} };
   int i;
   OUT("--- easy_access\n");
   for (i = 0; i < 3; ++i)
   {
      memset(&m, 0, sizeof m);
      pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
      ip = png_create_info_struct(pp);
      if (setjmp(png_jmpbuf(pp)))
      { png_destroy_write_struct(&pp,&ip); free(m.buf); continue; }
      png_set_write_fn(pp, &m, mem_write, mem_flush);
      png_set_IHDR(pp, ip, 8, 2, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
      png_set_pHYs(pp, ip, res[i][0], res[i][1], PNG_RESOLUTION_METER);
      png_set_oFFs(pp, ip, -1000000 + i*777777, 999999 - i*333333,
          i == 0 ? PNG_OFFSET_PIXEL : PNG_OFFSET_MICROMETER);
      png_write_info(pp, ip);
      { png_byte r[8]; png_uint_32 y; for (y=0;y<2;++y){ fill(r,8,y); png_write_row(pp,r);} }
      png_write_end(pp, ip);
      {
         png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
         png_infop rip = png_create_info_struct(rp);
         rdbuf rb; rb.buf=m.buf; rb.len=m.len; rb.pos=0;
         if (setjmp(png_jmpbuf(rp)))
         { png_destroy_read_struct(&rp,&rip,NULL); }
         else
         {
            png_set_read_fn(rp, &rb, mem_read);
            png_read_info(rp, rip);
            OUT("[%d] ppm=%u xppm=%u yppm=%u ppi=%u xppi=%u yppi=%u\n", i,
                png_get_pixels_per_meter(rp,rip), png_get_x_pixels_per_meter(rp,rip),
                png_get_y_pixels_per_meter(rp,rip), png_get_pixels_per_inch(rp,rip),
                png_get_x_pixels_per_inch(rp,rip), png_get_y_pixels_per_inch(rp,rip));
            pf("aspect", png_get_pixel_aspect_ratio(rp,rip));
            OUT("aspectfix=%d\n", png_get_pixel_aspect_ratio_fixed(rp,rip));
            OUT("xoffpix=%d yoffpix=%d xoffmic=%d yoffmic=%d\n",
                png_get_x_offset_pixels(rp,rip), png_get_y_offset_pixels(rp,rip),
                png_get_x_offset_microns(rp,rip), png_get_y_offset_microns(rp,rip));
            pf("xoffin", png_get_x_offset_inches(rp,rip));
            pf("yoffin", png_get_y_offset_inches(rp,rip));
            OUT("xoffinfix=%d yoffinfix=%d\n",
                png_get_x_offset_inches_fixed(rp,rip),
                png_get_y_offset_inches_fixed(rp,rip));
            { png_uint_32 rx, ry; int ut;
              if (png_get_pHYs_dpi(rp,rip,&rx,&ry,&ut)) OUT("dpi %u %u %d\n", rx,ry,ut); }
            OUT("sig=%p rgbgray=%u\n", (void*)png_get_signature(rp,rip),
                png_get_rgb_to_gray_status(rp));
            png_destroy_read_struct(&rp,&rip,NULL);
         }
      }
      feed(m.buf, m.len);
      png_destroy_write_struct(&pp,&ip);
      free(m.buf);
   }
   OUT("hash=%lu\n", g_hash);
}

/* user transforms, user chunk callback, MNG features */
static void test_callbacks(int mng, int use_utrans)
{
   png_structp pp; png_infop ip; membuf m;
   png_uint_32 w = 16, h = 4, y;
   png_bytep row;
   size_t rowbytes;
   int dummy = 42;

   memset(&m, 0, sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
   ip = png_create_info_struct(pp);
   OUT("--- callbacks mng=%d ut=%d\n", mng, use_utrans);
   if (setjmp(png_jmpbuf(pp)))
   { OUT("cb longjmp\n"); png_destroy_write_struct(&pp,&ip); free(m.buf); return; }
   png_set_write_fn(pp, &m, mem_write, mem_flush);
   if (mng) OUT("mng=%u\n", png_permit_mng_features(pp, PNG_ALL_MNG_FEATURES));
   png_set_IHDR(pp, ip, w, h, 8, PNG_COLOR_TYPE_RGB, 0, 0,
       mng ? PNG_INTRAPIXEL_DIFFERENCING : 0);
   if (use_utrans)
   {
      png_set_write_user_transform_fn(pp, utrans);
      png_set_user_transform_info(pp, &dummy, 8, 3);
   }
   png_set_write_status_fn(pp, wstat);
   OUT("cbsize=%zu\n", png_get_compression_buffer_size(pp));
   png_set_compression_buffer_size(pp, 512);
   OUT("cbsize2=%zu\n", png_get_compression_buffer_size(pp));
   png_write_info(pp, ip);
   rowbytes = png_get_rowbytes(pp, ip);
   row = malloc(rowbytes);
   for (y = 0; y < h; ++y) { fill(row, rowbytes, y+3); png_write_row(pp, row); }
   free(row);
   png_write_end(pp, ip);
   dump("CBPNG", m.buf, m.len);
   feed(m.buf, m.len);

   {
      png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, err_fn, warn_fn);
      png_infop rip = png_create_info_struct(rp);
      rdbuf rb; rb.buf=m.buf; rb.len=m.len; rb.pos=0;
      if (setjmp(png_jmpbuf(rp)))
      { OUT("cbr longjmp\n"); png_destroy_read_struct(&rp,&rip,NULL); goto done; }
      png_set_read_fn(rp, &rb, mem_read);
      png_set_read_status_fn(rp, rstat);
      png_set_read_user_chunk_fn(rp, &dummy, uchunk);
      if (mng) png_permit_mng_features(rp, PNG_ALL_MNG_FEATURES);
      if (use_utrans)
      {
         png_set_read_user_transform_fn(rp, utrans);
         png_set_user_transform_info(rp, &dummy, 8, 3);
      }
      png_read_info(rp, rip);
      png_read_update_info(rp, rip);
      {
         size_t rbs = png_get_rowbytes(rp, rip);
         png_bytep r = malloc(rbs);
         for (y = 0; y < h; ++y)
         { memset(r,0,rbs); png_read_row(rp, r, NULL); feed(r, rbs); dump("cbrow", r, rbs); }
         free(r);
      }
      png_read_end(rp, rip);
      OUT("reset_zstream=%d\n", png_reset_zstream(rp));
      png_destroy_read_struct(&rp,&rip,NULL);
   }
done:
   png_destroy_write_struct(&pp,&ip);
   free(m.buf);
   OUT("hash=%lu\n", g_hash);
}

/* read-side float transform APIs */
static unsigned char *mk(size_t *lenp, int ct, int bd)
{
   png_structp pp; png_infop ip; membuf m;
   png_uint_32 w=16,h=4,y; size_t rowbytes; png_bytep row;
   memset(&m,0,sizeof m);
   pp = png_create_write_struct(PNG_LIBPNG_VER_STRING,NULL,err_fn,warn_fn);
   ip = png_create_info_struct(pp);
   if (setjmp(png_jmpbuf(pp))) { *lenp=0; return NULL; }
   png_set_write_fn(pp,&m,mem_write,mem_flush);
   png_set_IHDR(pp,ip,w,h,bd,ct,0,0,0);
   if (ct & PNG_COLOR_MASK_PALETTE)
   {
      png_color pal[256]; png_byte tr[256]; int i,n=1<<bd;
      for(i=0;i<n;++i){pal[i].red=(png_byte)(i*5);pal[i].green=(png_byte)(i*9);pal[i].blue=(png_byte)(i*13);tr[i]=(png_byte)(i*4);}
      png_set_PLTE(pp,ip,pal,n); png_set_tRNS(pp,ip,tr,n,NULL);
   }
   else if (!(ct & PNG_COLOR_MASK_ALPHA))
   { png_color_16 t; memset(&t,0,sizeof t); t.red=t.green=t.blue=t.gray=7; png_set_tRNS(pp,ip,NULL,0,&t); }
   png_set_gAMA(pp,ip,0.5);
   { png_color_16 bg; memset(&bg,0,sizeof bg); bg.red=9;bg.green=8;bg.blue=7;bg.gray=6;bg.index=1; png_set_bKGD(pp,ip,&bg); }
   png_write_info(pp,ip);
   rowbytes = png_get_rowbytes(pp,ip);
   row = malloc(rowbytes);
   for(y=0;y<h;++y){ fill(row,rowbytes,y*7+2); png_write_row(pp,row); }
   free(row);
   png_write_end(pp,ip);
   png_destroy_write_struct(&pp,&ip);
   *lenp=m.len; return m.buf;
}

static void test_read_float(const unsigned char *png, size_t len, int which)
{
   png_structp rp = png_create_read_struct(PNG_LIBPNG_VER_STRING,NULL,err_fn,warn_fn);
   png_infop ip = png_create_info_struct(rp);
   rdbuf rb; rb.buf=png; rb.len=len; rb.pos=0;
   png_uint_32 h, y; size_t rbs; png_bytep r;
   OUT("--- read_float %d\n", which);
   if (setjmp(png_jmpbuf(rp)))
   { OUT("rf longjmp\n"); png_destroy_read_struct(&rp,&ip,NULL); return; }
   png_set_read_fn(rp,&rb,mem_read);
   png_read_info(rp,ip);
   switch (which)
   {
      case 0: png_set_expand(rp); png_set_gamma(rp, 2.2, 0.45455); break;
      case 1:
      { png_color_16 bg; memset(&bg,0,sizeof bg);
        bg.red=200;bg.green=100;bg.blue=50;bg.gray=128;
        png_set_expand(rp);
        png_set_background(rp, &bg, PNG_BACKGROUND_GAMMA_UNIQUE, 0, 1.8);
        png_set_gamma(rp, 2.2, 0.45455); break; }
      case 2: png_set_expand(rp);
              png_set_rgb_to_gray(rp, PNG_ERROR_ACTION_WARN, 0.2125, 0.7154);
              break;
      case 3: png_set_expand_16(rp);
              png_set_alpha_mode(rp, PNG_ALPHA_OPTIMIZED, 1.0); break;
      case 4: png_set_expand(rp);
              png_set_alpha_mode(rp, PNG_ALPHA_BROKEN, 2.2); break;
      case 5: png_set_expand(rp); png_set_add_alpha(rp, 0x7f, PNG_FILLER_BEFORE);
              png_set_expand_gray_1_2_4_to_8(rp); break;
      case 6: png_set_palette_to_rgb(rp); png_set_tRNS_to_alpha(rp);
              png_set_strip_alpha(rp); break;
      default: break;
   }
   png_read_update_info(rp, ip);
   h = png_get_image_height(rp, ip);
   rbs = png_get_rowbytes(rp, ip);
   OUT("rf rb=%zu ct=%d bd=%d ch=%d rgbgray=%u\n", rbs,
       png_get_color_type(rp,ip), png_get_bit_depth(rp,ip),
       png_get_channels(rp,ip), png_get_rgb_to_gray_status(rp));
   r = malloc(rbs);
   for (y = 0; y < h; ++y)
   { memset(r,0,rbs); png_read_row(rp,r,NULL); feed(r,rbs); dump("rfrow", r, rbs); }
   free(r);
   png_read_end(rp, NULL);
   png_destroy_read_struct(&rp,&ip,NULL);
   OUT("hash=%lu\n", g_hash);
}

/* free_data / data_freer / set_invalid / handle_as_unknown */
static void test_info_mgmt(void)
{
   png_structp pp = png_create_write_struct(PNG_LIBPNG_VER_STRING,NULL,err_fn,warn_fn);
   png_infop ip = png_create_info_struct(pp);
   static const png_byte list[] = { 'p','r','V','t',PNG_HANDLE_CHUNK_ALWAYS,
                                    'q','r','S','t',PNG_HANDLE_CHUNK_NEVER,
                                    'b','K','G','D',PNG_HANDLE_CHUNK_IF_SAFE };
   OUT("--- info_mgmt\n");
   if (setjmp(png_jmpbuf(pp)))
   { OUT("im longjmp\n"); png_destroy_write_struct(&pp,&ip); return; }
   png_set_IHDR(pp, ip, 8, 2, 8, PNG_COLOR_TYPE_PALETTE, 0, 0, 0);
   { png_color pal[256]; int i;
     for(i=0;i<256;++i){pal[i].red=(png_byte)i;pal[i].green=(png_byte)i;pal[i].blue=(png_byte)i;}
     png_set_PLTE(pp, ip, pal, 256); }
   { png_uint_16 hist[256]; int i;
     for(i=0;i<256;++i) hist[i]=(png_uint_16)i;
     png_set_hIST(pp, ip, hist); }
   { png_text t; memset(&t,0,sizeof t);
     t.compression=PNG_TEXT_COMPRESSION_NONE; t.key=(png_charp)"a"; t.text=(png_charp)"b";
     png_set_text(pp, ip, &t, 1); }
   OUT("valid1=%08x\n", (unsigned)png_get_valid(pp, ip, 0xffffffffu));
   png_set_keep_unknown_chunks(pp, PNG_HANDLE_CHUNK_ALWAYS, list, 3);
   OUT("hau prVt=%d qrSt=%d bKGD=%d IHDR=%d\n",
       png_handle_as_unknown(pp, (png_const_bytep)"prVt"),
       png_handle_as_unknown(pp, (png_const_bytep)"qrSt"),
       png_handle_as_unknown(pp, (png_const_bytep)"bKGD"),
       png_handle_as_unknown(pp, (png_const_bytep)"IHDR"));
   png_set_invalid(pp, ip, PNG_INFO_hIST);
   OUT("valid2=%08x\n", (unsigned)png_get_valid(pp, ip, 0xffffffffu));
   png_data_freer(pp, ip, PNG_USER_WILL_FREE_DATA, PNG_FREE_TEXT);
   png_data_freer(pp, ip, PNG_DESTROY_WILL_FREE_DATA, PNG_FREE_TEXT);
   png_free_data(pp, ip, PNG_FREE_HIST, -1);
   png_free_data(pp, ip, PNG_FREE_TEXT, -1);
   OUT("valid3=%08x\n", (unsigned)png_get_valid(pp, ip, 0xffffffffu));
   png_destroy_write_struct(&pp, &ip);
   OUT("hash=%lu\n", g_hash);
}

/* deprecated / niche entry points */
static void test_niche(void)
{
   png_structp pp = png_create_write_struct(PNG_LIBPNG_VER_STRING,NULL,err_fn,warn_fn);
   png_infop ip = png_create_info_struct(pp);
   png_time t;
   OUT("--- niche\n");
   if (setjmp(png_jmpbuf(pp)))
   { OUT("n longjmp\n"); png_destroy_write_struct(&pp,&ip); return; }
   memset(&t, 0, sizeof t);
   t.year = 2001; t.month = 1; t.day = 2; t.hour = 3; t.minute = 4; t.second = 5;
   OUT("rfc1123=%s\n", png_convert_to_rfc1123(pp, &t));
   png_set_filter_heuristics(pp, PNG_FILTER_HEURISTIC_DEFAULT, 0, NULL, NULL);
   png_set_filter_heuristics_fixed(pp, PNG_FILTER_HEURISTIC_DEFAULT, 0, NULL, NULL);
   /* unknown chunk location fiddling */
   {
      png_unknown_chunk u[2];
      memset(u, 0, sizeof u);
      memcpy(u[0].name, "prVt", 5);
      u[0].data = (png_bytep)"aaaa"; u[0].size = 4; u[0].location = PNG_HAVE_IHDR;
      memcpy(u[1].name, "qrSt", 5);
      u[1].data = (png_bytep)"bbbbbb"; u[1].size = 6; u[1].location = PNG_AFTER_IDAT;
      png_set_IHDR(pp, ip, 8, 2, 8, PNG_COLOR_TYPE_GRAY, 0, 0, 0);
      png_set_unknown_chunks(pp, ip, u, 2);
      png_set_unknown_chunk_location(pp, ip, 0, PNG_HAVE_PLTE);
      png_set_unknown_chunk_location(pp, ip, 1, PNG_HAVE_IHDR);
      {
         png_unknown_chunkp got;
         int n = png_get_unknown_chunks(pp, ip, &got), i;
         for (i = 0; i < n; ++i)
            OUT("unk %s %zu loc=%u\n", (char*)got[i].name, got[i].size,
                got[i].location);
      }
   }
   png_destroy_write_struct(&pp, &ip);
   OUT("hash=%lu\n", g_hash);
}

int main(void)
{
   static const int cts[] = { 0, 3, 2, 4, 6 };
   static const int bds[] = { 1, 2, 4, 8, 16 };
   size_t ci, bi;
   int i;

   g_hash = 5381;
   test_float_setters();
   test_easy_access();
   test_callbacks(0, 0);
   test_callbacks(0, 1);
   test_callbacks(1, 0);
   test_callbacks(1, 1);
   test_info_mgmt();
   test_niche();

   for (ci = 0; ci < 5; ++ci)
      for (bi = 0; bi < 5; ++bi)
      {
         int ct = cts[ci], bd = bds[bi];
         size_t len;
         unsigned char *png;
         if (ct == 3 && bd == 16) continue;
         if (ct != 0 && ct != 3 && bd < 8) continue;
         png = mk(&len, ct, bd);
         if (png == NULL) continue;
         OUT("=== RF ct=%d bd=%d len=%zu\n", ct, bd, len);
         for (i = 0; i <= 7; ++i) test_read_float(png, len, i);
         free(png);
      }

   OUT("FINAL hash=%lu\n", g_hash);
   return 0;
}
