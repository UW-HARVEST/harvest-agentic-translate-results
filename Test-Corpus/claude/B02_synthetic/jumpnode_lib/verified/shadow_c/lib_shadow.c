/* Probe build of the ground-truth C library.
 *
 * This file does NOT copy or modify c_src/ in any way: it `#include`s the
 * untouched c_src/src/lib.c so that the `static` helpers and the `static`
 * node_storage / node_count objects land in THIS translation unit. That lets us
 * add external-linkage wrappers around them without editing the original
 * source, and guarantees the ground truth can never drift from c_src/.
 *
 * Used only by the differential test suite, paired with the Rust crate's
 * `shadow_probe` cargo feature.
 */

#include "../c_src/src/lib.c"

int probe_init(void) {
    initialize_test_data();
    return node_count;
}

void probe_reset(void) {
    node_count = 0;
    memset(node_storage, 0, sizeof(node_storage));
}

int probe_node_count(void) {
    return node_count;
}

int probe_add_node(int id, int parent_id, double value) {
    return add_node(id, parent_id, value);
}

int probe_find(int id) {
    Node *p = find_node_by_id(id);
    if (p == NULL) {
        return -1;
    }
    return (int)(p - node_storage);
}

int probe_process_backward(int *array, size_t size, int start_offset) {
    return process_backward(array, size, start_offset);
}

int probe_compute_size_metric(const char *s) {
    return compute_size_metric(s);
}

int probe_safe_double_to_int(double value) {
    return safe_double_to_int(value);
}

int probe_node_id(int idx) {
    return node_storage[idx].id;
}

int probe_node_parent_id(int idx) {
    return node_storage[idx].parent_id;
}

double probe_node_value(int idx) {
    return node_storage[idx].value;
}

int probe_node_data(int idx, int k) {
    return node_storage[idx].data[k];
}

size_t probe_sizeof_node(void) {
    return sizeof(Node);
}

int probe_status(int which) {
    switch (which) {
        case 0: return STATUS_OK;
        case 1: return STATUS_WARNING;
        case 2: return STATUS_ERROR;
        case 3: return STATUS_CRITICAL;
        case 4: return MAX_NODES;
        default: return -1;
    }
}
