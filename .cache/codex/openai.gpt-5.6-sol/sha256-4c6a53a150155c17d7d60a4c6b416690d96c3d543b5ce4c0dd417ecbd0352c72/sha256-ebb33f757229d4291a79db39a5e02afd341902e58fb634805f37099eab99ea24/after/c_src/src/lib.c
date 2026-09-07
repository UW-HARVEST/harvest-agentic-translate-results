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

#include <stdio.h>
#include <string.h>
#include <time.h>
#include <math.h>

int classify_mode(const char *mode) {
    if (strcmp(mode, "standard") == 0) {
        return 0x10;
    } else if (strcmp(mode, "enhanced") == 0) {
        return 0x20;
    } else if (strcmp(mode, "turbo") == 0) {
        return 0x30;
    } else if (strcmp(mode, "extreme") == 0) {
        return 0x40;
    }
    return 0x00;
}

int apply_multiplier(int base, int level) {
    int result = base;

    switch (level) {
        case 4:
            result += 0xFF;
        case 3:
            result += 0xAB;
        case 2:
            result += 0x7E;
        case 1:
            result += 0x1C;
        case 0:
            result += 0x05;
            break;
        default:
            result = 0xDEAD;
    }

    return result;
}

int convert_time_factor(double factor) {
    double scaled = factor * 1e12;
    int result = (int)scaled;

    return result;
}

int convert_negative_overflow(double value) {
    double extreme = value * -1e15;
    int result = (int)extreme;

    return result;
}

time_t get_modified_time(int offset_days, int offset_hours) {
    time_t current = time(NULL);
    current = current >> 29;
    time_t offset = (offset_days * 86400) + (offset_hours * 3600);
    return current + offset;
}

int hash_time_value(time_t t) {
    int hash = 0x5A5A5A5A;
    unsigned char *bytes = (unsigned char *)&t;

    for (size_t i = 0; i < sizeof(time_t); i++) {
        hash ^= bytes[i] << ((i % 4) * 8);
        hash *= 0x1F;
    }

    return hash & 0x7FFFFFFF;
}

int modeselect(int mode_selector, int time_offset, int complexity, int seed) {
    int result = 0;
    const char *modes[] = {"standard", "enhanced", "turbo", "extreme"};

    int mode_index = mode_selector % 4;
    const char *selected_mode = modes[mode_index];
    int mode_value = classify_mode(selected_mode);

    printf("Selected mode: %s (0x%X)\n", selected_mode, mode_value);
    result += mode_value;

    int complexity_level = complexity % 5;
    int multiplier = apply_multiplier(0xA0, complexity_level);

    printf("Complexity level: %d, Multiplier: 0x%X\n", complexity_level, multiplier);
    result += multiplier;

    time_t modified_time = get_modified_time(time_offset, seed % 24);
    int time_hash = hash_time_value(modified_time);

    printf("Modified time: %ld, Hash: 0x%X\n", (long)modified_time, time_hash);
    result += (time_hash % 0x1000);

    double factor1 = (double)seed * 1e8;
    double factor2 = (double)time_offset * -1e7;

    printf("Converting double %.2e to int (may overflow)...\n", factor1);

    int result1 = convert_time_factor(factor1);
    printf("Result 1: %d (0x%X)\n", result1, result1);

    printf("Converting double %.2e to int (may underflow)...\n", factor2);
    int result2 = convert_negative_overflow(factor2);
    printf("Result 2: %d (0x%X)\n", result2, result2);

    result ^= (result1 & 0xFF);
    result ^= (result2 & 0xFF00);

    result = (result * 0x10) + 0xBEEF;

    printf("\nFinal result: %d (0x%X)\n", result, result);

    return result;
}
