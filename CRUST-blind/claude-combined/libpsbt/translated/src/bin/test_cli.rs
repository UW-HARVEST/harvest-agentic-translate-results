// The cli module is a binary entry point in the C codebase. Its only public
// surface area is a `main` function that the original C `cli.c` provides.
// In the Rust translation, `cli::main` is private (cannot be re-exported as
// `pub fn main`), so we exercise it indirectly by ensuring the module exists
// and compiles. We test the empty_inputs base64 PSBT example used by the C cli.

use libpsbt::psbt::{psbt_decode, psbt_init, psbt_read, Psbt, PsbtResult, PsbtState};

const EMPTY_INPUTS_B64: &str = "cHNidP8BACoCAAAAAAGA8PoCAAAAABepFCufG2xKKzFR7+3XGjiAZPO/VDBkhwAAAAAAAA==";

#[test]
fn test_cli_empty_inputs_psbt_decode_and_read() {
    // This is the exercise the C cli's main flow performs:
    // 1. psbt_decode the input
    // 2. psbt_init + psbt_read
    let mut buf = vec![0u8; 2048];
    let mut plen = 0usize;
    let res = psbt_decode(EMPTY_INPUTS_B64, EMPTY_INPUTS_B64.len(), &mut buf, 2048, &mut plen);
    assert_eq!(res, PsbtResult::Ok);
    // C's psbt_decode of this string yields 52 bytes
    assert_eq!(plen, 52);

    let mut psbt = Psbt::new(2048);
    let mut intbuf = vec![0u8; 2048];
    psbt_init(&mut psbt, &mut intbuf, 2048);

    let mut nothing: i32 = 0;
    let res = psbt_read(&buf, plen, &mut psbt, None, &mut nothing);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(psbt.state, PsbtState::Finalized);
}

fn main() {}
