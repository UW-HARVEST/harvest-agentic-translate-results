#include <stdint.h>
typedef struct cp_pixel_t cp_pixel_t;
struct cp_pixel_t {
  uint8_t r;
  uint8_t g;
  uint8_t b;
  uint8_t a;
};
void convert_pix(int bpp, int w, int h, uint8_t *src, cp_pixel_t *dst);
