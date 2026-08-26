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
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <limits.h>

#define MAX_NODES 100
#define MAX_NAME_LEN 50

typedef struct {
    int id;
    int parent_id;
    char name[MAX_NAME_LEN];
    double value;
    int active;
} Node;

static Node node_storage[MAX_NODES];
static int node_count = 0;

int add_node(int id, int parent_id, const char *name, double value) {
    if (node_count >= MAX_NODES) {
        return -1;
    }

    Node new_node = {
        .id = id,
        .parent_id = parent_id,
        .value = value,
        .active = 1
    };

    strncpy(new_node.name, name, MAX_NAME_LEN - 1);
    new_node.name[MAX_NAME_LEN - 1] = '\0';

    node_storage[node_count++] = new_node;
    return node_count - 1;
}

Node* find_node_by_id(int id) {
    for (int i = 0; i < node_count; i++) {
        if (node_storage[i].id == id && node_storage[i].active) {
            return &node_storage[i];
        }
    }
    return NULL;
}

int get_children_count(int parent_id) {
    int count = 0;
    for (int i = 0; i < node_count; i++) {
        if (node_storage[i].parent_id == parent_id && node_storage[i].active) {
            count++;
        }
    }
    return count;
}

double calculate_subtree_sum(int node_id) {
    Node *node = find_node_by_id(node_id);
    if (node == NULL) {
        return 0.0;
    }

    double sum = node->value;

    for (int i = 0; i < node_count; i++) {
        if (node_storage[i].parent_id == node_id && node_storage[i].active) {
            sum += calculate_subtree_sum(node_storage[i].id);
        }
    }

    return sum;
}

int process_string(char *str) {
    int result = 0;

    if (*str) {
        while (*str) {
            result += (int)(*str);
            str++;
        }
    }

    return result;
}

int safe_double_to_int(double d) {
    if (d > (double)INT_MAX) {
        return INT_MAX;
    }
    if (d < (double)INT_MIN) {
        return INT_MIN;
    }

    if (d != d) {
        return 0;
    }

    return (int)d;
}

int maxnmin(int param1, int param2, int param3, int param4) {
    int result = 0;

    node_count = 0;

    add_node(1, -1, "root", 10.5);
    add_node(2, 1, "child1", 20.7);
    add_node(3, 1, "child2", 15.3);
    add_node(4, 2, "grandchild1", 5.9);
    add_node(5, 2, "grandchild2", 8.2);
    add_node(6, 3, "grandchild3", 12.4);

    int node_id = (param1 % 6) + 1;
    Node *selected_node = find_node_by_id(node_id);

    if (selected_node != NULL) {
        char *name_ptr = selected_node->name;

        if (*name_ptr) {
            result += process_string(name_ptr);
        }

        double subtree_sum = calculate_subtree_sum(node_id);

        int sum_as_int = safe_double_to_int(subtree_sum);
        result += sum_as_int;
    }

    int second_node_id = (param2 % 6) + 1;
    Node *second_node = find_node_by_id(second_node_id);

    if (second_node != NULL) {
        double value_multiplied = second_node->value * param3;

        int converted_value = safe_double_to_int(value_multiplied);
        result += converted_value;
    }

    int parent_id = (param4 % 3) + 1;
    int children = get_children_count(parent_id);
    result += children * 10;

    double calculation = (double)(param1 + param2) / (double)(param3 + 1);
    calculation *= param4;

    int final_calc = safe_double_to_int(calculation);
    result += final_calc;

    return result;
}
