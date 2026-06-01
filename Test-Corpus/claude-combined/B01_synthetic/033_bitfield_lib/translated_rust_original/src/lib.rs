use std::ffi::{c_int, c_uint};

extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

// Mirrors the C struct
//   typedef struct {
//       unsigned int x : 2;
//       unsigned int y : 3;
//       bool b : 1;
//       int z;
//   } foo_t;
//
// On x86-64 Linux (System V ABI / Itanium), the bitfields are packed
// least-significant-bit-first into a single 32-bit allocation unit:
//   bits 0..=1  -> x
//   bits 2..=4  -> y
//   bits 5      -> b
// followed by a 4-byte int `z`. Total size: 8 bytes, alignment: 4.
#[repr(C)]
#[repr(align(4))]
pub struct FooT {
    bits: u32,
    z: c_int,
}

impl FooT {
    #[inline]
    fn x(&self) -> c_uint {
        (self.bits & 0x3) as c_uint
    }
    #[inline]
    fn y(&self) -> c_uint {
        ((self.bits >> 2) & 0x7) as c_uint
    }
    #[inline]
    fn b(&self) -> bool {
        ((self.bits >> 5) & 0x1) != 0
    }
}

/// Mirrors `void print_foo(const foo_t *foo)` from driver.c.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const FooT) {
    let foo = unsafe { &*foo };
    // C: printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    // foo->b is a bool bitfield; in C it's promoted to int when passed via varargs.
    let fmt = b"%u %u %d %d\n\0";
    let b_int: c_int = if foo.b() { 1 } else { 0 };
    unsafe {
        printf(fmt.as_ptr(), foo.x(), foo.y(), b_int, foo.z);
    }
}

/// Mirrors `void driver(unsigned int x, unsigned int y, bool b, int z)`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    // Pack bitfields the same way the C compiler would:
    //   x:2 in bits 0..=1, y:3 in bits 2..=4, b:1 in bit 5.
    // Truncate to the bitfield width (matches C's implementation-defined
    // truncation for unsigned bitfields, which both gcc and clang do).
    let x_bits = (x as u32) & 0x3;
    let y_bits = (y as u32) & 0x7;
    let b_bit: u32 = if b { 1 } else { 0 };
    let bits = x_bits | (y_bits << 2) | (b_bit << 5);
    let foo = FooT { bits, z };
    unsafe {
        print_foo(&foo as *const FooT);
    }
}
