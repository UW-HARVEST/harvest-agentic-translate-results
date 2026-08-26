/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 * 
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
#include <stddef.h>
#include <string.h>
#include <stdint.h>

/* Forward declarations */
static size_t compact_runs(uint8_t *buf, size_t len, uint8_t threshold);
static void rotate_buffer(uint8_t *buf, size_t len, int offset);
static size_t remove_duplicates(uint8_t *buf, size_t len, int preserve_order);
static void interleave_halves(uint8_t *buf, size_t len);
static void reverse_segments(uint8_t *buf, size_t len, size_t seg_size);

/**
 * Main entrance function - processes buffer based on operation flags
 * 
 * @param buffer Input/output buffer
 * @param length Buffer length
 * @param flags Bit flags: 
 *              bit 0: rotate buffer
 *              bit 1: compact runs
 *              bit 2: remove duplicates
 *              bit 3: interleave halves
 *              bit 4: reverse segments
 * @param param1 Operation-specific parameter (rotation offset, run threshold, segment size)
 * @param param2 Secondary parameter (preserve order flag, etc)
 * @return New buffer length after processing
 */
size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags, 
                     int param1, int param2) {
    size_t new_len = length;
    
    if (buffer == NULL || length == 0) {
        return 0;
    }
    
    /* Branch based on multiple flags - creates diverse control flow */
    if (flags & 0x01) {  /* Rotate */
        int offset = param1 % (int)length;
        if (offset != 0) {
            rotate_buffer(buffer, length, offset);
        }
    }
    
    if (flags & 0x02) {  /* Compact runs */
        uint8_t threshold = (param1 > 0 && param1 <= 255) ? (uint8_t)param1 : 3;
        new_len = compact_runs(buffer, new_len, threshold);
    }
    
    if (flags & 0x04) {  /* Remove duplicates */
        int preserve = (param2 != 0);
        new_len = remove_duplicates(buffer, new_len, preserve);
    }
    
    /* Conditional chaining based on length */
    if ((flags & 0x08) && new_len >= 2) {  /* Interleave */
        interleave_halves(buffer, new_len);
    }
    
    if ((flags & 0x10) && new_len >= 4) {  /* Reverse segments */
        size_t seg_size = (param1 > 0) ? (size_t)param1 : 4;
        if (seg_size <= new_len) {
            reverse_segments(buffer, new_len, seg_size);
        }
    }
    
    return new_len;
}

/**
 * Rotate buffer by offset positions (positive = right, negative = left)
 * Uses multiple memmove operations with different patterns
 */
static void rotate_buffer(uint8_t *buf, size_t len, int offset) {
    if (len <= 1) return;
    
    /* Normalize offset */
    offset = offset % (int)len;
    if (offset < 0) offset += len;
    if (offset == 0) return;
    
    /* Use reversal algorithm with memmove */
    uint8_t temp[256];
    size_t chunk = (offset < 256) ? offset : 256;
    
    if ((size_t)offset < len / 2) {
        /* Small offset: move prefix aside, shift main part, restore prefix */
        size_t i;
        for (i = 0; i < (size_t)offset; i += chunk) {
            size_t copy_len = ((size_t)offset - i < chunk) ? (size_t)offset - i : chunk;
            memmove(temp, buf + i, copy_len);
            memmove(buf + i, buf + offset, len - offset);
            memmove(buf + len - offset, temp, copy_len);
        }
    } else {
        /* Large offset: work from the right */
        size_t shift = len - offset;
        memmove(temp, buf, shift);
        memmove(buf, buf + shift, offset);
        memmove(buf + offset, temp, shift);
    }
}

/**
 * Compact consecutive runs of same value if run length >= threshold
 * Complex nested loops with multiple data paths
 */
