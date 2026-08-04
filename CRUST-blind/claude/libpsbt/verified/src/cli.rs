use crate::psbt::{
    psbt_decode, psbt_encode, psbt_init, psbt_read, Psbt, PsbtEncoding, PsbtResult,
};

#[allow(dead_code)]
fn usage() -> i32 {
    println!("usage: psbt <psbt>");
    1
}

#[allow(dead_code)]
fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return usage();
    }

    let mut buffer = vec![0u8; 4096];
    let mut psbt = Psbt::new(4096);

    if psbt_init(&mut psbt, &mut buffer, 4096) != PsbtResult::Ok {
        return 1;
    }

    let arg = &args[1];
    let mut psbt_len: usize = 0;
    let res = psbt_decode(arg, arg.len(), &mut buffer, 4096, &mut psbt_len);
    if res != PsbtResult::Ok {
        return 1;
    }

    let data = buffer[..psbt_len].to_vec();
    let res = psbt_read(&data, psbt_len, &mut psbt, None, &mut ());
    if res != PsbtResult::Ok {
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
        return 1;
    }

    0
}

fn main() -> i32 {
    run()
}
