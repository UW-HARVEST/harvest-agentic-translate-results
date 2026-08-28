/*
 * Test-only C shim.
 *
 * c_src/ is NEVER modified. This file lives outside c_src/ and simply #includes
 * the pristine C translation unit, then adds one exported wrapper around the
 * file-local `initialize_test_data()` so that the differential tests can reach
 * the code paths in `jumpnode` that are only live once `node_storage` is
 * populated (cases 0001 / 0002 / 0004 bodies, add_node, process_backward,
 * safe_double_to_int).
 *
 * The resulting .so exports exactly the same two symbols as the Rust cdylib
 * built with --features expose_init_test_data:
 *      jumpnode
 *      jumpnode_initialize_test_data
 */

#include "../../../c_src/src/lib.c"

void jumpnode_initialize_test_data(void) {
    initialize_test_data();
}
