#[derive(Debug)]
pub struct BitMap {
    pub height: usize,
    pub width: usize,
    pub pixels: Vec<u32>,
}

impl BitMap {
    pub fn new(pixels: Vec<u32>, width: usize, height: usize) -> Self {
        BitMap {
            height,
            width,
            pixels,
        }
    }
}

impl Default for BitMap {
    fn default() -> Self {
        BitMap {
            height: 0,
            width: 0,
            pixels: Vec::new(),
        }
    }
}
