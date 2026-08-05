/* Test harness: drives a FULL libpng write+read roundtrip against a shared
 * library loaded at runtime (either the reference C libpng.so or the Rust
 * liblibpng.so). All setjmp/longjmp lives in this C file, which is how libpng
 * is designed to be driven; running the whole operation in C avoids unwinding
 * a longjmp across Rust frames (which would be UB). The Rust test loads THIS
 * harness .so via libloading and calls harness_roundtrip(), passing the path
 * to the libpng under test. This still exercises the Rust `.so`'s #[no_mangle]
 * exports because the harness dlopen()s that library and calls its symbols.
 *
 * This file is NOT part of the library under test.
 */
#include <setjmp.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <dlfcn.h>
#include <stdint.h>

/* ---- libpng public constants we need (stable ABI) ---- */
#define PNG_COLOR_TYPE_GRAY       0
#define PNG_COLOR_TYPE_PALETTE    3
#define PNG_COLOR_TYPE_RGB        2
#define PNG_COLOR_TYPE_RGB_ALPHA  6
#define PNG_COLOR_TYPE_GRAY_ALPHA 4
#define PNG_INTERLACE_NONE   0
#define PNG_INTERLACE_ADAM7  1
#define PNG_COMPRESSION_TYPE_BASE 0
#define PNG_FILTER_TYPE_BASE 0

/* png_color = 3 bytes */
typedef struct { unsigned char red, green, blue; } h_png_color;

/* ---- error handling ---- */
static __thread jmp_buf g_env;
static __thread char g_msg[256];
static __thread int g_active;

static void h_error(void *png_ptr, const char *msg) {
    (void)png_ptr;
    if (msg) { strncpy(g_msg, msg, sizeof g_msg - 1); g_msg[sizeof g_msg - 1] = 0; }
    if (g_active) longjmp(g_env, 1);
    /* if not active, we cannot return from png_error; abort deterministically */
    abort();
}
static void h_warn(void *png_ptr, const char *msg) { (void)png_ptr; (void)msg; }

/* ---- growable byte buffer used as the PNG I/O sink/source ---- */
typedef struct {
    unsigned char *data;
    size_t len;
    size_t cap;
    size_t rpos;
} membuf;

static void mb_write(void *png_ptr, unsigned char *data, size_t length);
static void mb_read(void *png_ptr, unsigned char *data, size_t length);
static void mb_flush(void *png_ptr);

/* function-pointer typedefs for the libpng symbols we resolve */
typedef void *(*fp_create)(const char *, void *, void *, void *);
typedef void *(*fp_create_info)(void *);
typedef void (*fp_set_wfn)(void *, void *, void *, void *);
typedef void (*fp_set_rfn)(void *, void *, void *);
typedef void (*fp_set_IHDR)(void *, void *, uint32_t, uint32_t, int, int, int, int, int);
typedef uint32_t (*fp_get_IHDR)(void *, void *, uint32_t *, uint32_t *, int *, int *, int *, int *, int *);
typedef void (*fp_write_info)(void *, void *);
typedef void (*fp_read_info)(void *, void *);
typedef void (*fp_write_image)(void *, unsigned char **);
typedef void (*fp_read_image)(void *, unsigned char **);
typedef void (*fp_write_end)(void *, void *);
typedef void (*fp_read_end)(void *, void *);
typedef void (*fp_destroy_write)(void **, void **);
typedef void (*fp_destroy_read)(void **, void **, void **);
typedef void (*fp_set_PLTE)(void *, void *, void *, int);
typedef size_t (*fp_get_rowbytes)(void *, void *);
typedef void (*fp_set_filter)(void *, int, int);
typedef void (*fp_set_compression_level)(void *, int);
typedef void (*fp_set_gAMA_fixed)(void *, void *, uint32_t);
typedef void (*fp_set_pHYs)(void *, void *, uint32_t, uint32_t, int);

typedef struct {
    void *lib;
    fp_create create_write, create_read;
    fp_create_info create_info;
    fp_set_wfn set_write_fn;
    fp_set_rfn set_read_fn;
    fp_set_IHDR set_IHDR;
    fp_get_IHDR get_IHDR;
    fp_write_info write_info;
    fp_read_info read_info;
    fp_write_image write_image;
    fp_read_image read_image;
    fp_write_end write_end;
    fp_read_end read_end;
    fp_destroy_write destroy_write;
    fp_destroy_read destroy_read;
    fp_set_PLTE set_PLTE;
    fp_get_rowbytes get_rowbytes;
    fp_set_filter set_filter;
    fp_set_compression_level set_compression_level;
    fp_set_pHYs set_pHYs;
} pnglib;

#define VER "1.6.59.git"

