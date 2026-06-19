use std::ffi::{c_uchar, c_uint, c_ushort};

type TflacU8 = c_uchar;
type TflacU16 = c_ushort;
type TflacU32 = c_uint;

const CRC16_POLY: u16 = 0x8005;
const CRC16_TABLES: [[u16; 256]; 8] = make_crc16_tables();

const fn make_crc16_tables() -> [[u16; 256]; 8] {
    let mut tables = [[0u16; 256]; 8];
    let mut i = 0usize;

    while i < 256 {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;

        while bit < 8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ CRC16_POLY;
            } else {
                crc <<= 1;
            }
            bit += 1;
        }

        tables[0][i] = crc;
        i += 1;
    }

    let mut table = 1usize;
    while table < 8 {
        i = 0;
        while i < 256 {
            let crc = tables[table - 1][i];
            tables[table][i] = (crc << 8) ^ tables[0][(crc >> 8) as usize];
            i += 1;
        }
        table += 1;
    }

    tables
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(d: *const TflacU8, mut len: TflacU32, mut crc16: TflacU16) -> TflacU16 {
    let mut p = d;

    while len >= 8 {
        unsafe {
            crc16 ^= ((*p.add(0) as u16) << 8) | (*p.add(1) as u16);
            crc16 = CRC16_TABLES[7][(crc16 >> 8) as usize]
                ^ CRC16_TABLES[6][(crc16 & 0xff) as usize]
                ^ CRC16_TABLES[5][*p.add(2) as usize]
                ^ CRC16_TABLES[4][*p.add(3) as usize]
                ^ CRC16_TABLES[3][*p.add(4) as usize]
                ^ CRC16_TABLES[2][*p.add(5) as usize]
                ^ CRC16_TABLES[1][*p.add(6) as usize]
                ^ CRC16_TABLES[0][*p.add(7) as usize];
            p = p.add(8);
        }
        len -= 8;
    }

    while len != 0 {
        unsafe {
            crc16 = (crc16 << 8) ^ CRC16_TABLES[0][((crc16 >> 8) ^ (*p as u16)) as usize];
            p = p.add(1);
        }
        len -= 1;
    }

    crc16
}
