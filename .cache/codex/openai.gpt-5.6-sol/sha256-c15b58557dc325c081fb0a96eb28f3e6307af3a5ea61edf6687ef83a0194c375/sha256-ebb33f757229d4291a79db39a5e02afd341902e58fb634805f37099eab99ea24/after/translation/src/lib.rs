use std::ffi::{c_uchar, c_uint, c_ushort};

const TABLES: [[u16; 256]; 8] = make_tables();

const fn make_tables() -> [[u16; 256]; 8] {
    let mut tables = [[0; 256]; 8];
    let mut index = 0;

    while index < 256 {
        let mut crc = (index as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
            bit += 1;
        }
        tables[0][index] = crc;
        index += 1;
    }

    let mut slice = 1;
    while slice < 8 {
        index = 0;
        while index < 256 {
            let previous = tables[slice - 1][index];
            tables[slice][index] = (previous << 8) ^ tables[0][(previous >> 8) as usize];
            index += 1;
        }
        slice += 1;
    }

    tables
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(
    mut d: *const c_uchar,
    mut len: c_uint,
    mut crc16: c_ushort,
) -> c_ushort {
    while len >= 8 {
        let bytes = unsafe { std::slice::from_raw_parts(d, 8) };
        crc16 ^= u16::from(bytes[0]) << 8 | u16::from(bytes[1]);
        crc16 = TABLES[7][usize::from(crc16 >> 8)]
            ^ TABLES[6][usize::from(crc16 & 0xff)]
            ^ TABLES[5][usize::from(bytes[2])]
            ^ TABLES[4][usize::from(bytes[3])]
            ^ TABLES[3][usize::from(bytes[4])]
            ^ TABLES[2][usize::from(bytes[5])]
            ^ TABLES[1][usize::from(bytes[6])]
            ^ TABLES[0][usize::from(bytes[7])];
        d = unsafe { d.add(8) };
        len -= 8;
    }

    while len != 0 {
        let byte = unsafe { *d };
        crc16 = (crc16 << 8) ^ TABLES[0][usize::from((crc16 >> 8) ^ u16::from(byte))];
        d = unsafe { d.add(1) };
        len -= 1;
    }

    crc16
}
