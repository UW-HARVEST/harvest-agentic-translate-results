use crate::psbt::{
    psbt_decode, psbt_encode, psbt_geterr, psbt_init, psbt_read, psbt_state_tostr, Psbt,
    PsbtEncoding, PsbtResult,
};

/// Command-line entry point translated from c_src/cli.c.
/// Returns 0 on success, 1 on error.
pub fn main() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("usage: psbt <psbt>");
        return 1;
    }

    let mut psbt = Psbt::new(4096);
    let mut buffer = vec![0u8; 4096];
    let mut psbt_len: usize = 0;
    let mut out_len: usize = 0;

    let psbt_hex = &args[1];
    let psbt_hex_len = psbt_hex.len();

    let res = psbt_init(&mut psbt, &mut buffer, 4096);
    if res != PsbtResult::Ok {
        println!(
            "error: {}. last_state = {}",
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    let res = psbt_decode(psbt_hex, psbt_hex_len, &mut buffer, 4096, &mut psbt_len);
    if res != PsbtResult::Ok {
        println!(
            "error: {}. last_state = {}",
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    let res = psbt_read(&buffer, psbt_len, &mut psbt, None, &mut ());
    if res != PsbtResult::Ok {
        println!(
            "error: {}. last_state = {}",
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    let res = psbt_encode(
        &psbt,
        PsbtEncoding::Base62,
        &mut buffer,
        4096,
        &mut out_len,
    );
    if res != PsbtResult::Ok {
        println!(
            "error: {}. last_state = {}",
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    0
}
