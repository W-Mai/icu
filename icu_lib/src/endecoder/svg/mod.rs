use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;
use usvg;

pub struct SVG {}

impl EnDecoder for SVG {
    fn can_decode(&self, data: &[u8]) -> bool {
        let options = usvg::Options::default();
        let fontdb = usvg::fontdb::Database::new();
        usvg::Tree::from_data(data, &options, &fontdb).is_ok()
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        todo!()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        todo!()
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let options = usvg::Options::default();
        let fontdb = usvg::fontdb::Database::new();
        let tree = usvg::Tree::from_data(data, &options, &fontdb).unwrap();
        let view_box = tree.view_box();

        ImageInfo {
            width: view_box.rect.width() as u32,
            height: view_box.rect.height() as u32,
            data_size: data.len() as u32,
            format: "svg".to_string(),
            other_info: std::collections::HashMap::new(),
        }
    }
}
