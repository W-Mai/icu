pub mod common;

use crate::midata::MiData;

pub trait EnDecoder {
    fn encode(&self, data: &MiData) -> Vec<u8>;
    fn decode(&self, data: Vec<u8>) -> MiData;
}
