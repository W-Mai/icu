use crate::endecoder::utils::diff::ImageDiffResult;
use image::{Rgba, RgbaImage};

pub trait ImageOverlay {
    fn pixel_at(&self, x: u32, y: u32, base: &RgbaImage) -> Option<Rgba<u8>>;

    fn is_fullscreen(&self) -> bool {
        false
    }

    fn rebuild(&mut self, _base: &RgbaImage) {}
}

pub struct OverlayStack {
    pub base: RgbaImage,
    pub overlays: Vec<Box<dyn ImageOverlay>>,
    cached: Option<RgbaImage>,
    dirty: bool,
}

impl OverlayStack {
    pub fn new(base: RgbaImage) -> Self {
        let dirty = true;
        Self {
            base,
            overlays: Vec::new(),
            cached: None,
            dirty,
        }
    }

    pub fn push(&mut self, overlay: Box<dyn ImageOverlay>) {
        self.overlays.push(overlay);
        self.dirty = true;
    }

    pub fn pop(&mut self) {
        self.overlays.pop();
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.overlays.clear();
        self.dirty = true;
    }

    pub fn set_base(&mut self, base: RgbaImage) {
        self.base = base;
        self.dirty = true;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn composite(&mut self) -> &RgbaImage {
        if self.dirty {
            let mut result = self.base.clone();
            for overlay in &self.overlays {
                let w = result.width();
                let h = result.height();
                for y in 0..h {
                    for x in 0..w {
                        if let Some(color) = overlay.pixel_at(x, y, &result) {
                            blend_pixel(result.get_pixel_mut(x, y), &color);
                        }
                    }
                }
            }
            self.cached = Some(result);
            self.dirty = false;
        }
        self.cached.as_ref().unwrap()
    }
}

fn blend_pixel(dst: &mut Rgba<u8>, src: &Rgba<u8>) {
    let sa = src.0[3] as u32;
    if sa == 255 {
        *dst = *src;
        return;
    }
    if sa == 0 {
        return;
    }
    let da = dst.0[3] as u32;
    let out_a = sa + da * (255 - sa) / 255;
    if out_a == 0 {
        *dst = Rgba([0, 0, 0, 0]);
        return;
    }
    for i in 0..3 {
        dst.0[i] = ((src.0[i] as u32 * sa + dst.0[i] as u32 * da * (255 - sa) / 255) / out_a) as u8;
    }
    dst.0[3] = out_a as u8;
}

pub struct IndexHoverOverlay {
    pub palette_index: u8,
    pub indexes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub highlight: Rgba<u8>,
    pub dim: Rgba<u8>,
}

impl IndexHoverOverlay {
    pub fn new(indexed: &crate::midata::IndexedImageData, palette_index: u8) -> Self {
        Self {
            palette_index,
            indexes: indexed.indexes.clone(),
            width: indexed.width,
            height: indexed.height,
            highlight: Rgba([255, 255, 0, 200]),
            dim: Rgba([0, 0, 0, 160]),
        }
    }
}

impl ImageOverlay for IndexHoverOverlay {
    fn pixel_at(&self, x: u32, y: u32, _base: &RgbaImage) -> Option<Rgba<u8>> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y * self.width + x) as usize;
        if idx >= self.indexes.len() {
            return None;
        }
        if self.indexes[idx] == self.palette_index {
            Some(self.highlight)
        } else {
            Some(self.dim)
        }
    }

    fn is_fullscreen(&self) -> bool {
        true
    }
}

pub struct QualityOverlay {
    pub original: RgbaImage,
    pub indexes: Vec<u8>,
    pub palette: Vec<[u8; 4]>,
    pub width: u32,
    pub height: u32,
}

impl QualityOverlay {
    pub fn new(indexed: &crate::midata::IndexedImageData, original: RgbaImage) -> Self {
        Self {
            original,
            indexes: indexed.indexes.clone(),
            palette: indexed.palette.clone(),
            width: indexed.width,
            height: indexed.height,
        }
    }
}

impl ImageOverlay for QualityOverlay {
    fn pixel_at(&self, x: u32, y: u32, _base: &RgbaImage) -> Option<Rgba<u8>> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y * self.width + x) as usize;
        if idx >= self.indexes.len() {
            return None;
        }
        let pal_idx = self.indexes[idx] as usize;
        if pal_idx >= self.palette.len() {
            return None;
        }
        let orig = self.original.get_pixel(x, y);
        let pal = self.palette[pal_idx];
        let diff = [
            orig.0[0].abs_diff(pal[0]),
            orig.0[1].abs_diff(pal[1]),
            orig.0[2].abs_diff(pal[2]),
        ];
        let max_diff = diff.iter().copied().max().unwrap_or(0);
        let intensity = (max_diff as u32 * 4).min(255) as u8;
        Some(Rgba([intensity, intensity / 4, 0, 255]))
    }

    fn is_fullscreen(&self) -> bool {
        true
    }
}

pub struct DiffOverlay {
    pub diff_result: ImageDiffResult,
    pub tolerance: f32,
    pub blend: f32,
    pub color: Rgba<u8>,
}

impl DiffOverlay {
    pub fn new(diff_result: ImageDiffResult, tolerance: f32, blend: f32) -> Self {
        Self {
            diff_result,
            tolerance,
            blend,
            color: Rgba([255, 0, 0, 255]),
        }
    }
}

