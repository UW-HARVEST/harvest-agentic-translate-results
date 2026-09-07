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
	int values[4];
	int count;
	char *label;
} DataBlock;

void shift_array(int *arr, int size, int positions) {
	if (positions > 0 && positions < size) {
		memmove(arr + positions, arr, (size - positions) * sizeof(int));
		for (int i = 0; i < positions; i++) {
			arr[i] = 0;
		}
	}
}

int process_string(const char *str) {
	if (*str) {
		return (int)strlen(str);
	}
	return 0;
}

int apply_bitmask(int value, int operation) {
	int mask1 = 0b11110000;
	int mask2 = 0b00001111;
	int mask3 = 0b10101010;
	int mask4 = 0b01010101;

	switch (operation) {
		case 0:
			return value & mask1;
		case 1:
			return value & mask2;
		case 2:
			return value | mask3;
		case 3:
			return value ^ mask4;
		default:
			return value;
	}
}

void init_matrix(int matrix[3][4]) {
	int temp[3][4] = {
		{1, 2, 3, 4},
		{5, 6, 7, 8},
		{9, 10, 11, 12}
	};

	for (int i = 0; i < 3; i++) {
		for (int j = 0; j < 4; j++) {
			matrix[i][j] = temp[i][j];
		}
	}
}

int compare_allocations(int val1, int val2) {
	int *ptr1 = (int *)malloc(sizeof(int));
	int *ptr2 = (int *)malloc(sizeof(int));

	int *uninit_ptr;

	if (ptr1 == NULL || ptr2 == NULL) {
		free(ptr1);
		free(ptr2);
		return -1;
	}

	*ptr1 = val1;
	*ptr2 = val2;

	int result = 0;

	if (ptr1 < ptr2) {
		result = 1;
	} else if (ptr1 > ptr2) {
		result = 2;
	} else {
		result = 3;
	}

	uninit_ptr = ptr1;
	result += (*uninit_ptr > 0) ? 10 : 0;

	free(ptr1);
	free(ptr2);

	return result;
}

int arity4(int param1, int param2, int param3, int param4) {
	int result = 0;

	DataBlock block = {
		.values = {param1, param2, param3, param4},
		.count = 4,
		.label = NULL
	};

	char test_str[] = "Hello";
	char empty_str[] = "";

	int len1 = process_string(test_str);
	int len2 = process_string(empty_str);

	result += len1 + len2;

	shift_array(block.values, 4, 1);

	for (int i = 0; i < block.count; i++) {
		result += block.values[i];
	}

	result = apply_bitmask(result, param1 % 4);

	int matrix[3][4];
	init_matrix(matrix);

	result += matrix[0][0] + matrix[2][3];

	int alloc_result = compare_allocations(param1, param2);
	result += alloc_result;

	if (param3 != 0) {
		result = (result * param3) / 100;
	}

	if (param4 != 0) {
		result += param4;
	}

	return result;
}

int arity2(int p1, int p2) {
	return arity4(p1, p2, 0, 0);
}

int arity3(int p1, int p2, int p3) {
	return arity4(p1, p2, p3, 0);
}

int arity(unsigned char len, int *params) {
	if (len < 2) {
		return -1;
	} else if (len == 2) {
		return arity2(params[0], params[1]);
	} else if (len == 3) {
		return arity3(params[0], params[1], params[2]);
	} else {
		return arity4(params[0], params[1], params[2], params[3]);
	}
}
