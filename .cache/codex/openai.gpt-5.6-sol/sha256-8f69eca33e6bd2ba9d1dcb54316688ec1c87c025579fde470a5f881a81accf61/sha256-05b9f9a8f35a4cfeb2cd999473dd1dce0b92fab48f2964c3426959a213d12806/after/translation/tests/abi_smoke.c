#include <png.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int write_image(const char *path)
{
   png_image image;
   png_byte pixels[19 * 13 * 4];
   size_t i;

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   image.width = 19;
   image.height = 13;
   image.format = PNG_FORMAT_RGBA;

   for (i = 0; i < sizeof pixels; ++i)
      pixels[i] = (png_byte)((i * 73U + (i >> 2) * 29U + 17U) & 255U);

   if (!png_image_write_to_file(&image, path, 0, pixels, 0, NULL))
   {
      fprintf(stderr, "%s\n", image.message);
      return 1;
   }

   return 0;
}

static int read_image(const char *input, const char *output)
{
   png_image image;
   png_bytep pixels;
   FILE *file;
   size_t size;

   memset(&image, 0, sizeof image);
   image.version = PNG_IMAGE_VERSION;
   if (!png_image_begin_read_from_file(&image, input))
   {
      fprintf(stderr, "%s\n", image.message);
      return 1;
   }

   image.format = PNG_FORMAT_RGBA;
   size = PNG_IMAGE_SIZE(image);
   pixels = malloc(size);
   if (pixels == NULL)
      return 1;

   if (!png_image_finish_read(&image, NULL, pixels, 0, NULL))
   {
      fprintf(stderr, "%s\n", image.message);
      free(pixels);
      return 1;
   }

   file = fopen(output, "wb");
   if (file == NULL || fwrite(pixels, 1, size, file) != size)
   {
      free(pixels);
      if (file != NULL)
         fclose(file);
      return 1;
   }

   free(pixels);
   return fclose(file) != 0;
}

int main(int argc, char **argv)
{
   if (argc == 3 && strcmp(argv[1], "write") == 0)
      return write_image(argv[2]);
   if (argc == 4 && strcmp(argv[1], "read") == 0)
      return read_image(argv[2], argv[3]);

   return 2;
}
