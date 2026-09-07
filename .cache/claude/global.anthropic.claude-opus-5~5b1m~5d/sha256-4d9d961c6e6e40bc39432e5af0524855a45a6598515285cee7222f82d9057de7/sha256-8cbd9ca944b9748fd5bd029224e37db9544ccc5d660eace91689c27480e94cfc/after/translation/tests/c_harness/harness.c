/* Differential-test harness for the ORIGINAL C library.
 *
 * c_src/ is read-only, so instead of editing it we #include the translation
 * unit verbatim. That gives us access to the file-static helpers
 * (initialize_test_data, add_node, node_count) which are dead code in the
 * shipped .so but which the Rust translation must still reproduce exactly.
 *
 * The exported hook names match the ones the Rust crate exposes behind the
 * `expose_init_test_data` cargo feature, so both sides are driven identically.
 */

#include "../../../c_src/src/lib.c"

void jumpnode_initialize_test_data(void) {
    initialize_test_data();
}

int jumpnode_test_add_node(int id, int parent_id, double value) {
    return add_node(id, parent_id, value);
}

void jumpnode_test_set_node_count(int n) {
    node_count = n;
}

int jumpnode_test_get_node_count(void) {
    return node_count;
}

/* Direct probes of the remaining file-static helpers, so every C function is
 * covered differentially and not just the ones jumpnode happens to reach. */
int jumpnode_test_find_node_index(int id) {
    Node *p = find_node_by_id(id);
    if (p == NULL) {
        return -1;
    }
    return (int)(p - node_storage);
}

int jumpnode_test_compute_size_metric(const char *s) {
    return compute_size_metric(s);
}

int jumpnode_test_safe_double_to_int(double v) {
    return safe_double_to_int(v);
}

int jumpnode_test_process_backward(int *array, size_t size, int start_offset) {
    return process_backward(array, size, start_offset);
}

/* Reads node_storage[i] into caller-provided out params. */
int jumpnode_test_get_node(int index, int *id, int *parent_id, double *value,
                          int *data_out /* 4 ints */) {
    if (index < 0 || index >= MAX_NODES) {
        return -1;
    }
    *id = node_storage[index].id;
    *parent_id = node_storage[index].parent_id;
    *value = node_storage[index].value;
    data_out[0] = node_storage[index].data[0];
    data_out[1] = node_storage[index].data[1];
    data_out[2] = node_storage[index].data[2];
    data_out[3] = node_storage[index].data[3];
    return 0;
}

int jumpnode_test_sizeof_node(void) { return (int)sizeof(Node); }
int jumpnode_test_max_nodes(void) { return MAX_NODES; }
