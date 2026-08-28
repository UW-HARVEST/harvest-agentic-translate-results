/*
 * Test-only harness. Does NOT modify c_src/: it #includes the pristine
 * translation unit so that the `static` (file-local) functions and the
 * file-scope mutable state become reachable, and re-exports them under
 * `h_*` names for differential testing.
 *
 * Pointers are never returned across the FFI boundary (the two libraries
 * have different load addresses); results are normalised to indices.
 */

#include <stddef.h>

#include "lib.c"

void h_reset(void) {
    node_count = 0;
}

int h_node_count(void) {
    return node_count;
}

void h_set_node_count(int n) {
    node_count = n;
}

/* Returns the index of the matching node, or -1 for NULL. */
int h_find_node_by_id(int id) {
    Node *p = find_node_by_id(id);
    if (p == NULL) {
        return -1;
    }
    return (int)(p - node_storage);
}

int h_add_node(int id, int parent_id, double value) {
    return add_node(id, parent_id, value);
}

int h_process_backward(int *array, size_t size, int start_offset) {
    return process_backward(array, size, start_offset);
}

int h_compute_size_metric(const char *str) {
    return compute_size_metric(str);
}

int h_safe_double_to_int(double value) {
    return safe_double_to_int(value);
}

void h_initialize_test_data(void) {
    initialize_test_data();
}

/* Reads one slot of node_storage. Returns 0 on success, -1 if out of range. */
int h_get_node(int index, int *id, int *parent_id, double *value, int *data_out4) {
    int i;
    if (index < 0 || index >= 100) {
        return -1;
    }
    *id = node_storage[index].id;
    *parent_id = node_storage[index].parent_id;
    *value = node_storage[index].value;
    for (i = 0; i < 4; i++) {
        data_out4[i] = node_storage[index].data[i];
    }
    return 0;
}

/* Raw byte view of one node, to compare struct layout/padding exactly. */
int h_node_bytes(int index, unsigned char *out, size_t out_len) {
    size_t n = sizeof(Node);
    const unsigned char *src;
    size_t i;
    if (index < 0 || index >= 100 || out_len < n) {
        return -1;
    }
    src = (const unsigned char *)&node_storage[index];
    for (i = 0; i < n; i++) {
        out[i] = src[i];
    }
    return (int)n;
}

size_t h_sizeof_node(void) {
    return sizeof(Node);
}

/* Status constants, so the octal literals are compared rather than assumed. */
int h_status_ok(void) { return STATUS_OK; }
int h_status_warning(void) { return STATUS_WARNING; }
int h_status_error(void) { return STATUS_ERROR; }
int h_status_critical(void) { return STATUS_CRITICAL; }
int h_max_nodes(void) { return MAX_NODES; }
