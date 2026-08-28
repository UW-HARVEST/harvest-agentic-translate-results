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
//! Port of `main.c`.

mod cio;
mod dag_lib;

use cio::{sscanf_int, Console, Input};
use dag_lib::*;

const MAX_INPUT: usize = 256;

fn print_menu(c: &mut Console) {
    c.out(b"\n=== DAG City Route Manager ===\n");
    c.out(b"1. Add city (node)\n");
    c.out(b"2. Add route (edge)\n");
    c.out(b"3. Show all cities\n");
    c.out(b"4. Show city details\n");
    c.out(b"5. Find shortest path\n");
    c.out(b"6. Make shallow copy of subsection\n");
    c.out(b"7. Delete node\n");
    c.out(b"8. Exit\n");
    c.out(b"Choice: ");
}

/// `input[strcspn(input, "\n")] = 0` applied to a C string: the bytes before the
/// first newline, or before the first NUL if there is no newline.
fn chomp(buf: &[u8]) -> &[u8] {
    let s = cstr(buf);
    match s.iter().position(|&b| b == b'\n') {
        Some(i) => &s[..i],
        None => s,
    }
}

fn main() {
    let mut c = Console::new();
    let mut input_stream = Input::new();
    let mut heap = Heap::new();

    // create_graph() only fails when malloc fails, which we do not model.
    let mut graph = create_graph();

    c.out(b"City Route Management System\n");
    c.out(b"Commands are read from stdin\n");

    loop {
        print_menu(&mut c);

        let line = match input_stream.fgets(MAX_INPUT) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(cstr(&line)) {
            Some(v) => v,
            None => {
                c.out(b"Invalid input\n");
                continue;
            }
        };

        // Note: in the C code a `break` inside a case leaves the switch, not the
        // while loop, so hitting EOF mid-command prints the menu once more
        // before the outer fgets ends the program. The labelled blocks below
        // reproduce that.
        match choice {
            1 => 'case1: {
                // Add city
                c.out(b"Enter city name: ");
                let raw = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case1,
                };

                // Remove newline
                let name = chomp(&raw).to_vec();

                let node = add_node(&mut heap, &mut graph, &name, &mut c);
                if node != NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"Added city: ");
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"\n");
                    c.out(&m);
                } else {
                    c.out(b"Failed to add city\n");
                }
            }

            2 => 'case2: {
                // Add route
                c.out(b"Enter from city: ");
                let raw_from = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case2,
                };
                let from_city = chomp(&raw_from).to_vec();

                c.out(b"Enter to city: ");
                let raw_to = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case2,
                };
                let to_city = chomp(&raw_to).to_vec();

                c.out(b"Enter distance: ");
                let raw_dist = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case2,
                };
                let distance = match sscanf_int(cstr(&raw_dist)) {
                    Some(v) => v,
                    None => {
                        c.out(b"Invalid distance\n");
                        break 'case2;
                    }
                };

                let from = get_node_by_name(&heap, &graph, &from_city);
                let to = get_node_by_name(&heap, &graph, &to_city);

                if from == NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&from_city);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                    break 'case2;
                }
                if to == NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&to_city);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                    break 'case2;
                }

                if add_edge(&mut heap, from, to, distance, &mut c) == 0 {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"Added route: ");
                    m.extend_from_slice(&from_city);
                    m.extend_from_slice(b" -> ");
                    m.extend_from_slice(&to_city);
                    m.extend_from_slice(format!(" (distance: {})\n", distance).as_bytes());
                    c.out(&m);
                } else {
                    c.out(b"Failed to add route\n");
                }
            }

            3 => {
                // Show all cities
                print_graph(&heap, &graph, &mut c);
            }

            4 => 'case4: {
                // Show city details
                c.out(b"Enter city name: ");
                let raw = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case4,
                };
                let name = chomp(&raw).to_vec();

                let node = get_node_by_name(&heap, &graph, &name);
                if node != NULL {
                    print_node(&heap, node, &mut c);
                } else {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                }
            }

            5 => 'case5: {
                // Find shortest path
                c.out(b"Enter start city: ");
                let raw_start = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case5,
                };
                let start_city = chomp(&raw_start).to_vec();

                c.out(b"Enter end city: ");
                let raw_end = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case5,
                };
                let end_city = chomp(&raw_end).to_vec();

                let start = get_node_by_name(&heap, &graph, &start_city);
                let end = get_node_by_name(&heap, &graph, &end_city);

                if start == NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&start_city);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                    break 'case5;
                }
                if end == NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&end_city);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                    break 'case5;
                }

                let mut path_length: i32 = 0;
                let path = find_shortest_path(&mut heap, start, end, &mut path_length, &mut c);

                match path {
                    Some(path) => {
                        let mut m = Vec::new();
                        m.extend_from_slice(b"Shortest path from ");
                        m.extend_from_slice(&start_city);
                        m.extend_from_slice(b" to ");
                        m.extend_from_slice(&end_city);
                        m.extend_from_slice(b":\n");
                        for i in 0..path_length as usize {
                            m.extend_from_slice(format!("  {}. ", i + 1).as_bytes());
                            m.extend_from_slice(heap.name(path[i]));
                            m.extend_from_slice(b"\n");
                        }
                        c.out(&m);
                    }
                    None => {
                        c.out(b"No path found\n");
                    }
                }
            }

            6 => 'case6: {
                // Make shallow copy
                c.out(b"Enter start city for shallow copy: ");
                let raw = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case6,
                };
                let name = chomp(&raw).to_vec();

                let node = get_node_by_name(&heap, &graph, &name);
                if node == NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                    break 'case6;
                }

                let copy = shallow_copy(&mut heap, node, &mut c);
                if copy != NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"Created shallow copy starting from ");
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"\n");
                    m.extend_from_slice(b"Reference counts incremented for all reachable nodes\n");
                    c.out(&m);
                    print_node(&heap, copy, &mut c);
                } else {
                    c.out(b"Failed to create shallow copy\n");
                }
            }

            7 => 'case7: {
                // Delete node
                c.out(b"Enter city name to delete: ");
                let raw = match input_stream.fgets(MAX_INPUT) {
                    Some(l) => l,
                    None => break 'case7,
                };
                let name = chomp(&raw).to_vec();

                let node = get_node_by_name(&heap, &graph, &name);
                if node == NULL {
                    let mut m = Vec::new();
                    m.extend_from_slice(b"City '");
                    m.extend_from_slice(&name);
                    m.extend_from_slice(b"' not found\n");
                    c.out(&m);
                    break 'case7;
                }

                c.out(format!("Current ref count: {}\n", heap.get(node).ref_count).as_bytes());
                delete_node(&mut heap, node, &mut c);
                let mut m = Vec::new();
                m.extend_from_slice(b"Decremented reference count for ");
                m.extend_from_slice(&name);
                m.extend_from_slice(b"\n");
                m.extend_from_slice(b"Note: Node will be freed when ref count reaches 0\n");
                c.out(&m);
            }

            8 => {
                // Exit
                c.out(b"Freeing graph and exiting...\n");
                free_graph(&mut heap, &graph, &mut c);
                c.flush();
                std::process::exit(0);
            }

            _ => {
                c.out(b"Invalid choice\n");
            }
        }
    }

    free_graph(&mut heap, &graph, &mut c);
    c.flush();
}
