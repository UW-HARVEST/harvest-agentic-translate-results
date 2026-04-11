pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type tflac_u8 = u8;
pub type tflac_u32 = u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tflac {
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub frame_header: u32,
    pub cur_blocksize: u32,
}
impl std::default::Default for tflac {
    fn default() -> Self {
        tflac {
        samplerate: u32::default(),
        channels: u32::default(),
        bitdepth: u32::default(),
        channel_mode: u8::default(),
        frame_header: u32::default(),
        cur_blocksize: u32::default()
        }
    }
}

pub const TFLAC_CHANNEL_MID_SIDE: TFLAC_CHANNEL_MODE = 3;
pub const TFLAC_CHANNEL_SIDE_RIGHT: TFLAC_CHANNEL_MODE = 2;
pub const TFLAC_CHANNEL_LEFT_SIDE: TFLAC_CHANNEL_MODE = 1;
pub const TFLAC_CHANNEL_INDEPENDENT: TFLAC_CHANNEL_MODE = 0;
pub type TFLAC_CHANNEL_MODE = libc::unix::c_uint;
pub const TFLAC_CHANNEL_MODE_COUNT: TFLAC_CHANNEL_MODE = 4;
#[no_mangle]
pub unsafe extern "C" fn update_frame_header<'a1>(mut t: Option<&'a1 mut crate::src::lib::tflac>) {
    (*borrow_mut(&mut t).unwrap()).frame_header = ((0xfff8 as libc::c_uint) << 16 as libc::c_int) as tflac_u32;
    match (*borrow(& t).unwrap()).cur_blocksize {
        192 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x1 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        576 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x2 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        1152 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x3 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        2304 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x4 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        4608 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x5 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        256 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x8 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        512 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x9 as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        1024 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xa as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        2048 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xb as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        4096 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xc as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        8192 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xd as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        16384 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xe as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        32768 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xf as libc::c_uint) << 12 as libc::c_int)
                as tflac_u32;
        }
        _ => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | if (*borrow(& t).unwrap()).cur_blocksize <= 256 as tflac_u32 {
                    (0x6 as libc::c_uint) << 12 as libc::c_int
                } else {
                    (0x7 as libc::c_uint) << 12 as libc::c_int
                }) as tflac_u32;
        }
    }
    match (*borrow(& t).unwrap()).samplerate {
        882000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x1 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        176400 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x2 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        192000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x3 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        8000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x4 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        16000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x5 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        22050 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x6 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        24000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x7 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        32000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x8 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        44100 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0x9 as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        48000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xa as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        96000 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (0xb as libc::c_uint) << 8 as libc::c_int)
                as tflac_u32;
        }
        _ => {
            if (*borrow_mut(&mut t).unwrap()).samplerate.wrapping_rem(1000 as tflac_u32) == 0 as tflac_u32 {
                if (*borrow_mut(&mut t).unwrap()).samplerate.wrapping_div(1000 as tflac_u32) < 256 as tflac_u32 {
                    (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                        | (0xc as libc::c_uint) << 8 as libc::c_int)
                        as tflac_u32;
                }
            } else if (*borrow(& t).unwrap()).samplerate < 65536 as tflac_u32 {
                (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                    | (0xd as libc::c_uint) << 8 as libc::c_int)
                    as tflac_u32;
            } else if (*borrow_mut(&mut t).unwrap()).samplerate.wrapping_rem(10 as tflac_u32) == 0 as tflac_u32 {
                if (*borrow_mut(&mut t).unwrap()).samplerate.wrapping_div(10 as tflac_u32) < 65536 as tflac_u32 {
                    (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                        | (0xe as libc::c_uint) << 8 as libc::c_int)
                        as tflac_u32;
                }
            }
        }
    }
    let mut mode: tflac_u8 =
        ((*borrow(& t).unwrap()).channel_mode as libc::c_int % 4 as libc::c_int) as tflac_u8;
    match mode as TFLAC_CHANNEL_MODE as libc::c_uint {
        0 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | ((*borrow_mut(&mut t).unwrap()).channels.wrapping_sub(1 as tflac_u32) << 4 as libc::c_int)
                    as libc::c_uint) as tflac_u32;
        }
        1 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | ((0x8 as libc::c_int) << 4 as libc::c_int) as libc::c_uint)
                as tflac_u32;
        }
        2 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | ((0x9 as libc::c_int) << 4 as libc::c_int) as libc::c_uint)
                as tflac_u32;
        }
        3 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | ((0xa as libc::c_int) << 4 as libc::c_int) as libc::c_uint)
                as tflac_u32;
        }
        _ => {}
    }
    match (*borrow(& t).unwrap()).bitdepth {
        8 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (1 as libc::c_uint) << 1 as libc::c_int)
                as tflac_u32;
        }
        12 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (2 as libc::c_uint) << 1 as libc::c_int)
                as tflac_u32;
        }
        16 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (4 as libc::c_uint) << 1 as libc::c_int)
                as tflac_u32;
        }
        20 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (5 as libc::c_uint) << 1 as libc::c_int)
                as tflac_u32;
        }
        24 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (6 as libc::c_uint) << 1 as libc::c_int)
                as tflac_u32;
        }
        32 => {
            (*borrow_mut(&mut t).unwrap()).frame_header = ((*borrow(& t).unwrap()).frame_header as libc::c_uint
                | (7 as libc::c_uint) << 1 as libc::c_int)
                as tflac_u32;
        }
        _ => {}
    };
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

