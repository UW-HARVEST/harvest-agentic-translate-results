use std::io::{self, Write};

pub const SHAPE_COUNT: i32 = 10;

pub struct Shape {
    pub type_id: i32,
    pub name: &'static [u8],
    art: &'static [&'static [u8]],
}

const TREE: &[&[u8]] = &[
    b"    /\\    ",
    b"   /  \\   ",
    b"  /____\\  ",
    b"  /    \\  ",
    b" /______\\ ",
    b"    ||    ",
    b"    ||    ",
];

const TRACTOR: &[&[u8]] = &[
    b"      ________     ",
    b"     |        |___ ",
    b"     |  []  []|   |",
    b"  ___|________|___|",
    b" /  o        o   \\",
    b"|___|        |___| ",
];

const HOUSE: &[&[u8]] = &[
    b"     /\\     ",
    b"    /  \\    ",
    b"   /____\\   ",
    b"   |    |   ",
    b"   | [] |   ",
    b"   |    |   ",
    b"   |____|   ",
];

const SUN: &[&[u8]] = &[
    b"  \\  |  / ",
    b"   \\ | /  ",
    b"--- (@) ---",
    b"   / | \\  ",
    b"  /  |  \\ ",
    b"          ",
    b"          ",
];

const CLOUD: &[&[u8]] = &[
    b"   _____       ",
    b"  /     \\_    ",
    b" /  ___  _\\  ",
    b"(__/   \\_)   ",
];

const FLOWER: &[&[u8]] = &[
    b"  \\|/  ",
    b" -(@)- ",
    b"  /|\\  ",
    b"   |   ",
    b"   |   ",
    b"  / \\  ",
    b" /   \\ ",
];

const CAR: &[&[u8]] = &[
    b"  ____       ",
    b" /|_||_\\____ ",
    b"( o     o  ) ",
    b" -----------  ",
];

const STAR: &[&[u8]] = &[
    b"    *    ",
    b"   ***   ",
    b"  *****  ",
    b" ******* ",
    b"*********",
];

const HEART: &[&[u8]] = &[
    b" *** ***  ",
    b"*********  ",
    b"*********  ",
    b" ******* ",
    b"  *****  ",
    b"   ***   ",
];

const RAINBOW: &[&[u8]] = &[
    b"      _______      ",
    b"    /         \\    ",
    b"   /           \\   ",
    b"  /             \\  ",
    b" /               \\ ",
];

const SHAPE_DATA: &[(&[u8], &[&[u8]])] = &[
    (b"Tree", TREE),
    (b"Tractor", TRACTOR),
    (b"House", HOUSE),
    (b"Sun", SUN),
    (b"Cloud", CLOUD),
    (b"Flower", FLOWER),
    (b"Car", CAR),
    (b"Star", STAR),
    (b"Heart", HEART),
    (b"Rainbow", RAINBOW),
];

pub struct ShapeManager {
    shapes: Vec<Box<Shape>>,
}

impl ShapeManager {
    pub fn new() -> Self {
        let shapes = SHAPE_DATA
            .iter()
            .enumerate()
            .map(|(type_id, &(name, art))| {
                Box::new(Shape {
                    type_id: type_id as i32,
                    name,
                    art,
                })
            })
            .collect();
        Self { shapes }
    }

    pub fn get(&self, type_id: i32) -> Option<&Shape> {
        usize::try_from(type_id)
            .ok()
            .and_then(|index| self.shapes.get(index))
            .map(Box::as_ref)
    }

    pub fn ptr(&self, type_id: i32) -> *const Shape {
        self.get(type_id).map_or(std::ptr::null(), |shape| shape)
    }
}

pub fn type_name(type_id: i32) -> &'static [u8] {
    usize::try_from(type_id)
        .ok()
        .and_then(|index| SHAPE_DATA.get(index))
        .map_or(b"Unknown", |(name, _)| *name)
}

pub fn print<W: Write>(out: &mut W, shape: Option<&Shape>) -> io::Result<()> {
    let Some(shape) = shape else {
        out.write_all(b"(null shape)\n")?;
        return Ok(());
    };

    out.write_all(shape.name)?;
    out.write_all(b":\n")?;
    for line in shape.art {
        out.write_all(line)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}
