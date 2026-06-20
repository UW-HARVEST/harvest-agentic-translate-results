use crate::encoder_interface::EncoderInterface;
use crate::output_interface::OutputInterface;
pub fn range_encoder_new(enc: &mut dyn EncoderInterface, output: &mut dyn OutputInterface) {
    let _ = enc;
    let _ = output;
}
pub fn range_encoder_free(enc: &mut dyn EncoderInterface) {
    let _ = enc;
}
