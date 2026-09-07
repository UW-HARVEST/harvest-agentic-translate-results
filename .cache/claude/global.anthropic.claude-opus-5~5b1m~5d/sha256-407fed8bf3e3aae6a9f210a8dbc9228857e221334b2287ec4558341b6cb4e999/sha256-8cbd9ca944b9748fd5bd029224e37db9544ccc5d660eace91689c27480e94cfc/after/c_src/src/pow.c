// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#include "pow.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

// Takes two arguments, a base and an exponent, and returns base^exponent
double my_pow(double base, double exponent) {
  // Calculate power
  errno = 0;
  double result = pow(base, exponent);
  if (errno == EDOM) {
    fprintf(stderr,
            "Domain error: pow(%.2f, %.2f) is undefined in the real number "
            "domain.\n",
            base, exponent);
    return -1;
  } else if (errno == ERANGE) {
    fprintf(stderr,
            "Range error: pow(%.2f, %.2f) caused overflow or underflow.\n",
            base, exponent);
    return -1;
  }

  return result;
}
