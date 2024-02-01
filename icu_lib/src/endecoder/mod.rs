pub mod common;

use crate::midata::MiData;

pub trait EnDecoder {
    fn encode(data: &MiData) -> Vec<u8>;
    fn decode(data: Vec<u8>) -> MiData;
}
