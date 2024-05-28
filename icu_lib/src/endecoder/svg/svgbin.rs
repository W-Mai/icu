use crate::endecoder::svg::SVGBin;
use crate::endecoder::{EnDecoder, ImageInfo};
use crate::midata::MiData;
use crate::EncoderParams;
use image::EncodableLayout;
use std::mem::size_of_val;
use std::ops::Deref;
use usvg::tiny_skia_path::PathSegment;
use usvg::{Node, Paint};

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
    section_type: SectionType,
    section_flags: u8,
    section_length: u32,
    section_data: Vec<DataBlock>,
}

#[repr(C)]
enum DataBlock {
    FixedLength(Tag, Vec<u8>),
    VariableLength(Tag, u32, Vec<u8>),
}

#[derive(Copy, Clone)]
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

#[derive(Copy, Clone)]
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
            bytes.extend_from_slice(&(section.section_type as u8).to_le_bytes());
            bytes.extend_from_slice(&section.section_flags.to_le_bytes());
            bytes.extend_from_slice(&section.section_length.to_le_bytes());
            for data_block in &section.section_data {
                match data_block {
                    DataBlock::FixedLength(tag, data) => {
                        bytes.extend_from_slice(&(*tag as u8).to_le_bytes());
                        bytes.extend_from_slice(data);
                    }
                    DataBlock::VariableLength(tag, length, data) => {
                        bytes.extend_from_slice(&(*tag as u8).to_le_bytes());
                        bytes.extend_from_slice(&length.to_le_bytes());
                        bytes.extend_from_slice(data);
                    }
                }
            }
        }

        unsafe {
            let header = bytes.as_mut_ptr() as *mut FileHeader;
            (*header).data_size = bytes.len() as u32 - 40;
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

        let mut sections = vec![];
        // deal gradient
        for gradient in tree.linear_gradients() {
            let gradient_id = gradient.id().to_string();

            let mut section_data = vec![
                DataBlock::VariableLength(
                    Tag::Id,
                    gradient_id.len() as u32,
                    gradient_id.as_bytes().to_vec(),
                ),
                DataBlock::FixedLength(Tag::GradientStyle, 0u8.to_le_bytes().to_vec()),
                DataBlock::FixedLength(
                    Tag::GradientStopSpread,
                    (gradient.spread_method() as u8).to_le_bytes().to_vec(),
                ),
                DataBlock::FixedLength(Tag::X1, gradient.x1().to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::Y1, gradient.y1().to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::X2, gradient.x2().to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::Y2, gradient.y2().to_le_bytes().to_vec()),
            ];

            for stop in gradient.stops().iter() {
                section_data.push(DataBlock::FixedLength(
                    Tag::GradientStopOffset,
                    (stop.offset().get() as u8).to_le_bytes().to_vec(),
                ));

                let color_argb: u32 = ((stop.opacity().get() * 256f32) as u32) << 24
                    | (stop.color().red as u32) << 16
                    | (stop.color().green as u32) << 8
                    | (stop.color().blue as u32);
                section_data.push(DataBlock::FixedLength(
                    Tag::FillColor,
                    color_argb.to_le_bytes().to_vec(),
                ));
            }

            sections.push(Section {
                section_type: SectionType::Defs,
                section_flags: SFDefs::Gradient as u8,
                section_length: 0,
                section_data,
            });
        }
        for gradient in tree.radial_gradients() {
            let gradient_id = gradient.id().to_string();

            let mut section_data = vec![
                DataBlock::VariableLength(
                    Tag::Id,
                    gradient_id.len() as u32,
                    gradient_id.as_bytes().to_vec(),
                ),
                DataBlock::FixedLength(Tag::GradientStyle, 0u8.to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::GradientStopSpread, 0u8.to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::Cx, gradient.cx().to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::Cy, gradient.cy().to_le_bytes().to_vec()),
                DataBlock::FixedLength(Tag::R, gradient.r().get().to_le_bytes().to_vec()),
            ];
            for stop in gradient.stops().iter() {
                section_data.push(DataBlock::FixedLength(
                    Tag::GradientStopOffset,
                    (stop.offset().get() as u8).to_le_bytes().to_vec(),
                ));

                let color_argb: u32 = ((stop.opacity().get() * 256f32) as u32) << 24
                    | (stop.color().red as u32) << 16
                    | (stop.color().green as u32) << 8;
                section_data.push(DataBlock::FixedLength(
                    Tag::FillColor,
                    color_argb.to_le_bytes().to_vec(),
                ));
            }
            sections.push(Section {
                section_type: SectionType::Defs,
                section_flags: SFDefs::Gradient as u8,
                section_length: 0,
                section_data,
            });
        }

        let view_box = tree.view_box();

        let root = tree.root();
        //  前序遍历 使用 has_children 判断是否有子节点，然后遍历所有节点，并打印
        let mut node_stack: Vec<(&Node, usize)> = Vec::new();

        for child in root.children() {
            node_stack.push((child, 0));
            while let Some((node, index)) = node_stack.pop() {
                match node {
                    Node::Group(group) => {
                        // println!("{:?}", group);

                        let mut children_count = group.children().len();
                        while children_count > 0 {
                            children_count -= 1;
                            node_stack.push((&group.children()[children_count], 0));
                        }

                        if index < group.children().len() {
                            node_stack.push((node, index + 1));
                        }
                    }
                    Node::Path(path) => {
                        let mut section_data = vec![];
                        let mut path_data = vec![];
                        for segment in path.data().segments() {
                            match segment {
                                PathSegment::MoveTo(p) => {
                                    path_data.push(77u32);
                                    path_data.push(p.x.to_bits());
                                    path_data.push(p.y.to_bits());
                                }
                                PathSegment::LineTo(p) => {
                                    path_data.push(76u32);
                                    path_data.push(p.x.to_bits());
                                    path_data.push(p.y.to_bits());
                                }
                                PathSegment::QuadTo(p0, p1) => {
                                    path_data.push(67u32);
                                    path_data.push(p0.x.to_bits());
                                    path_data.push(p0.y.to_bits());
                                    path_data.push(p1.x.to_bits());
                                    path_data.push(p1.y.to_bits());
                                }
                                PathSegment::CubicTo(p0, p1, p2) => {
                                    path_data.push(81u32);
                                    path_data.push(p0.x.to_bits());
                                    path_data.push(p0.y.to_bits());
                                    path_data.push(p1.x.to_bits());
                                    path_data.push(p1.y.to_bits());
                                    path_data.push(p2.x.to_bits());
                                    path_data.push(p2.y.to_bits());
                                }
                                PathSegment::Close => {
                                    path_data.push(90u32);
                                }
                            }
                            
                            // match segment {
                            //     PathSegment::MoveTo(p) => {
                            //         path_data.push(0);
                            //         path_data.extend(p.x.to_bits().to_le_bytes());
                            //         path_data.extend(p.y.to_bits().to_le_bytes());
                            //     }
                            //     PathSegment::LineTo(p) => {
                            //         path_data.push(1);
                            //         path_data.extend(p.x.to_bits().to_le_bytes());
                            //         path_data.extend(p.y.to_bits().to_le_bytes());
                            //     }
                            //     PathSegment::QuadTo(p0, p1) => {
                            //         path_data.push(2);
                            //         path_data.extend(p0.x.to_bits().to_le_bytes());
                            //         path_data.extend(p0.y.to_bits().to_le_bytes());
                            //         path_data.extend(p1.x.to_bits().to_le_bytes());
                            //         path_data.extend(p1.y.to_bits().to_le_bytes());
                            //     }
                            //     PathSegment::CubicTo(p0, p1, p2) => {
                            //         path_data.push(3);
                            //         path_data.extend(p0.x.to_bits().to_le_bytes());
                            //         path_data.extend(p0.y.to_bits().to_le_bytes());
                            //         path_data.extend(p1.x.to_bits().to_le_bytes());
                            //         path_data.extend(p1.y.to_bits().to_le_bytes());
                            //         path_data.extend(p2.x.to_bits().to_le_bytes());
                            //         path_data.extend(p2.y.to_bits().to_le_bytes());
                            //     }
                            //     PathSegment::Close => {
                            //         path_data.push(4);
                            //     }
                            // }
                        }
                        let path_data = path_data
                            .into_iter()
                            .flat_map(|x| x.to_le_bytes())
                            .collect::<Vec<_>>();

                        section_data.push(DataBlock::VariableLength(
                            Tag::Path,
                            path_data.len() as u32,
                            path_data,
                        ));

                        if let Some(fill) = path.fill() {
                            match fill.paint() {
                                Paint::Color(color) => {}
                                Paint::LinearGradient(gradient) => {
                                    let gid = gradient.id();
                                    section_data.push(DataBlock::VariableLength(
                                        Tag::FillReferenceName,
                                        gid.len() as u32,
                                        gid.as_bytes().to_vec(),
                                    ))
                                }
                                Paint::RadialGradient(gradient) => {
                                    let gid = gradient.id();
                                    section_data.push(DataBlock::VariableLength(
                                        Tag::FillReferenceName,
                                        gid.len() as u32,
                                        gid.as_bytes().to_vec(),
                                    ))
                                }
                                Paint::Pattern(pattern) => {}
                            }
                        }

                        let trans = path.abs_transform();
                        if !trans.is_identity() {
                            section_data.push(DataBlock::VariableLength(
                                Tag::GlobalTransform,
                                36,
                                [
                                    trans.sx, trans.kx, trans.ky, trans.sy, trans.tx, trans.ty,
                                    0f32, 0f32, 1f32,
                                ]
                                .as_bytes()
                                .to_vec(),
                            ));
                        }

                        sections.push(Section {
                            section_type: SectionType::Path,
                            section_flags: 0,
                            section_length: 0,
                            section_data,
                        });
                    }
                    Node::Image(image) => {
                        // println!("{:?}", image);
                    }
                    Node::Text(text) => {
                        // println!("{:?}", text);
                    }
                }
            }
        }

        let header = FileHeader {
            signature: [b'V', b'e', b'l', b'a', b'V', b'G', 0, 0],
            version: 0x01_00_00_00u32.to_be(),
            num_sections: sections.len() as u32,
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
