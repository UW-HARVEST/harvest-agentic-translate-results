#include <string.h>
#include <stdint.h>
#include <stdlib.h>
#include <limits.h>
#include <assert.h>
#include <stdio.h>

#include "lib.h"

typedef struct cp_pixel_t cp_pixel_t;
typedef struct cp_image_t cp_image_t;
struct cp_pixel_t {
  uint8_t r;
  uint8_t g;
  uint8_t b;
  uint8_t a;
};
struct cp_image_t {
  int w;
  int h;
  cp_pixel_t *pix;
};

static cp_pixel_t cp_make_pixel_a(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
  cp_pixel_t p;
  p.r = r;
  p.g = g;
  p.b = b;
  p.a = a;
  return p;
}
static cp_pixel_t cp_make_pixel(uint8_t r, uint8_t g, uint8_t b) {
  cp_pixel_t p;
  p.r = r;
  p.g = g;
  p.b = b;
  p.a = 0xFF;
  return p;
}
const char *cp_error_reason;
uint8_t cp_fixed_table[288 + 32] = {
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
};
uint8_t cp_permutation_order[19] = {16, 17, 18, 0, 8,  7, 9,  6, 10, 5,
                                    11, 4,  12, 3, 13, 2, 14, 1, 15};
uint8_t cp_len_extra_bits[29 + 2] = {0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1,
                                     1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4,
                                     4, 4, 5, 5, 5, 5, 0, 0, 0};
uint32_t cp_len_base[29 + 2] = {
    3,  4,  5,  6,  7,  8,  9,  10,  11,  13,  15,  17,  19,  23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0,  0};
uint8_t cp_dist_extra_bits[30 + 2] = {0,  0,  0,  0,  1,  1,  2,  2,  3, 3, 4,
                                      4,  5,  5,  6,  6,  7,  7,  8,  8, 9, 9,
                                      10, 10, 11, 11, 12, 12, 13, 13, 0, 0};
uint32_t cp_dist_base[30 + 2] = {
    1,    2,    3,    4,    5,    7,     9,     13,    17,  25,   33,
    49,   65,   97,   129,  193,  257,   385,   513,   769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0,   0};
