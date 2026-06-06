use crate::psbt::{
    psbt_decode, psbt_encode, psbt_geterr, psbt_init, psbt_read, psbt_state_tostr,
    psbt_txelem_type_tostr, psbt_type_tostr, PsbtElem, Psbt, PsbtEncoding, PsbtResult,
};
use crate::tx::PsbtTxElem;

fn hex_print(out: &mut String, data: &[u8]) {
    for b in data {
        out.push_str(&format!("{:02x}", b));
    }
}

fn txid_print(out: &mut String, data: &[u8]) {
    for i in (0..32).rev() {
        if i < data.len() {
            out.push_str(&format!("{:02x}", data[i]));
        }
    }
}

fn print_rec(elem: &mut PsbtElem, user_data: &mut dyn std::any::Any) {
    let buf = match user_data.downcast_mut::<String>() {
        Some(b) => b,
        None => return,
    };
    match elem {
        PsbtElem::Record { index, record } => {
            let type_str = psbt_type_tostr(record.record_type, record.scope.clone());
            buf.push_str(&format!("{}\t{} ", type_str, index));
            if !record.key.is_empty() {
                hex_print(buf, &record.key);
                buf.push(' ');
            }
            hex_print(buf, &record.val);
            buf.push('\n');
        }
        PsbtElem::TxElem { index: _, txelem } => match txelem {
            PsbtTxElem::TxIn(txin) => {
                buf.push_str(&format!(
                    "{}\t",
                    psbt_txelem_type_tostr(crate::psbt::PsbtTxElemType::TxIn)
                ));
                txid_print(buf, &txin.txid);
                buf.push_str(&format!(
                    " ind:{} seq:{}",
                    txin.index, txin.sequence_number
                ));
                if !txin.script.is_empty() {
                    buf.push(' ');
                    hex_print(buf, &txin.script);
                }
                buf.push('\n');
            }
            PsbtTxElem::TxOut(txout) => {
                buf.push_str(&format!(
                    "{}\t",
                    psbt_txelem_type_tostr(crate::psbt::PsbtTxElemType::TxOut)
                ));
                if !txout.script.is_empty() {
                    hex_print(buf, &txout.script);
                    buf.push(' ');
                }
                buf.push_str(&format!("amount:{}\n", txout.amount));
            }
            PsbtTxElem::Tx(tx) => {
                buf.push_str(&format!(
                    "{}\tver:{} locktime:{}\n",
                    psbt_txelem_type_tostr(crate::psbt::PsbtTxElemType::Tx),
                    tx.version,
                    tx.lock_time
                ));
            }
            PsbtTxElem::WitnessItem(_) => {}
        },
    }
}

fn usage() -> i32 {
    eprintln!("usage: psbt <psbt>");
    1
}

#[allow(dead_code)]
fn main() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return usage();
    }

    let psbt_arg = &args[1];
    let psbt_hex_len = psbt_arg.len();

    let mut buffer = vec![0u8; 4096];
    let mut psbt = Psbt::new(4096);
    let mut psbt_len: usize = 0;

    if psbt_init(&mut psbt, &mut buffer, 4096) != PsbtResult::Ok {
        eprintln!("init failed: {}", psbt_geterr());
        return 1;
    }

    let res = psbt_decode(psbt_arg, psbt_hex_len, &mut buffer, 4096, &mut psbt_len);
    if res != PsbtResult::Ok {
        eprintln!(
            "error ({:?}): {}. last_state = {}",
            res,
            psbt_geterr(),
            psbt_state_tostr(crate::psbt::PsbtState::Init)
        );
        return 1;
    }

    let mut user_data: String = String::new();
    let res = psbt_read(
        &buffer[..psbt_len],
        psbt_len,
        &mut psbt,
        Some(print_rec),
        &mut user_data,
    );
    if res != PsbtResult::Ok {
        eprintln!("error ({:?}): {}", res, psbt_geterr());
        return 1;
    }
    print!("{}", user_data);

    let mut out_len: usize = 0;
    let res = psbt_encode(
        &psbt,
        PsbtEncoding::Base62,
        &mut buffer,
        4096,
        &mut out_len,
    );
    if res != PsbtResult::Ok {
        eprintln!("error ({:?}): {}", res, psbt_geterr());
        return 1;
    }

    0
}
