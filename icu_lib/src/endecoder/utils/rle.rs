#[derive(Debug)]
pub enum RleError {
    InvalidBlockSize,
    InvalidInput,
}

type Result<T> = std::result::Result<T, RleError>;

/// RLE (Run-Length Encoding) encoder/decoder
#[derive(Debug, Clone)]
pub struct RleCoder {
    block_size: usize,
}

impl RleCoder {
    pub fn new(block_size: usize) -> Result<Self> {
        if block_size == 0 {
            return Err(RleError::InvalidBlockSize);
        }
        Ok(Self { block_size })
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() % self.block_size != 0 {
            return Err(RleError::InvalidInput);
        }

        let mut result = Vec::new();
        let blocks: Vec<_> = data.chunks(self.block_size).collect();
        let mut i = 0;

        while i < blocks.len() {
            let block = blocks[i];

            // Count repeated blocks
            let repeat_count = blocks[i..]
                .iter()
                .take_while(|&x| x == &block)
                .count()
                .min(127);

            if repeat_count > 1 {
                // Run-length mode
                result.push(repeat_count as u8);
                result.extend_from_slice(block);
                i += repeat_count;
            } else {
                // Direct copy mode
                let literal_end = blocks[i..]
                    .iter()
                    .take(127)
                    .take_while(|&&x| {
                        let next_pos = i + 1;
                        next_pos >= blocks.len() || x != blocks[next_pos]
                    })
                    .count();

                result.push(0x80 | (literal_end as u8));
                blocks[i..i + literal_end]
                    .iter()
                    .for_each(|block| result.extend_from_slice(block));
                i += literal_end;
            }
        }

        Ok(result)
    }

    pub fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let ctrl = *data.get(i).ok_or(RleError::InvalidInput)?;
            i += 1;

            let block = data
                .get(i..i + self.block_size)
                .ok_or(RleError::InvalidInput)?;

            if ctrl & 0x80 == 0 {
                // Run-length mode
                result.extend((0..ctrl).flat_map(|_| block.iter().copied()));
            } else {
                // Direct copy mode
                let count = (ctrl & 0x7f) as usize;
                let bytes = count * self.block_size;
                let slice = data.get(i..i + bytes).ok_or(RleError::InvalidInput)?;
                result.extend_from_slice(slice);
                i += bytes - self.block_size;
            }
            i += self.block_size;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_functionality() -> Result<()> {
        let cases = [
            vec![1, 1, 1, 2, 2, 3],
            vec![1, 2, 1, 2, 3, 4],
            vec![1, 2, 3, 4, 5, 6],
        ];

        for data in cases {
            let coder = RleCoder::new(2)?;
            let encoded = coder.encode(&data)?;
            let decoded = coder.decode(&encoded)?;
            assert_eq!(data, decoded);
        }
        Ok(())
    }

    #[test]
    fn test_invalid_cases() {
        // Invalid block size
        assert!(RleCoder::new(0).is_err());

        let coder = RleCoder::new(2).unwrap();

        // Unaligned input
        assert!(coder.encode(&[1, 2, 3]).is_err());

        // Invalid encoded data
        assert!(coder.decode(&[1]).is_err());
    }
}