typedef struct cp_state_t {
  uint64_t bits;
  int count;
  uint32_t *words;
  int word_count;
  int word_index;
  int bits_left;
  int final_word_available;
  uint32_t final_word;
  char *out;
  char *out_end;
  char *begin;
  uint16_t lookup[(1 << 9)];
  uint32_t lit[288];
  uint32_t dst[32];
  uint32_t len[19];
  uint32_t nlit;
  uint32_t ndst;
  uint32_t nlen;
} cp_state_t;
static int cp_would_overflow(cp_state_t *s, int num_bits) {
  return (s->bits_left + s->count) - num_bits < 0;
}
static char *cp_ptr(cp_state_t *s) {
  assert(!(s->bits_left & 7));
  return (char *)(s->words + s->word_index) - (s->count / 8);
}
static uint64_t cp_peak_bits(cp_state_t *s, int num_bits_to_read) {
  if (s->count < num_bits_to_read) {
    if (s->word_index < s->word_count) {
      uint32_t word = s->words[s->word_index++];
      s->bits |= (uint64_t)word << s->count;
      s->count += 32;
      assert(s->word_index <= s->word_count);
    } else if (s->final_word_available) {
      uint32_t word = s->final_word;
      s->bits |= (uint64_t)word << s->count;
      s->count += s->bits_left;
      s->final_word_available = 0;
    }
  }
  return s->bits;
}
static uint32_t cp_consume_bits(cp_state_t *s, int num_bits_to_read) {
  assert(s->count >= num_bits_to_read);
  uint32_t bits = s->bits & (((uint64_t)1 << num_bits_to_read) - 1);
  s->bits >>= num_bits_to_read;
  s->count -= num_bits_to_read;
  s->bits_left -= num_bits_to_read;
  return bits;
}
static uint32_t cp_read_bits(cp_state_t *s, int num_bits_to_read) {
  assert(num_bits_to_read <= 32);
  assert(num_bits_to_read >= 0);
  assert(s->bits_left > 0);
  assert(s->count <= 64);
  assert(!cp_would_overflow(s, num_bits_to_read));
  cp_peak_bits(s, num_bits_to_read);
  uint32_t bits = cp_consume_bits(s, num_bits_to_read);
  return bits;
}
static uint32_t cp_rev16(uint32_t a) {
  a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
  a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
  a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
  a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
  return a;
}
static int cp_build(cp_state_t *s, uint32_t *tree, uint8_t *lens,
                    int sym_count) {
  int n, codes[16], first[16], counts[16] = {0};
  for (n = 0; n < sym_count; n++)
    counts[lens[n]]++;
  counts[0] = codes[0] = first[0] = 0;
  for (n = 1; n <= 15; ++n) {
    codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
    first[n] = first[n - 1] + counts[n - 1];
  }
  if (s)
    memset(s->lookup, 0, sizeof(s->lookup));
  for (int i = 0; i < sym_count; ++i) {
    int len = lens[i];
    if (len != 0) {
      assert(len < 16);
      uint32_t code = codes[len]++;
      uint32_t slot = first[len]++;
      tree[slot] = (code << (32 - len)) | (i << 4) | len;
      if (s && len <= 9) {
        int j = cp_rev16(code) >> (16 - len);
        while (j < (1 << 9)) {
          s->lookup[j] = (uint16_t)((len << 9) | i);
          j += (1 << len);
        }
      }
    }
  }
  int max_index = first[15];
  return max_index;
}
static int cp_stored(cp_state_t *s) {
  char *p;
  cp_read_bits(s, s->count & 7);
  uint16_t LEN = (uint16_t)cp_read_bits(s, 16);
  uint16_t NLEN = (uint16_t)cp_read_bits(s, 16);
  do {
    if (!(LEN == (uint16_t)(~NLEN))) {
      cp_error_reason = "Failed to find LEN and NLEN as complements within "
                        "stored (uncompressed) stream.";
      do {
        goto cp_err;
      } while (0);
    }
  } while (0);
  do {
    if (!(s->bits_left / 8 <= (int)LEN)) {
      cp_error_reason = "Stored block extends beyond end of input stream.";
      do {
        goto cp_err;
      } while (0);
    }
  } while (0);
  p = cp_ptr(s);
  memcpy(s->out, p, LEN);
  s->out += LEN;
  return 1;
cp_err:
  return 0;
}
static int cp_fixed(cp_state_t *s) {
  s->nlit = cp_build(s, s->lit, cp_fixed_table, 288);
  s->ndst = cp_build(0, s->dst, cp_fixed_table + 288, 32);
  return 1;
}
static int cp_decode(cp_state_t *s, uint32_t *tree, int hi) {
  uint64_t bits = cp_peak_bits(s, 16);
  uint32_t search = (cp_rev16((uint32_t)bits) << 16) | 0xFFFF;
  int lo = 0;
  while (lo < hi) {
    int guess = (lo + hi) >> 1;
    if (search < tree[guess])
      hi = guess;
    else
      lo = guess + 1;
  }
  uint32_t key = tree[lo - 1];
  uint32_t len = (32 - (key & 0xF));
  assert((search >> len) == (key >> len));
  int code = cp_consume_bits(s, key & 0xF);
  (void)code;
  return (key >> 4) & 0xFFF;
}
static int cp_dynamic(cp_state_t *s) {
  uint8_t lenlens[19] = {0};
  int nlit = 257 + cp_read_bits(s, 5);
  int ndst = 1 + cp_read_bits(s, 5);
  int nlen = 4 + cp_read_bits(s, 4);
  for (int i = 0; i < nlen; ++i)
    lenlens[cp_permutation_order[i]] = (uint8_t)cp_read_bits(s, 3);
  s->nlen = cp_build(0, s->len, lenlens, 19);
  uint8_t lens[288 + 32];
  for (int n = 0; n < nlit + ndst;) {
    int sym = cp_decode(s, s->len, s->nlen);
    switch (sym) {
    case 16:
      for (int i = 3 + cp_read_bits(s, 2); i; --i, ++n)
        lens[n] = lens[n - 1];
      break;
    case 17:
      for (int i = 3 + cp_read_bits(s, 3); i; --i, ++n)
        lens[n] = 0;
      break;
    case 18:
      for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n)
        lens[n] = 0;
      break;
    default:
      lens[n++] = (uint8_t)sym;
      break;
    }
  }
  s->nlit = cp_build(s, s->lit, lens, nlit);
  s->ndst = cp_build(0, s->dst, lens + nlit, ndst);
  return 1;
}
static int cp_block(cp_state_t *s) {
  while (1) {
    int symbol = cp_decode(s, s->lit, s->nlit);
    if (symbol < 256) {
      do {
        if (!(s->out + 1 <= s->out_end)) {
          cp_error_reason =
              "Attempted to overwrite out buffer while outputting a symbol.";
          do {
            goto cp_err;
          } while (0);
        }
      } while (0);
      *s->out = (char)symbol;
      s->out += 1;
    } else if (symbol > 256) {
      symbol -= 257;
      int length =
          cp_read_bits(s, cp_len_extra_bits[symbol]) + cp_len_base[symbol];
      int distance_symbol = cp_decode(s, s->dst, s->ndst);
      int backwards_distance =
          cp_read_bits(s, cp_dist_extra_bits[distance_symbol]) +
          cp_dist_base[distance_symbol];
      do {
        if (!(s->out - backwards_distance >= s->begin)) {
          cp_error_reason = "Attempted to write before out buffer (invalid "
                            "backwards distance).";
          do {
            goto cp_err;
          } while (0);
        }
      } while (0);
      do {
        if (!(s->out + length <= s->out_end)) {
          cp_error_reason =
              "Attempted to overwrite out buffer while outputting a string.";
          do {
            goto cp_err;
          } while (0);
        }
      } while (0);
      char *src = s->out - backwards_distance;
      char *dst = s->out;
      s->out += length;
      switch (backwards_distance) {
      case 1:
        memset(dst, *src, length);
        break;
      default:
        while (length--)
          *dst++ = *src++;
      }
    } else
      break;
  }
  return 1;
cp_err:
  return 0;
}
int pinflate(void *in, int in_bytes, void *out, int out_bytes) {
  cp_state_t *s = (cp_state_t *)calloc(1, sizeof(cp_state_t));
  s->bits = 0;
  s->count = 0;
  s->word_index = 0;
  s->bits_left = in_bytes * 8;
  int first_bytes = (int)((((size_t)in + 3) & ~3) - (size_t)in);
  s->words = (uint32_t *)((char *)in + first_bytes);
  s->word_count = (in_bytes - first_bytes) / 4;
  int last_bytes = ((in_bytes - first_bytes) & 3);
  for (int i = 0; i < first_bytes; ++i)
    s->bits |= (uint64_t)(((uint8_t *)in)[i]) << (i * 8);
  s->final_word_available = last_bytes ? 1 : 0;
  s->final_word = 0;
  for (int i = 0; i < last_bytes; i++)
    s->final_word |= ((uint8_t *)in)[in_bytes - last_bytes + i] << (i * 8);
  s->count = first_bytes * 8;
  s->out = (char *)out;
  s->out_end = s->out + out_bytes;
  s->begin = (char *)out;
  int count = 0;
  int bfinal;
  do {
    bfinal = cp_read_bits(s, 1);
    int btype = cp_read_bits(s, 2);
    switch (btype) {
    case 0:
      do {
        if (!(cp_stored(s)))
          goto cp_err;
      } while (0);
      break;
    case 1:
      cp_fixed(s);
      do {
        if (!(cp_block(s)))
          goto cp_err;
      } while (0);
      break;
    case 2:
      cp_dynamic(s);
      do {
        if (!(cp_block(s)))
          goto cp_err;
      } while (0);
      break;
    case 3:
      do {
        if (!(0)) {
          cp_error_reason = "Detected unknown block type within input stream.";
          do {
            goto cp_err;
          } while (0);
        }
      } while (0);
    }
    ++count;
  } while (!bfinal);
  free(s);
  return 1;
cp_err:
  free(s);
  return 0;
}
