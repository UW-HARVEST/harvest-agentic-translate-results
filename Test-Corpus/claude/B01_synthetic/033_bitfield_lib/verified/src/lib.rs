use std::ffi::c_int;
use std::os::raw::c_uint;

// Binary layout of C's:
//   typedef struct {
//       unsigned int x : 2;
//       unsigned int y : 3;
//       bool b : 1;
//       int z;
//   } foo_t;
//
// Verified layout (sizeof = 8, offsetof(z) = 4):
//   byte 0: bits 0..1 = x, bits 2..4 = y, bit 5 = b, bits 6..7 = padding
//   bytes 1..3: padding
//   bytes 4..7: z (int)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Foo {
    /// Packed byte holding x (bits 0..1), y (bits 2..4), b (bit 5).
    packed: u8,
    _pad: [u8; 3],
    z: c_int,
}

impl Foo {
    fn new(x: c_uint, y: c_uint, b: bool, z: c_int) -> Self {
        let xv = (x & 0x3) as u8;
        let yv = (y & 0x7) as u8;
        let bv = if b { 1u8 } else { 0u8 };
        let packed = xv | (yv << 2) | (bv << 5);
        Foo {
            packed,
            _pad: [0; 3],
            z,
        }
    }

    fn x(&self) -> c_uint {
        (self.packed & 0x3) as c_uint
    }

    fn y(&self) -> c_uint {
        ((self.packed >> 2) & 0x7) as c_uint
    }

    fn b(&self) -> bool {
        ((self.packed >> 5) & 0x1) != 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const Foo) {
    // Mimic C printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    // The C bool bit-field is promoted to int via %d.
    let foo = unsafe { &*foo };
    let b_int: c_int = if foo.b() { 1 } else { 0 };
    println!("{} {} {} {}", foo.x(), foo.y(), b_int, foo.z);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    let foo = Foo::new(x, y, b, z);
    unsafe {
        print_foo(&foo as *const Foo);
    }
}