static int load_lib(const char *path, pnglib *L) {
    memset(L, 0, sizeof *L);
    /* Ensure libpng's dependencies (zlib, libm) are resolvable when we dlopen
     * the reference C libpng.so, which does not itself DT_NEEDED them in a way
     * that satisfies RTLD_NOW here. Load them GLOBAL first. */
    dlopen("libm.so.6", RTLD_NOW | RTLD_GLOBAL);
    dlopen("libz.so.1", RTLD_NOW | RTLD_GLOBAL);
    L->lib = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!L->lib) return -1;
    L->create_write = (fp_create)dlsym(L->lib, "png_create_write_struct");
    L->create_read  = (fp_create)dlsym(L->lib, "png_create_read_struct");
    L->create_info  = (fp_create_info)dlsym(L->lib, "png_create_info_struct");
    L->set_write_fn = (fp_set_wfn)dlsym(L->lib, "png_set_write_fn");
    L->set_read_fn  = (fp_set_rfn)dlsym(L->lib, "png_set_read_fn");
    L->set_IHDR     = (fp_set_IHDR)dlsym(L->lib, "png_set_IHDR");
    L->get_IHDR     = (fp_get_IHDR)dlsym(L->lib, "png_get_IHDR");
    L->write_info   = (fp_write_info)dlsym(L->lib, "png_write_info");
    L->read_info    = (fp_read_info)dlsym(L->lib, "png_read_info");
    L->write_image  = (fp_write_image)dlsym(L->lib, "png_write_image");
    L->read_image   = (fp_read_image)dlsym(L->lib, "png_read_image");
    L->write_end    = (fp_write_end)dlsym(L->lib, "png_write_end");
    L->read_end     = (fp_read_end)dlsym(L->lib, "png_read_end");
    L->destroy_write= (fp_destroy_write)dlsym(L->lib, "png_destroy_write_struct");
    L->destroy_read = (fp_destroy_read)dlsym(L->lib, "png_destroy_read_struct");
    L->set_PLTE     = (fp_set_PLTE)dlsym(L->lib, "png_set_PLTE");
    L->get_rowbytes = (fp_get_rowbytes)dlsym(L->lib, "png_get_rowbytes");
    L->set_filter   = (fp_set_filter)dlsym(L->lib, "png_set_filter");
    L->set_compression_level = (fp_set_compression_level)dlsym(L->lib, "png_set_compression_level");
    L->set_pHYs     = (fp_set_pHYs)dlsym(L->lib, "png_set_pHYs");
    if (!L->create_write || !L->create_read || !L->create_info ||
        !L->set_write_fn || !L->set_read_fn || !L->set_IHDR || !L->get_IHDR ||
        !L->write_info || !L->read_info || !L->write_image || !L->read_image ||
        !L->write_end || !L->read_end || !L->destroy_write || !L->destroy_read ||
        !L->set_PLTE || !L->get_rowbytes)
        return -2;
    return 0;
}

static __thread membuf *g_cur_io; /* io_ptr for callbacks */

static void mb_ensure(membuf *m, size_t extra) {
    if (m->len + extra > m->cap) {
        size_t nc = m->cap ? m->cap * 2 : 4096;
        while (nc < m->len + extra) nc *= 2;
        m->data = (unsigned char *)realloc(m->data, nc);
        m->cap = nc;
    }
}
static void mb_write(void *png_ptr, unsigned char *data, size_t length) {
    (void)png_ptr;
    membuf *m = g_cur_io;
    mb_ensure(m, length);
    memcpy(m->data + m->len, data, length);
    m->len += length;
}
static void mb_read(void *png_ptr, unsigned char *data, size_t length) {
    (void)png_ptr;
    membuf *m = g_cur_io;
    if (m->rpos + length > m->len) {
        /* underflow: fill with zero (should not happen for valid streams) */
        memset(data, 0, length);
        return;
    }
    memcpy(data, m->data + m->rpos, length);
    m->rpos += length;
}
static void mb_flush(void *png_ptr) { (void)png_ptr; }

/* Parameters describing the image to round-trip. */
typedef struct {
    uint32_t width, height;
    int bit_depth, color_type, interlace;
    int filters;            /* value passed to png_set_filter, -1 to skip */
    int compression_level;  /* -1 to skip */
    int use_gamma;          /* unused placeholder */
    int use_phys;
} img_params;

/* Result: encoded stream + decoded rows, so the Rust side can compare both
 * libraries' encoded bytes and decoded pixels. */

