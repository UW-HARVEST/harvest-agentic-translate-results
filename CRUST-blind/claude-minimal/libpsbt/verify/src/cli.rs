use crate::psbt::{
    psbt_decode, psbt_encode, psbt_geterr, psbt_init, psbt_read, psbt_state_tostr, Psbt,
    PsbtEncoding, PsbtResult,
};

fn main() -> i32 {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("usage: psbt <psbt>");
        return 1;
    }

    let mut psbt = Psbt::new(4096);
    let mut buffer = vec![0u8; 4096];
    let psbt_hex = &args[1];

    let res = psbt_init(&mut psbt, &mut buffer, 4096);
    if res != PsbtResult::Ok {
        println!(
            "error ({:?}): {}. last_state = {}",
            res,
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    let mut psbt_len: usize = 0;
    let res = psbt_decode(
        psbt_hex,
        psbt_hex.len(),
        &mut buffer,
        4096,
        &mut psbt_len,
    );
    if res != PsbtResult::Ok {
        println!(
            "error ({:?}): {}. last_state = INIT",
            res,
            psbt_geterr()
        );
        return 1;
    }

    let src = buffer[..psbt_len].to_vec();
    let mut user_data: () = ();
    let res = psbt_read(&src, psbt_len, &mut psbt, None, &mut user_data);
    if res != PsbtResult::Ok {
        println!(
            "error ({:?}): {}. last_state = {}",
            res,
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    let mut out_len: usize = 0;
    let res = psbt_encode(
        &psbt,
        PsbtEncoding::Base62,
        &mut buffer,
        4096,
        &mut out_len,
    );
    if res != PsbtResult::Ok {
        println!(
            "error ({:?}): {}. last_state = {}",
            res,
            psbt_geterr(),
            psbt_state_tostr(psbt.state)
        );
        return 1;
    }

    0
}
