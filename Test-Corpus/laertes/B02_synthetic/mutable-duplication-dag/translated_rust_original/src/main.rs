#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    static mut stdin: *mut _IO_FILE;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn sscanf(
        __s: *const libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn free(__ptr: *mut libc::c_void);
    fn strcspn(
        __s: *const libc::c_char,
        __reject: *const libc::c_char,
    ) -> libc::c_ulong;
    fn create_graph() -> *mut graph_t;
    fn add_node(graph: *mut graph_t, city_name: *const libc::c_char) -> *mut node_t;
    fn add_edge(
        from: *mut node_t,
        to: *mut node_t,
        distance: libc::c_int,
    ) -> libc::c_int;
    fn delete_node(node: *mut node_t);
    fn shallow_copy(start: *mut node_t) -> *mut node_t;
    fn find_shortest_path(
        start: *mut node_t,
        end: *mut node_t,
        path_length: *mut libc::c_int,
    ) -> *mut *mut node_t;
    fn free_graph(graph: *mut graph_t);
    fn get_node_by_name(graph: *mut graph_t, city_name: *const libc::c_char) -> *mut node_t;
    fn print_node(node: *mut node_t);
    fn print_graph(graph: *mut graph_t);
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct node_t {
    pub city_name: [libc::c_char; 64],
    pub ref_count: libc::c_int,
    pub edges: [edge_t; 10],
    pub edge_count: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct edge_t {
    pub destination: *mut node_t,
    pub distance: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct graph_t {
    pub nodes: [*mut node_t; 100],
    pub node_count: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const MAX_INPUT: libc::c_int = 256 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn print_menu() {
    printf(b"\n=== DAG City Route Manager ===\n\0" as *const u8 as *const libc::c_char);
    printf(b"1. Add city (node)\n\0" as *const u8 as *const libc::c_char);
    printf(b"2. Add route (edge)\n\0" as *const u8 as *const libc::c_char);
    printf(b"3. Show all cities\n\0" as *const u8 as *const libc::c_char);
    printf(b"4. Show city details\n\0" as *const u8 as *const libc::c_char);
    printf(b"5. Find shortest path\n\0" as *const u8 as *const libc::c_char);
    printf(b"6. Make shallow copy of subsection\n\0" as *const u8 as *const libc::c_char);
    printf(b"7. Delete node\n\0" as *const u8 as *const libc::c_char);
    printf(b"8. Exit\n\0" as *const u8 as *const libc::c_char);
    printf(b"Choice: \0" as *const u8 as *const libc::c_char);
}
unsafe fn main_0() -> libc::c_int {
    let mut graph: *mut graph_t = create_graph();
    if graph.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Failed to create graph\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    let mut input: [libc::c_char; 256] = [0; 256];
    let mut choice: libc::c_int = 0;
    printf(b"City Route Management System\n\0" as *const u8 as *const libc::c_char);
    printf(b"Commands are read from stdin\n\0" as *const u8 as *const libc::c_char);
    loop {
        print_menu();
        if fgets(
            &raw mut input as *mut libc::c_char,
            MAX_INPUT,
            stdin as *mut FILE,
        )
        .is_null()
        {
            break;
        }
        if sscanf(
            &raw mut input as *mut libc::c_char,
            b"%d\0" as *const u8 as *const libc::c_char,
            &raw mut choice,
        ) != 1 as libc::c_int
        {
            printf(b"Invalid input\n\0" as *const u8 as *const libc::c_char);
        } else {
            match choice {
                1 => {
                    printf(b"Enter city name: \0" as *const u8 as *const libc::c_char);
                    if !fgets(
                        &raw mut input as *mut libc::c_char,
                        MAX_INPUT,
                        stdin as *mut FILE,
                    )
                    .is_null()
                    {
                        input[strcspn(
                            &raw mut input as *mut libc::c_char,
                            b"\n\0" as *const u8 as *const libc::c_char,
                        ) as usize] = 0 as libc::c_char;
                        let mut node: *mut node_t =
                            add_node(graph, &raw mut input as *mut libc::c_char);
                        if !node.is_null() {
                            printf(
                                b"Added city: %s\n\0" as *const u8 as *const libc::c_char,
                                &raw mut input as *mut libc::c_char,
                            );
                        } else {
                            printf(
                                b"Failed to add city\n\0" as *const u8
                                    as *const libc::c_char,
                            );
                        }
                    }
                }
                2 => {
                    let mut from_city: [libc::c_char; 256] = [0; 256];
                    let mut to_city: [libc::c_char; 256] = [0; 256];
                    let mut distance: libc::c_int = 0;
                    printf(b"Enter from city: \0" as *const u8 as *const libc::c_char);
                    if !fgets(
                        &raw mut from_city as *mut libc::c_char,
                        MAX_INPUT,
                        stdin as *mut FILE,
                    )
                    .is_null()
                    {
                        from_city[strcspn(
                            &raw mut from_city as *mut libc::c_char,
                            b"\n\0" as *const u8 as *const libc::c_char,
                        ) as usize] = 0 as libc::c_char;
                        printf(b"Enter to city: \0" as *const u8 as *const libc::c_char);
                        if !fgets(
                            &raw mut to_city as *mut libc::c_char,
                            MAX_INPUT,
                            stdin as *mut FILE,
                        )
                        .is_null()
                        {
                            to_city[strcspn(
                                &raw mut to_city as *mut libc::c_char,
                                b"\n\0" as *const u8 as *const libc::c_char,
                            ) as usize] = 0 as libc::c_char;
                            printf(
                                b"Enter distance: \0" as *const u8 as *const libc::c_char,
                            );
                            if !fgets(
                                &raw mut input as *mut libc::c_char,
                                MAX_INPUT,
                                stdin as *mut FILE,
                            )
                            .is_null()
                            {
                                if sscanf(
                                    &raw mut input as *mut libc::c_char,
                                    b"%d\0" as *const u8 as *const libc::c_char,
                                    &raw mut distance,
                                ) != 1 as libc::c_int
                                {
                                    printf(
                                        b"Invalid distance\n\0" as *const u8
                                            as *const libc::c_char,
                                    );
                                } else {
                                    let mut from: *mut node_t = get_node_by_name(
                                        graph,
                                        &raw mut from_city as *mut libc::c_char,
                                    );
                                    let mut to: *mut node_t = get_node_by_name(
                                        graph,
                                        &raw mut to_city as *mut libc::c_char,
                                    );
                                    if from.is_null() {
                                        printf(
                                            b"City '%s' not found\n\0" as *const u8
                                                as *const libc::c_char,
                                            &raw mut from_city as *mut libc::c_char,
                                        );
                                    } else if to.is_null() {
                                        printf(
                                            b"City '%s' not found\n\0" as *const u8
                                                as *const libc::c_char,
                                            &raw mut to_city as *mut libc::c_char,
                                        );
                                    } else if add_edge(from, to, distance)
                                        == 0 as libc::c_int
                                    {
                                        printf(
                                            b"Added route: %s -> %s (distance: %d)\n\0" as *const u8
                                                as *const libc::c_char,
                                            &raw mut from_city as *mut libc::c_char,
                                            &raw mut to_city as *mut libc::c_char,
                                            distance,
                                        );
                                    } else {
                                        printf(
                                            b"Failed to add route\n\0" as *const u8
                                                as *const libc::c_char,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                3 => {
                    print_graph(graph);
                }
                4 => {
                    printf(b"Enter city name: \0" as *const u8 as *const libc::c_char);
                    if !fgets(
                        &raw mut input as *mut libc::c_char,
                        MAX_INPUT,
                        stdin as *mut FILE,
                    )
                    .is_null()
                    {
                        input[strcspn(
                            &raw mut input as *mut libc::c_char,
                            b"\n\0" as *const u8 as *const libc::c_char,
                        ) as usize] = 0 as libc::c_char;
                        let mut node_0: *mut node_t =
                            get_node_by_name(graph, &raw mut input as *mut libc::c_char);
                        if !node_0.is_null() {
                            print_node(node_0);
                        } else {
                            printf(
                                b"City '%s' not found\n\0" as *const u8
                                    as *const libc::c_char,
                                &raw mut input as *mut libc::c_char,
                            );
                        }
                    }
                }
                5 => {
                    let mut start_city: [libc::c_char; 256] = [0; 256];
                    let mut end_city: [libc::c_char; 256] = [0; 256];
                    printf(b"Enter start city: \0" as *const u8 as *const libc::c_char);
                    if !fgets(
                        &raw mut start_city as *mut libc::c_char,
                        MAX_INPUT,
                        stdin as *mut FILE,
                    )
                    .is_null()
                    {
                        start_city[strcspn(
                            &raw mut start_city as *mut libc::c_char,
                            b"\n\0" as *const u8 as *const libc::c_char,
                        ) as usize] = 0 as libc::c_char;
                        printf(b"Enter end city: \0" as *const u8 as *const libc::c_char);
                        if !fgets(
                            &raw mut end_city as *mut libc::c_char,
                            MAX_INPUT,
                            stdin as *mut FILE,
                        )
                        .is_null()
                        {
                            end_city[strcspn(
                                &raw mut end_city as *mut libc::c_char,
                                b"\n\0" as *const u8 as *const libc::c_char,
                            ) as usize] = 0 as libc::c_char;
                            let mut start: *mut node_t = get_node_by_name(
                                graph,
                                &raw mut start_city as *mut libc::c_char,
                            );
                            let mut end: *mut node_t = get_node_by_name(
                                graph,
                                &raw mut end_city as *mut libc::c_char,
                            );
                            if start.is_null() {
                                printf(
                                    b"City '%s' not found\n\0" as *const u8
                                        as *const libc::c_char,
                                    &raw mut start_city as *mut libc::c_char,
                                );
                            } else if end.is_null() {
                                printf(
                                    b"City '%s' not found\n\0" as *const u8
                                        as *const libc::c_char,
                                    &raw mut end_city as *mut libc::c_char,
                                );
                            } else {
                                let mut path_length: libc::c_int = 0;
                                let mut path: *mut *mut node_t =
                                    find_shortest_path(start, end, &raw mut path_length);
                                if !path.is_null() {
                                    printf(
                                        b"Shortest path from %s to %s:\n\0" as *const u8
                                            as *const libc::c_char,
                                        &raw mut start_city as *mut libc::c_char,
                                        &raw mut end_city as *mut libc::c_char,
                                    );
                                    let mut i: libc::c_int = 0 as libc::c_int;
                                    while i < path_length {
                                        printf(
                                            b"  %d. %s\n\0" as *const u8
                                                as *const libc::c_char,
                                            i + 1 as libc::c_int,
                                            &raw mut (**path.offset(i as isize)).city_name
                                                as *mut libc::c_char,
                                        );
                                        i += 1;
                                    }
                                    free(path as *mut libc::c_void);
                                } else {
                                    printf(
                                        b"No path found\n\0" as *const u8
                                            as *const libc::c_char,
                                    );
                                }
                            }
                        }
                    }
                }
                6 => {
                    printf(
                        b"Enter start city for shallow copy: \0" as *const u8
                            as *const libc::c_char,
                    );
                    if !fgets(
                        &raw mut input as *mut libc::c_char,
                        MAX_INPUT,
                        stdin as *mut FILE,
                    )
                    .is_null()
                    {
                        input[strcspn(
                            &raw mut input as *mut libc::c_char,
                            b"\n\0" as *const u8 as *const libc::c_char,
                        ) as usize] = 0 as libc::c_char;
                        let mut node_1: *mut node_t =
                            get_node_by_name(graph, &raw mut input as *mut libc::c_char);
                        if node_1.is_null() {
                            printf(
                                b"City '%s' not found\n\0" as *const u8
                                    as *const libc::c_char,
                                &raw mut input as *mut libc::c_char,
                            );
                        } else {
                            let mut copy: *mut node_t = shallow_copy(node_1);
                            if !copy.is_null() {
                                printf(
                                    b"Created shallow copy starting from %s\n\0" as *const u8
                                        as *const libc::c_char,
                                    &raw mut input as *mut libc::c_char,
                                );
                                printf(
                                    b"Reference counts incremented for all reachable nodes\n\0"
                                        as *const u8
                                        as *const libc::c_char,
                                );
                                print_node(copy);
                            } else {
                                printf(
                                    b"Failed to create shallow copy\n\0" as *const u8
                                        as *const libc::c_char,
                                );
                            }
                        }
                    }
                }
                7 => {
                    printf(
                        b"Enter city name to delete: \0" as *const u8 as *const libc::c_char,
                    );
                    if !fgets(
                        &raw mut input as *mut libc::c_char,
                        MAX_INPUT,
                        stdin as *mut FILE,
                    )
                    .is_null()
                    {
                        input[strcspn(
                            &raw mut input as *mut libc::c_char,
                            b"\n\0" as *const u8 as *const libc::c_char,
                        ) as usize] = 0 as libc::c_char;
                        let mut node_2: *mut node_t =
                            get_node_by_name(graph, &raw mut input as *mut libc::c_char);
                        if node_2.is_null() {
                            printf(
                                b"City '%s' not found\n\0" as *const u8
                                    as *const libc::c_char,
                                &raw mut input as *mut libc::c_char,
                            );
                        } else {
                            printf(
                                b"Current ref count: %d\n\0" as *const u8
                                    as *const libc::c_char,
                                (*node_2).ref_count,
                            );
                            delete_node(node_2);
                            printf(
                                b"Decremented reference count for %s\n\0" as *const u8
                                    as *const libc::c_char,
                                &raw mut input as *mut libc::c_char,
                            );
                            printf(
                                b"Note: Node will be freed when ref count reaches 0\n\0"
                                    as *const u8
                                    as *const libc::c_char,
                            );
                        }
                    }
                }
                8 => {
                    printf(
                        b"Freeing graph and exiting...\n\0" as *const u8
                            as *const libc::c_char,
                    );
                    free_graph(graph);
                    return 0 as libc::c_int;
                }
                _ => {
                    printf(b"Invalid choice\n\0" as *const u8 as *const libc::c_char);
                }
            }
        }
    }
    free_graph(graph);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