/* Encode: returns 0 ok, non-zero on png_error; fills *out (malloc'd). */
static int do_encode(pnglib *L, const img_params *p,
                     unsigned char **rows, size_t rowbytes,
                     h_png_color *palette, int num_palette,
                     membuf *out) {
    void *png = L->create_write(VER, 0, (void *)h_error, (void *)h_warn);
    if (!png) return 100;
    void *info = L->create_info(png);
    if (!info) { L->destroy_write(&png, 0); return 101; }

    g_active = 1;
    if (setjmp(g_env)) {
        g_active = 0;
        L->destroy_write(&png, &info);
        return 1;
    }
    g_cur_io = out;
    L->set_write_fn(png, out, (void *)mb_write, (void *)mb_flush);
    L->set_IHDR(png, info, p->width, p->height, p->bit_depth, p->color_type,
                p->interlace, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
    if (p->color_type == PNG_COLOR_TYPE_PALETTE)
        L->set_PLTE(png, info, palette, num_palette);
    if (p->use_phys && L->set_pHYs)
        L->set_pHYs(png, info, 2835, 2835, 1);
    if (p->compression_level >= 0 && L->set_compression_level)
        L->set_compression_level(png, p->compression_level);
    if (p->filters >= 0 && L->set_filter)
        L->set_filter(png, 0, p->filters);
    L->write_info(png, info);
    L->write_image(png, rows);
    L->write_end(png, info);
    g_active = 0;
    L->destroy_write(&png, &info);
    return 0;
}

/* Decode: returns 0 ok. Fills decoded row pointers (preallocated by caller as
 * out_rows[height], each get_rowbytes long -- but rowbytes discovered from the
 * stream; caller must allocate generously). */
static int do_decode(pnglib *L, membuf *in,
                     uint32_t *out_w, uint32_t *out_h,
                     int *out_bd, int *out_ct, int *out_il,
                     size_t *out_rowbytes,
                     unsigned char **out_rows, size_t out_row_cap) {
    void *png = L->create_read(VER, 0, (void *)h_error, (void *)h_warn);
    if (!png) return 100;
    void *info = L->create_info(png);
    if (!info) { L->destroy_read(&png, 0, 0); return 101; }

    g_active = 1;
    if (setjmp(g_env)) {
        g_active = 0;
        L->destroy_read(&png, &info, 0);
        return 1;
    }
    in->rpos = 0;
    g_cur_io = in;
    L->set_read_fn(png, in, (void *)mb_read);
    L->read_info(png, info);
    uint32_t w = 0, h = 0; int bd = 0, ct = 0, il = 0, cm = 0, fm = 0;
    L->get_IHDR(png, info, &w, &h, &bd, &ct, &il, &cm, &fm);
    size_t rb = L->get_rowbytes(png, info);
    if (rb > out_row_cap) { g_active = 0; L->destroy_read(&png, &info, 0); return 50; }
    *out_w = w; *out_h = h; *out_bd = bd; *out_ct = ct; *out_il = il;
    *out_rowbytes = rb;
    L->read_image(png, out_rows);
    L->read_end(png, info);
    g_active = 0;
    L->destroy_read(&png, &info, 0);
    return 0;
}

const char *harness_msg(void) { return g_msg; }

/* Decode an arbitrary raw byte buffer as a PNG datastream. Reports whether a
 * png_error fired (via longjmp) and copies the error message out. This drives
 * the read side directly for Phase C error-path differential tests.
 *
 * Returns: 0 = decode completed with no png_error; 1 = png_error fired.
 * *out_msg_buf receives the error/last message (up to msg_cap-1 bytes).
 * *out_w/h/... receive the IHDR if it was read before any error.
 */
int harness_decode_raw(const char *lib_path,
                       const unsigned char *stream, size_t stream_len,
                       char *out_msg_buf, size_t msg_cap,
                       uint32_t *out_w, uint32_t *out_h,
                       int *out_bd, int *out_ct) {
    pnglib L;
    if (load_lib(lib_path, &L) != 0) return 200;

    /* copy stream into a membuf so mb_read can consume it */
    membuf in; memset(&in, 0, sizeof in);
    in.data = (unsigned char *)malloc(stream_len ? stream_len : 1);
    memcpy(in.data, stream, stream_len);
    in.len = stream_len;
    in.rpos = 0;

    void *png = L.create_read(VER, 0, (void *)h_error, (void *)h_warn);
    if (!png) { free(in.data); dlclose(L.lib); return 100; }
    void *info = L.create_info(png);
    if (!info) { L.destroy_read(&png, 0, 0); free(in.data); dlclose(L.lib); return 101; }

    if (out_w) *out_w = 0;
    if (out_h) *out_h = 0;
    if (out_bd) *out_bd = 0;
    if (out_ct) *out_ct = 0;
    g_msg[0] = 0;

    int fired = 0;
    g_active = 1;
    if (setjmp(g_env)) {
        fired = 1;
    } else {
        g_cur_io = &in;
        L.set_read_fn(png, &in, (void *)mb_read);
        L.read_info(png, info);
        uint32_t w = 0, h = 0; int bd = 0, ct = 0, il = 0, cm = 0, fm = 0;
        L.get_IHDR(png, info, &w, &h, &bd, &ct, &il, &cm, &fm);
        if (out_w) *out_w = w;
        if (out_h) *out_h = h;
        if (out_bd) *out_bd = bd;
        if (out_ct) *out_ct = ct;
        size_t rb = L.get_rowbytes(png, info);
        /* read the image row-by-row into a scratch buffer */
        if (h > 0 && rb > 0) {
            unsigned char *scratch = (unsigned char *)malloc(rb);
            unsigned char **rows = (unsigned char **)malloc(sizeof(void *) * h);
            /* read_image expects an array of row pointers; reuse scratch for all
             * rows (we don't validate pixels here, only that decode succeeds). */
            for (uint32_t i = 0; i < h; i++) rows[i] = scratch;
            /* Note: read_image writes each row; using the same buffer is fine
             * because we discard the pixels. */
            L.read_image(png, rows);
            free(rows);
            free(scratch);
        }
        L.read_end(png, info);
    }
    g_active = 0;

    if (out_msg_buf && msg_cap > 0) {
        strncpy(out_msg_buf, g_msg, msg_cap - 1);
        out_msg_buf[msg_cap - 1] = 0;
    }

    L.destroy_read(&png, &info, 0);
    free(in.data);
    dlclose(L.lib);
    return fired;
}

/* Full roundtrip run against ONE library path.
 * Inputs: params + flattened source pixel rows (row-major, rowbytes each).
 * Outputs (caller-allocated):
 *   enc_out / enc_len : pointer to malloc'd encoded stream + its length
 *   dec_rows_flat     : decoded pixels (height*rowbytes), caller-allocated
 *   returns 0 ok, non-zero on failure (encode/decode error stage encoded in
 *   return value: 1=encode err, 2=decode err, other=setup).
 */
int harness_roundtrip(const char *lib_path,
                      uint32_t width, uint32_t height,
                      int bit_depth, int color_type, int interlace,
                      int filters, int compression_level, int use_phys,
                      const unsigned char *src_flat, size_t src_rowbytes,
                      const unsigned char *palette_flat, int num_palette,
                      unsigned char **enc_out, size_t *enc_len,
                      unsigned char *dec_rows_flat, size_t dec_row_cap,
                      uint32_t *dec_w, uint32_t *dec_h,
                      int *dec_bd, int *dec_ct, int *dec_il,
                      size_t *dec_rowbytes) {
    pnglib L;
    if (load_lib(lib_path, &L) != 0) return 200;

    img_params p;
    p.width = width; p.height = height; p.bit_depth = bit_depth;
    p.color_type = color_type; p.interlace = interlace;
    p.filters = filters; p.compression_level = compression_level;
    p.use_gamma = 0; p.use_phys = use_phys;

    /* build row pointer array for source */
    unsigned char **rows = (unsigned char **)malloc(sizeof(void *) * height);
    for (uint32_t i = 0; i < height; i++)
        rows[i] = (unsigned char *)(uintptr_t)(src_flat + (size_t)i * src_rowbytes);

    h_png_color *pal = 0;
    if (num_palette > 0) {
        pal = (h_png_color *)malloc(sizeof(h_png_color) * num_palette);
        for (int i = 0; i < num_palette; i++) {
            pal[i].red = palette_flat[i * 3 + 0];
            pal[i].green = palette_flat[i * 3 + 1];
            pal[i].blue = palette_flat[i * 3 + 2];
        }
    }

    membuf out; memset(&out, 0, sizeof out);
    int erc = do_encode(&L, &p, rows, src_rowbytes, pal, num_palette, &out);
    free(rows);
    if (erc != 0) { free(pal); free(out.data); dlclose(L.lib); return 1; }

    /* copy encoded stream out */
    *enc_out = (unsigned char *)malloc(out.len);
    memcpy(*enc_out, out.data, out.len);
    *enc_len = out.len;

    /* decode */
    unsigned char **drows = (unsigned char **)malloc(sizeof(void *) * height);
    for (uint32_t i = 0; i < height; i++)
        drows[i] = dec_rows_flat + (size_t)i * (dec_row_cap / (height ? height : 1));
    size_t per_row = dec_row_cap / (height ? height : 1);
    int drc = do_decode(&L, &out, dec_w, dec_h, dec_bd, dec_ct, dec_il,
                        dec_rowbytes, drows, per_row);
    free(drows);
    free(pal);
    free(out.data);
    dlclose(L.lib);
    if (drc != 0) return 2;
    return 0;
}
