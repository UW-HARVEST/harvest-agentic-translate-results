// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef struct {
    int id;
    char name[32];
    uint8_t flags;
} DataBlock;

typedef struct {
    int *data;
    size_t size;
} MemoryBlock;

DataBlock create_block(int id, const char *name, uint8_t flags) {
    DataBlock block;
    block.id = id;
    strcpy(block.name, name);
    block.flags = flags;
    return block;
}

MemoryBlock* allocate_block(size_t count, int init_value) {
    MemoryBlock *mb = (MemoryBlock*)malloc(sizeof(MemoryBlock));
    if (!mb) return NULL;

    mb->data = (int*)calloc(count, sizeof(int));
    if (!mb->data) {
        free(mb);
        return NULL;
    }

    mb->size = count;

    for (size_t i = 0; i < count; i++) {
        mb->data[i] = init_value + i;
    }

    return mb;
}

void free_block(MemoryBlock *mb) {
    if (mb) {
        if (mb->data) free(mb->data);
        free(mb);
    }
}

int compute_hash(MemoryBlock *mb1, MemoryBlock *mb2) {
    int hash = 0;

    if (mb1->data < mb2->data) {
        hash += 100;
    } else if (mb1->data > mb2->data) {
        hash += 200;
    }

    if (mb1 < mb2) {
        hash += 10;
    } else if (mb1 > mb2) {
        hash += 20;
    }

    return hash;
}

int betagamma(int param1, int param2, int param3, int param4) {
    int result = 0;

    DataBlock blocks[] = {
        {1, "Block_Alpha", 0b10101010},
        {2, "Block_Beta", 0b11001100},
        {3, "Block_Gamma", 0b11110000}
    };

    int num_blocks = sizeof(blocks) / sizeof(blocks[0]);

    for (int i = 0; i < num_blocks; i++) {
        DataBlock *current = &blocks[i];

        char temp_name[32];
        strcpy(temp_name, current->name);

        int flag_contribution = 0;
        if (current->flags & 0b00001111) {
            flag_contribution += param1;
        }
        if (current->flags & 0b11110000) {
            flag_contribution += param2;
        }
        if (current->flags & 0b10101010) {
            flag_contribution += param3;
        }
        if (current->flags & 0b01010101) {
            flag_contribution += param4;
        }

        result += flag_contribution * current->id;
    }

    size_t block_size = (param1 % 10) + 5;
    MemoryBlock *mem1 = allocate_block(block_size, param1);
    MemoryBlock *mem2 = allocate_block(block_size, param2);

    if (!mem1 || !mem2) {
        free_block(mem1);
        free_block(mem2);
        return -1;
    }

    int hash = compute_hash(mem1, mem2);
    result += hash;

    int sum1 = 0, sum2 = 0;
    for (size_t i = 0; i < mem1->size; i++) {
        sum1 += mem1->data[i];
    }
    for (size_t i = 0; i < mem2->size; i++) {
        sum2 += mem2->data[i];
    }

    result += (sum1 - sum2) / 10;

    DataBlock special = {99, "Special", 0b11111111};
    strcpy(special.name, "Modified");

    if (mem1->data != mem2->data) {
        result += special.id;
    }

    if (mem1->data > NULL && mem2->data > NULL) {
        result += special.flags;
    }

    free_block(mem1);
    free_block(mem2);

    return result;
}
