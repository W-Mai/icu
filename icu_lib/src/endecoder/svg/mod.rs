pub mod export;
pub mod import;

use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::{MiData, SceneData};
use crate::EncoderParams;
use serde_json::json;

pub struct Svg;

impl EnDecoder for Svg {
    fn can_decode(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        let head = data.iter().take(64).copied().collect::<Vec<_>>();
        let s = String::from_utf8_lossy(&head);
        s.contains("<svg") || s.contains("<?xml")
    }

    fn encode(&self, data: &MiData, _params: EncoderParams) -> Vec<u8> {
        match data {
            MiData::PATH(sd) => export::scene_to_svg(&sd.scene, 0, 0).into_bytes(),
            _ => Vec::new(),
        }
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        let scene = import::svg_to_scene(&data);
        MiData::PATH(SceneData { scene })
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        let scene = import::svg_to_scene(data);
        ImageInfo {
            width: 0,
            height: 0,
            data_size: data.len() as u32,
            format: "svg".to_string(),
            other_info: json!({
                "layout": "svg",
                "op_count": scene.ops.len(),
            }),
        }
    }
}
