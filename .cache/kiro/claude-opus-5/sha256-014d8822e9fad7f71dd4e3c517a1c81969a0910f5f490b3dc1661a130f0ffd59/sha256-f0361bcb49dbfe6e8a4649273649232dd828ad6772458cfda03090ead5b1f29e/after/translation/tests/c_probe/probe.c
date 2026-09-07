/* Test-only probe shim.
 *
 * c_src/ is READ-ONLY and is not modified: this file #includes the original
 * translation unit verbatim (the path is supplied by -DLIB_C_PATH=...) so the
 * `static` functions become visible here, then re-exports them under `probe_`
 * names. Compiled into its own separate shared object used ONLY by
 * tests/phase_b_internals.rs; the shipped C .so built by CMakeLists.txt is
 * untouched and still exports only `jumpnode`.
 */

#include LIB_C_PATH

int probe_node_count(void) { return node_count; }
void probe_set_node_count(int n) { node_count = n; }
void probe_reset(void) { node_count = 0; }

int probe_add_node(int id, int parent_id, double value) {
    return add_node(id, parent_id, value);
}

void probe_initialize_test_data(void) { initialize_test_data(); }

/* -1 stands in for the NULL sentinel; otherwise the index into node_storage. */
int probe_find_node_index(int id) {
    Node *p = find_node_by_id(id);
    if (p == NULL) {
        return -1;
    }
    return (int)(p - node_storage);
}

int probe_compute_size_metric(const char *s) { return compute_size_metric(s); }

int probe_safe_double_to_int(double v) { return safe_double_to_int(v); }

int probe_process_backward(int *array, unsigned long size, int start_offset) {
    return process_backward(array, (size_t)size, start_offset);
}

/* Writes into a caller-provided 50-byte buffer, matching jumpnode's `buffer`. */
int probe_sprintf_node_depth(char *buffer, int node_id, int depth) {
    return sprintf(buffer, "Node_%d_Depth_%d", node_id, depth);
}

/* Lets a test read back a node's fields without knowing the struct layout. */
double probe_node_value(int index) { return node_storage[index].value; }
int probe_node_id(int index) { return node_storage[index].id; }
int probe_node_parent_id(int index) { return node_storage[index].parent_id; }
int probe_node_data(int index, int k) { return node_storage[index].data[k]; }
