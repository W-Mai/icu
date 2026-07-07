#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColorFormat {
    #[default]
    RGB565,
    I1,
    I2,
    I4,
    I8,
    A1,
    A2,
    A4,
    A8,
    L8,
    RGB565Swapped,
    RGB565A8,
    RGB888,
    XRGB8888,
    ARGB8888,
    RGBA8888,
    BGRA8888,
}

impl ColorFormat {
    pub fn to_mirx(self) -> Option<mirx::ColorFormat> {
        match self {
            Self::RGB565 => Some(mirx::ColorFormat::RGB565),
            Self::RGB565Swapped => Some(mirx::ColorFormat::RGB565Swapped),
            Self::RGB888 => Some(mirx::ColorFormat::RGB888),
            Self::RGBA8888 | Self::XRGB8888 => Some(mirx::ColorFormat::RGBA8888),
            Self::BGRA8888 => Some(mirx::ColorFormat::BGRA8888),
            _ => None,
        }
    }
}

impl From<ColorFormat> for mirx::ColorFormat {
    fn from(cf: ColorFormat) -> Self {
        cf.to_mirx().unwrap_or(mirx::ColorFormat::RGB565)
    }
}

impl From<ColorFormat> for crate::endecoder::lvgl::ColorFormat {
    fn from(cf: ColorFormat) -> Self {
        match cf {
            ColorFormat::I1 => Self::I1,
            ColorFormat::I2 => Self::I2,
            ColorFormat::I4 => Self::I4,
            ColorFormat::I8 => Self::I8,
            ColorFormat::A1 => Self::A1,
            ColorFormat::A2 => Self::A2,
            ColorFormat::A4 => Self::A4,
            ColorFormat::A8 => Self::A8,
            ColorFormat::L8 => Self::L8,
            ColorFormat::RGB565 => Self::RGB565,
            ColorFormat::RGB565A8 => Self::RGB565A8,
            ColorFormat::RGB888 => Self::RGB888,
            ColorFormat::XRGB8888 => Self::XRGB8888,
            ColorFormat::ARGB8888 => Self::ARGB8888,
            ColorFormat::RGB565Swapped
            | ColorFormat::RGBA8888
            | ColorFormat::BGRA8888 => Self::UNKNOWN,
        }
    }
}
