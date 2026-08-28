//! Translation of shape.c / shape.h
//!
//! The C code allocates one `shape_t` per shape type up front (a singleton per
//! type) and compares shapes by pointer identity. Here a shape is identified by
//! its type index; pointer identity becomes index equality.

use crate::cio::{cprintf, Arg, Out};

pub const SHAPE_COUNT: i32 = 10;

/// `sizeof(shape_t)` is 2444 bytes, so glibc hands out chunks 0x9a0 bytes
/// apart. The base addresses below are what the original binary prints when
/// address randomisation is disabled; they differ between a terminal and a
/// pipe because glibc allocates the stdout buffer (`st_blksize`: 1024 for a
/// terminal, 4096 otherwise) before the shape singletons. The C program's
/// `%p` output is inherently environment dependent.
const HEAP_BASE_PIPE: usize = 0x4092b0;
const HEAP_BASE_TTY: usize = 0x4086b0;
const CHUNK_STRIDE: usize = 0x9a0;

fn heap_base() -> usize {
    use std::io::IsTerminal;
    use std::sync::OnceLock;
    static BASE: OnceLock<usize> = OnceLock::new();
    *BASE.get_or_init(|| {
        if std::io::stdout().is_terminal() {
            HEAP_BASE_TTY
        } else {
            HEAP_BASE_PIPE
        }
    })
}

pub struct Shape {
    pub name: &'static [u8],
    pub art: &'static [&'static str],
    pub height: i32,
}

/// Shape data, byte-for-byte as initialised in shape.c. Note that the `width`
/// field is never used for output, and rows past `height` are never printed.
pub static SHAPES: [Shape; SHAPE_COUNT as usize] = [
    // SHAPE_TREE
    Shape {
        name: b"Tree",
        height: 7,
        art: &[
            "    /\\    ",
            "   /  \\   ",
            "  /____\\  ",
            "  /    \\  ",
            " /______\\ ",
            "    ||    ",
            "    ||    ",
        ],
    },
    // SHAPE_TRACTOR
    Shape {
        name: b"Tractor",
        height: 6,
        art: &[
            "      ________     ",
            "     |        |___ ",
            "     |  []  []|   |",
            "  ___|________|___|",
            " /  o        o   \\",
            "|___|        |___| ",
        ],
    },
    // SHAPE_HOUSE
    Shape {
        name: b"House",
        height: 7,
        art: &[
            "     /\\     ",
            "    /  \\    ",
            "   /____\\   ",
            "   |    |   ",
            "   | [] |   ",
            "   |    |   ",
            "   |____|   ",
        ],
    },
    // SHAPE_SUN
    Shape {
        name: b"Sun",
        height: 7,
        art: &[
            "  \\  |  / ",
            "   \\ | /  ",
            "--- (@) ---",
            "   / | \\  ",
            "  /  |  \\ ",
            "          ",
            "          ",
        ],
    },
    // SHAPE_CLOUD
    Shape {
        name: b"Cloud",
        height: 4,
        art: &[
            "   _____       ",
            "  /     \\_    ",
            " /  ___  _\\  ",
            "(__/   \\_)   ",
        ],
    },
    // SHAPE_FLOWER
    Shape {
        name: b"Flower",
        height: 7,
        art: &[
            "  \\|/  ",
            " -(@)- ",
            "  /|\\  ",
            "   |   ",
            "   |   ",
            "  / \\  ",
            " /   \\ ",
        ],
    },
    // SHAPE_CAR
    Shape {
        name: b"Car",
        height: 4,
        art: &[
            "  ____       ",
            " /|_||_\\____ ",
            "( o     o  ) ",
            " -----------  ",
        ],
    },
    // SHAPE_STAR
    Shape {
        name: b"Star",
        height: 5,
        art: &[
            "    *    ",
            "   ***   ",
            "  *****  ",
            " ******* ",
            "*********",
        ],
    },
    // SHAPE_HEART
    Shape {
        name: b"Heart",
        height: 6,
        art: &[
            " *** ***  ",
            "*********  ",
            "*********  ",
            " ******* ",
            "  *****  ",
            "   ***   ",
        ],
    },
    // SHAPE_RAINBOW
    Shape {
        name: b"Rainbow",
        height: 5,
        art: &[
            "      _______      ",
            "    /         \\    ",
            "   /           \\   ",
            "  /             \\  ",
            " /               \\ ",
        ],
    },
];

/// `shape_get()`: returns `NULL` for out of range types.
pub fn shape_get(type_: i32) -> Option<usize> {
    if type_ < 0 || type_ >= SHAPE_COUNT {
        return None;
    }
    Some(type_ as usize)
}

/// The address `%p` would print for a shape singleton.
pub fn shape_ptr(idx: usize) -> usize {
    heap_base() + idx * CHUNK_STRIDE
}

pub fn shape_name(idx: usize) -> &'static [u8] {
    SHAPES[idx].name
}

/// `shape_print()`
pub fn shape_print(out: &mut Out, shape: Option<usize>) {
    let idx = match shape {
        None => {
            cprintf(out, b"(null shape)\n", &[]);
            return;
        }
        Some(i) => i,
    };

    let s = &SHAPES[idx];
    cprintf(out, b"%s:\n", &[Arg::S(s.name)]);
    let mut i = 0;
    while i < s.height {
        cprintf(out, b"%s\n", &[Arg::S(s.art[i as usize].as_bytes())]);
        i += 1;
    }
}

/// `shape_equals()`: equality is pointer identity.
pub fn shape_equals(s1: Option<usize>, s2: Option<usize>) -> bool {
    s1 == s2
}

/// `shape_type_name()`
pub fn shape_type_name(type_: i32) -> &'static [u8] {
    match type_ {
        0 => b"Tree",
        1 => b"Tractor",
        2 => b"House",
        3 => b"Sun",
        4 => b"Cloud",
        5 => b"Flower",
        6 => b"Car",
        7 => b"Star",
        8 => b"Heart",
        9 => b"Rainbow",
        _ => b"Unknown",
    }
}

/// Shape type of a singleton, i.e. `shape->type`.
pub fn shape_type(idx: usize) -> i32 {
    idx as i32
}