static size_t compact_runs(uint8_t *buf, size_t len, uint8_t threshold) {
    size_t read = 0, write = 0;
    
    while (read < len) {
        uint8_t current = buf[read];
        size_t run_len = 1;
        
        /* Count run length */
        while (read + run_len < len && buf[read + run_len] == current) {
            run_len++;
        }
        
        if (run_len >= threshold) {
            /* Compact to 2 elements: value, count */
            if (run_len > 255) run_len = 255;  /* Cap at 255 */
            
            buf[write++] = current;
            buf[write++] = (uint8_t)run_len;
            
            /* Shift remaining data if needed */
            if (read + run_len < len) {
                size_t remaining = len - (read + run_len);
                memmove(buf + write, buf + read + run_len, remaining);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            /* Keep run as-is, but may need to move it */
            if (write != read) {
                memmove(buf + write, buf + read, run_len);
            }
            write += run_len;
            read += run_len;
        }
    }
    
    return len;
}

/**
 * Remove duplicate values - different paths for ordered/unordered
 */
static size_t remove_duplicates(uint8_t *buf, size_t len, int preserve_order) {
    if (len <= 1) return len;
    
    if (preserve_order) {
        /* Preserve order: O(n^2) but maintains sequence */
        size_t write = 1;
        for (size_t i = 1; i < len; i++) {
            size_t j;
            for (j = 0; j < write; j++) {
                if (buf[i] == buf[j]) break;
            }
            if (j == write) {
                if (write != i) {
                    buf[write] = buf[i];
                }
                write++;
            }
        }
        return write;
    } else {
        /* Don't preserve order: sort-like approach with memmove */
        uint8_t seen[256] = {0};
        size_t write = 0;
        
        for (size_t i = 0; i < len; i++) {
            if (!seen[buf[i]]) {
                seen[buf[i]] = 1;
                if (write != i) {
                    /* Swap to front */
                    uint8_t temp = buf[write];
                    buf[write] = buf[i];
                    buf[i] = temp;
                }
                write++;
            }
        }
        return write;
    }
}

/**
 * Interleave first and second halves of buffer
 * Complex memmove pattern with temporary storage
 */
static void interleave_halves(uint8_t *buf, size_t len) {
    if (len < 2) return;
    
    size_t half = len / 2;
    size_t odd = len % 2;
    uint8_t temp[512];
    
    if (half <= 256) {
        /* Use temp buffer for small sizes */
        memmove(temp, buf, half);
        
        for (size_t i = 0; i < half; i++) {
            memmove(buf + i * 2 + 1, buf + half + i, 1);
            buf[i * 2] = temp[i];
        }
        if (odd) {
            buf[len - 1] = buf[half];
        }
    } else {
        /* In-place for large buffers - more complex */
        for (size_t i = 0; i < half; i++) {
            size_t src = half + i;
            size_t dst = i * 2 + 1;
            if (dst < src) {
                uint8_t val = buf[src];
                memmove(buf + dst + 1, buf + dst, src - dst);
                buf[dst] = val;
            }
        }
    }
}

/**
 * Reverse buffer in fixed-size segments
 * Nested loops with conditional memmove operations
 */
static void reverse_segments(uint8_t *buf, size_t len, size_t seg_size) {
    if (seg_size <= 1 || len < seg_size) return;
    
    size_t num_segments = len / seg_size;
    size_t remainder = len % seg_size;
    
    /* Process complete segments */
    for (size_t seg = 0; seg < num_segments; seg++) {
        size_t base = seg * seg_size;
        
        /* Reverse within segment using memmove */
        for (size_t i = 0; i < seg_size / 2; i++) {
            uint8_t temp;
            size_t left = base + i;
            size_t right = base + seg_size - 1 - i;
            
            temp = buf[left];
            memmove(buf + left, buf + right, 1);
            memmove(buf + right, &temp, 1);
        }
    }
    
    /* Handle remainder if exists and is > 1 */
    if (remainder > 1) {
        size_t base = num_segments * seg_size;
        for (size_t i = 0; i < remainder / 2; i++) {
            uint8_t temp = buf[base + i];
            buf[base + i] = buf[base + remainder - 1 - i];
            buf[base + remainder - 1 - i] = temp;
        }
    }
}
