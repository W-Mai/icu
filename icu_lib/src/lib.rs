pub mod endecoder;
pub mod midata;

#[derive(Default)]
pub struct EncoderParams {
    pub stride_align: u32,
    pub dither: bool,
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
