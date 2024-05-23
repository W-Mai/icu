use crate::endecoder::svg::SVG;
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;

impl EnDecoder for SVG {
    fn can_decode(&self, data: &[u8]) -> bool {
        let options = usvg::Options::default();
        let fontdb = usvg::fontdb::Database::new();
        usvg::Tree::from_data(data, &options, &fontdb).is_ok()
    }

    fn encode(&self, data: &MiData, _encoder_params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::PATH(tree) => {
                let write_options = usvg::WriteOptions {
                    indent: usvg::Indent::None,
                    ..Default::default()
                };
                let svg_data = tree.to_string(&write_options);
                svg_data.into_bytes()
            }
            _ => panic!("SVG encoder can only encode MiData::PATH"),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        let options = usvg::Options::default();
        let fontdb = usvg::fontdb::Database::new();
        MiData::PATH(usvg::Tree::from_data(&data, &options, &fontdb).unwrap())
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
