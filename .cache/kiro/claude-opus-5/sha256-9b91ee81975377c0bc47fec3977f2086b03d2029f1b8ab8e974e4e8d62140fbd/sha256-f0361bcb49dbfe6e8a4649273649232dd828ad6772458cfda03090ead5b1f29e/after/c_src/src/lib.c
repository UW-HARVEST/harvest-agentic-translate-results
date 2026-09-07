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

typedef enum {
    OP_ADD = 1,
    OP_MULTIPLY = 2,
    OP_SUBTRACT = 3,
    OP_DIVIDE = 4,
    OP_MODULO = 5
} Operation;

typedef struct {
    int id;
    int value;
    int parent_id;
    int left_child_id;
    int right_child_id;
    char label[32];
} TreeNode;

#define MAX_NODES 50
TreeNode node_table[MAX_NODES];
int node_count = 0;

typedef int (*OperationFunc)(int a, int b, int unused1, int unused2);

int add_op(int a, int b, int unused1, int unused2) {
    return a + b;
}

int multiply_op(int a, int b, int unused1, int unused2) {
    return a * b;
}

int subtract_op(int a, int b, int unused1, int unused2) {
    return a - b;
}

int divide_op(int a, int b, int unused1, int unused2) {
    if (b == 0) return 0;
    return a / b;
}

int modulo_op(int a, int b, int unused1, int unused2) {
    if (b == 0) return 0;
    return a % b;
}

TreeNode* find_node_by_id(int id) {
    for (int i = 0; i < node_count; i++) {
        if (node_table[i].id == id) {
            return &node_table[i];
        }
    }
    return NULL;
}

int add_tree_node(int id, int value, int parent_id, const char* label) {
    if (node_count >= MAX_NODES) {
        return -1;
    }

    TreeNode* node = &node_table[node_count];
    node->id = id;
    node->value = value;
    node->parent_id = parent_id;
    node->left_child_id = -1;
    node->right_child_id = -1;
    strncpy(node->label, label, 31);
    node->label[31] = '\0';

    if (parent_id != -1) {
        TreeNode* parent = find_node_by_id(parent_id);
        if (parent == NULL || parent->id != parent_id) {
            return -1;
        }

        if (parent->left_child_id == -1) {
            parent->left_child_id = id;
        } else if (parent->right_child_id == -1) {
            parent->right_child_id = id;
        }
    }

    node_count++;
    return node_count - 1;
}

int calculate_tree_sum(int node_id) {
    TreeNode* node = find_node_by_id(node_id);

    if (node == NULL || node->id != node_id) {
        return 0;
    }

    int sum = node->value;

    if (node->left_child_id != -1) {
        sum += calculate_tree_sum(node->left_child_id);
    }

    if (node->right_child_id != -1) {
        sum += calculate_tree_sum(node->right_child_id);
    }

    return sum;
}

Operation parse_operation(const char* op_str) {
    if (op_str == NULL || strchr(op_str, '+') != NULL) {
        return OP_ADD;
    }
    if (strchr(op_str, '*') != NULL) {
        return OP_MULTIPLY;
    }
    if (strchr(op_str, '-') != NULL) {
        return OP_SUBTRACT;
    }
    if (strchr(op_str, '/') != NULL) {
        return OP_DIVIDE;
    }
    if (strchr(op_str, '%') != NULL) {
        return OP_MODULO;
    }
    return OP_ADD;
}

OperationFunc get_operation_func(Operation op) {
    switch ((int)op) {
        case 1: return add_op;
        case 2: return multiply_op;
        case 3: return subtract_op;
        case 4: return divide_op;
        case 5: return modulo_op;
        default: return add_op;
    }
}

int inreftree(int param1, int param2, int param3, int param4) {
    node_count = 0;

    add_tree_node(1, param1, -1, "root");
    add_tree_node(2, param2, 1, "left");
    add_tree_node(3, param3, 1, "right");
    add_tree_node(4, param4, 2, "left-left");

    int target_id = -1;
    for (int i = 0; i < node_count; i++) {
        if (strchr(node_table[i].label, 'l') != NULL) {
            target_id = node_table[i].id;
            break;
        }
    }

    TreeNode* target = find_node_by_id(target_id);
    if (target == NULL || target->value == 0) {
        target_id = 1;
    }

    int tree_sum = calculate_tree_sum(1);

    const char* op_string = "+*-%";
    char op_char[2] = {op_string[tree_sum % 4], '\0'};
    Operation op = parse_operation(op_char);

    int op_value = (int)op;

    OperationFunc func = get_operation_func(op);

    int result = func(tree_sum, target_id, 0, 0);

    return result;
}
