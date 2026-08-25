#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "png.h"

#define LOAD(handle, name)                                                      \
   ((__typeof__(&name))load_symbol((handle), #name))

static void *
load_symbol(void *handle, const char *name)
{
   void *symbol = dlsym(handle, name);
   if (symbol == NULL)
   {
      fprintf(stderr, "dlsym(%s): %s\n", name, dlerror());
      exit(2);
   }
   return symbol;
}

static void
write_bytes(const void *data, size_t size)
{
   if (fwrite(data, 1, size, stdout) != size)
      exit(3);
}

int
main(int argc, char **argv)
{
   static const png_byte pixels[] = {
      255,   0,   0, 255,   0, 255,   0, 192,   0,   0, 255, 128,
       12,  34,  56,  78,  90, 123, 210, 255, 255, 255, 255,   0
   };
   static const png_byte integers[] = {
      0x89, 0xab, 0xcd, 0xef, 0x12, 0x34
   };
   png_image image;
   png_image decoded;
   png_byte *encoded;
   png_byte decoded_pixels[sizeof pixels];
   size_t encoded_size = 0;
   void *handle;
   png_uint_32 scalar_results[5];
   int status;

   if (argc != 2)
      return 64;

   handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
   if (handle == NULL)
   {
      fprintf(stderr, "dlopen: %s\n", dlerror());
      return 2;
   }

   scalar_results[0] = LOAD(handle, png_access_version_number)();
   scalar_results[1] = LOAD(handle, png_get_uint_32)(integers);
   scalar_results[2] = (png_uint_32)LOAD(handle, png_get_int_32)(integers);
   scalar_results[3] = LOAD(handle, png_get_uint_16)(integers + 4);
   scalar_results[4] = (png_uint_32)LOAD(handle, png_sig_cmp)(
       (png_const_bytep)"\x89PNG\r\n\x1a\n", 0, 8);
   write_bytes(scalar_results, sizeof scalar_results);

   write_bytes(LOAD(handle, png_get_libpng_ver)(NULL),
       strlen(LOAD(handle, png_get_libpng_ver)(NULL)) + 1);
   write_bytes(load_symbol(handle, "png_sRGB_table"), 512);
   write_bytes(load_symbol(handle, "png_sRGB_base"), 1024);
   write_bytes(load_symbol(handle, "png_sRGB_delta"), 512);

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = 3;
   image.height = 2;
   image.format = PNG_FORMAT_RGBA;

   status = LOAD(handle, png_image_write_to_memory)(
       &image, NULL, &encoded_size, 0, pixels, 0, NULL);
   if (status == 0)
   {
      fprintf(stderr, "size write: %s\n", image.message);
      return 4;
   }

   encoded = malloc(encoded_size);
   if (encoded == NULL)
      return 5;

   status = LOAD(handle, png_image_write_to_memory)(
       &image, encoded, &encoded_size, 0, pixels, 0, NULL);
   if (status == 0)
   {
      fprintf(stderr, "memory write: %s\n", image.message);
      return 6;
   }

   write_bytes(&encoded_size, sizeof encoded_size);
   write_bytes(encoded, encoded_size);

   memset(&decoded, 0, sizeof decoded);
   decoded.version = PNG_IMAGE_VERSION;
   status = LOAD(handle, png_image_begin_read_from_memory)(
       &decoded, encoded, encoded_size);
   if (status == 0)
   {
      fprintf(stderr, "begin read: %s\n", decoded.message);
      return 7;
   }

   decoded.format = PNG_FORMAT_RGBA;
   memset(decoded_pixels, 0xa5, sizeof decoded_pixels);
   status = LOAD(handle, png_image_finish_read)(
       &decoded, NULL, decoded_pixels, 0, NULL);
   if (status == 0)
   {
      fprintf(stderr, "finish read: %s\n", decoded.message);
      return 8;
   }

   write_bytes(&decoded.width, sizeof decoded.width);
   write_bytes(&decoded.height, sizeof decoded.height);
   write_bytes(&decoded.format, sizeof decoded.format);
   write_bytes(decoded_pixels, sizeof decoded_pixels);
   LOAD(handle, png_image_free)(&decoded);

   free(encoded);
   dlclose(handle);
   return 0;
}