impl ImageOverlay for DiffOverlay {
    fn pixel_at(&self, x: u32, y: u32, _base: &RgbaImage) -> Option<Rgba<u8>> {
        let (w, h) = self.diff_result.size();
        if x >= w || y >= h {
            return None;
        }
        let idx = (y * w + x) as usize;
        let diffs = self.diff_result.diffs();
        if idx >= diffs.len() {
            return None;
        }
        let diff_pixel = &diffs[idx];
        let max_diff = diff_pixel
            .diff
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0_f32, f32::max);
        if max_diff < self.tolerance {
            return None;
        }
        let t = (self.blend - 0.5).abs() / 0.5;
        let alpha = ((1.0 - t) * 255.0) as u8;
        if alpha == 0 {
            return None;
        }
        Some(Rgba([
            self.color.0[0],
            self.color.0[1],
            self.color.0[2],
            alpha,
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(c))
    }

    #[test]
    fn empty_stack_returns_base() {
        let base = rgba(2, 2, [10, 20, 30, 255]);
        let mut stack = OverlayStack::new(base.clone());
        let result = stack.composite();
        assert_eq!(result.dimensions(), base.dimensions());
        assert_eq!(result.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));
    }

    #[test]
    fn fullscreen_overlay_replaces_all() {
        let base = rgba(2, 2, [10, 20, 30, 255]);
        let mut stack = OverlayStack::new(base);
        stack.push(Box::new(IndexHoverOverlay {
            palette_index: 0,
            indexes: vec![0, 0, 0, 0],
            width: 2,
            height: 2,
            highlight: Rgba([255, 255, 0, 255]),
            dim: Rgba([0, 0, 0, 255]),
        }));
        let result = stack.composite();
        assert_eq!(result.get_pixel(0, 0), &Rgba([255, 255, 0, 255]));
        assert_eq!(result.get_pixel(1, 1), &Rgba([255, 255, 0, 255]));
    }

    #[test]
    fn partial_overlay_blends() {
        let base = rgba(2, 1, [0, 0, 0, 255]);
        let mut stack = OverlayStack::new(base);
        stack.push(Box::new(IndexHoverOverlay {
            palette_index: 1,
            indexes: vec![0, 1],
            width: 2,
            height: 1,
            highlight: Rgba([255, 255, 0, 255]),
            dim: Rgba([0, 0, 0, 100]),
        }));
        let result = stack.composite();
        assert_eq!(result.get_pixel(0, 0), &Rgba([0, 0, 0, 255]));
        assert_eq!(result.get_pixel(1, 0), &Rgba([255, 255, 0, 255]));
    }

    #[test]
    fn cached_composite_is_reused() {
        let base = rgba(2, 2, [10, 20, 30, 255]);
        let mut stack = OverlayStack::new(base);
        let _ = stack.composite();
        let ptr1 = stack.composite().as_ptr();
        let ptr2 = stack.composite().as_ptr();
        assert_eq!(ptr1, ptr2, "second composite should reuse cache");
        stack.mark_dirty();
        let _ = stack.composite();
        let ptr3 = stack.composite().as_ptr();
        let ptr4 = stack.composite().as_ptr();
        assert_eq!(
            ptr3, ptr4,
            "after dirty re-composite, subsequent calls reuse"
        );
    }

    #[test]
    fn push_and_pop_mark_dirty() {
        let base = rgba(1, 1, [0, 0, 0, 255]);
        let mut stack = OverlayStack::new(base);
        let _ = stack.composite();
        assert!(!stack.dirty);
        stack.push(Box::new(IndexHoverOverlay {
            palette_index: 0,
            indexes: vec![0],
            width: 1,
            height: 1,
            highlight: Rgba([255, 0, 0, 255]),
            dim: Rgba([0, 0, 0, 255]),
        }));
        assert!(stack.dirty);
        let _ = stack.composite();
        assert!(!stack.dirty);
        stack.pop();
        assert!(stack.dirty);
    }

    #[test]
    fn quality_overlay_marks_error() {
        use crate::midata::IndexedImageData;
        let original = rgba(2, 1, [200, 100, 50, 255]);
        let indexed = IndexedImageData {
            rgba: rgba(2, 1, [100, 50, 25, 255]),
            palette: vec![[100, 50, 25, 255]],
            indexes: vec![0, 0],
            bpp: 1,
            width: 2,
            height: 1,
        };
        let overlay = QualityOverlay::new(&indexed, original);
        let c0 = overlay.pixel_at(0, 0, &indexed.rgba).unwrap();
        assert!(c0.0[0] > 0, "red channel should carry error magnitude");
        assert_eq!(c0.0[3], 255);
    }

    #[test]
    fn diff_overlay_skips_matching_pixels() {
        use crate::endecoder::utils::diff::diff_image;
        use crate::midata::MiData;
        let img1 = rgba(2, 1, [10, 20, 30, 255]);
        let img2 = rgba(2, 1, [11, 21, 31, 255]);
        let dr = diff_image(&MiData::RGBA(img1), &MiData::RGBA(img2))
            .expect("diff_image should produce a result");
        let overlay = DiffOverlay::new(dr, 5.0, 0.5);
        let base = rgba(2, 1, [0, 0, 0, 255]);
        assert!(overlay.pixel_at(0, 0, &base).is_none());
    }

    #[test]
    fn diff_overlay_marks_changed_pixels() {
        use crate::endecoder::utils::diff::diff_image as compute_diff;
        use crate::midata::MiData;
        let img1 = rgba(1, 1, [10, 20, 30, 255]);
        let img2 = rgba(1, 1, [200, 20, 30, 255]);
        let dr = compute_diff(&MiData::RGBA(img1), &MiData::RGBA(img2))
            .expect("diff_image should produce a result");
        let overlay = DiffOverlay::new(dr, 1.0, 0.5);
        let base = rgba(1, 1, [0, 0, 0, 255]);
        let p = overlay.pixel_at(0, 0, &base).unwrap();
        assert_eq!(p.0[3], 255);
        assert!(p.0[0] > 0, "blend=0.5 → t=0 → full RED");
    }
}
