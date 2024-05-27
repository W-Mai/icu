use crate::endecoder::svg::SVGBin;
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;

#[repr(C, packed)]
struct FileHeader {
    signature: [u8; 8],
    version: u32,
    num_sections: u32,
    width: u32,
    height: u32,
    data_size: u32,
    compress_flag: u8,
    reserved_bytes: [u8; 3],
    uncompress_data_size: u32,
    uncompress_data_bits: u32,
}

#[repr(C)]
struct Section {
    section_type: u8,
    section_flags: u8,
    section_length: u32,
    section_data: Vec<DataBlock>,
}

#[repr(C)]
enum DataBlock {
    FixedLength(u8, Vec<u8>),
    VariableLength(u8, u32, Vec<u8>),
}

#[repr(C)]
enum SectionType {
    Defs = 0x01,
    Viewport = 0x02,
    Group = 0x03,
    Use = 0x04,
    Shapes = 0x05,
    Path = 0x06,
    Image = 0x07,
    Text = 0x08,
}

#[repr(C)]
enum SFDefs {
    Gradient = 0x10,
    SolidColor = 0x20,
}

#[repr(C)]
enum SFShapes {
    Rect = 0x10,
    Circle = 0x20,
    Ellipse = 0x30,
    Line = 0x40,
}

#[repr(C)]
enum SFText {
    Text = 0x00,
    Content = 0x10,
    TSpan = 0x20,
}

#[repr(u8)]
enum Tag {
    Count = 0x04,
    Id = 0x05,
    ViewportFill = 0x06,
    FillColor = 0x10,
    FillOpacity = 0x11,
    FillRule = 0x12,
    FillTransform = 0x13,
    StrokeColor = 0x20,
    StrokeOpacity = 0x21,
    StrokeWidth = 0x22,
    StrokeCap = 0x23,
    StrokeJoin = 0x24,
    StrokeMiterLimit = 0x25,
    StrokeTransform = 0x26,
    StrokeDash = 0x27,
    GlobalTransform = 0x31,
    BlendMode = 0x32,
    ScissorArea = 0x33,
    FillReferenceName = 0x34,
    StrokeReferenceName = 0x35,
    GradientStyle = 0x36,
    GradientStopSpread = 0x37,
    GradientStopOffset = 0x38,
    X = 0x40,
    Y = 0x41,
    Width = 0x42,
    Height = 0x43,
    Cx = 0x44,
    Cy = 0x45,
    R = 0x46,
    Rx = 0x47,
    Ry = 0x48,
    X1 = 0x49,
    Y1 = 0x4A,
    X2 = 0x4B,
    Y2 = 0x4C,
    Path = 0x60,
    BoundRect = 0x61,
    XLink = 0x70,
    Ratio = 0x71,
    FontFamily = 0x80,
    FontSize = 0x81,
    FontStyle = 0x82,
    TextContents = 0x83,
}

#[repr(C)]
struct SVGBinFile {
    header: FileHeader,
    sections: Vec<Section>,
}

impl SVGBinFile {
    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend_from_slice(&self.header.signature);
        bytes.extend_from_slice(&self.header.version.to_le_bytes());
        bytes.extend_from_slice(&self.header.num_sections.to_le_bytes());
        bytes.extend_from_slice(&self.header.width.to_le_bytes());
        bytes.extend_from_slice(&self.header.height.to_le_bytes());
        bytes.extend_from_slice(&self.header.data_size.to_le_bytes());
        bytes.extend_from_slice(&self.header.compress_flag.to_le_bytes());
        bytes.extend_from_slice(&self.header.reserved_bytes);
        bytes.extend_from_slice(&self.header.uncompress_data_size.to_le_bytes());
        bytes.extend_from_slice(&self.header.uncompress_data_bits.to_le_bytes());
        for section in &self.sections {
            bytes.extend_from_slice(&section.section_type.to_le_bytes());
            bytes.extend_from_slice(&section.section_flags.to_le_bytes());
            bytes.extend_from_slice(&section.section_length.to_le_bytes());
            for data_block in &section.section_data {
                match data_block {
                    DataBlock::FixedLength(tag, data) => {
                        bytes.extend_from_slice(&tag.to_le_bytes());
                        bytes.extend_from_slice(data);
                    }
                    DataBlock::VariableLength(tag, length, data) => {
                        bytes.extend_from_slice(&tag.to_le_bytes());
                        bytes.extend_from_slice(&length.to_le_bytes());
                        bytes.extend_from_slice(data);
                    }
                }
            }
        }
        bytes
    }
}

impl EnDecoder for SVGBin {
    fn can_decode(&self, data: &[u8]) -> bool {
        if data.len() < 12 {
            return false;
        }

        &data[0..8] == b"VelaVG\x00\x00"
            && u32::from_le_bytes((&data[8..12]).try_into().unwrap()) == 0x01_00_00_00
    }

    fn encode(&self, data: &MiData, encoder_params: EncoderParams) -> Vec<u8> {
        let tree = match data {
            MiData::PATH(tree) => tree,
            _ => {
                return vec![];
            }
        };

        let view_box = tree.view_box();

        let sections = vec![];

        let header = FileHeader {
            signature: [b'V', b'e', b'l', b'a', b'V', b'G', 0, 0],
            version: 0x01_00_00_00,
            num_sections: 0,
            width: view_box.rect.width() as u32,
            height: view_box.rect.height() as u32,
            data_size: 0,
            compress_flag: 0,
            reserved_bytes: [0; 3],
            uncompress_data_size: 0,
            uncompress_data_bits: 0,
        };

        let file = SVGBinFile { header, sections };

        file.into_bytes()
    }

    fn decode(&self, data: Vec<u8>) -> MiData {
        todo!()
    }

    fn info(&self, data: &[u8]) -> ImageInfo {
        todo!()
    }
}
