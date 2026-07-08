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
        assert_eq!(ptr3, ptr4, "after dirty re-composite, subsequent calls reuse");
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
}
