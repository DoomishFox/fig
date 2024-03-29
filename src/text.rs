pub struct TextLine {
    buffer: Buffer,
    metadata: Metadata,
}

impl TextLine {
    pub fn empty() -> Self {
        Self {
            buffer: Buffer::from(""),
            metadata: Metadata::empty(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Metadata {
    pub pixel_size: [u32; 2],
    pub pixel_position: [u32; 2],
    length: u32,
    pub kerning: u32,
}

impl Metadata {
    pub fn empty() -> Self {
        Self {
            pixel_size: [0; 2],
            pixel_position: [0; 2],
            length: 0,
            kerning: 0,
        }
    }
    pub fn from(buffer: &Buffer) -> Self {
        Self {
            pixel_size: [0; 2],
            pixel_position: [0; 2],
            length: buffer.chars.len() as u32,
            kerning: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Glyph {
    atlas_offset: [u32; 4],
    color: [f32; 4],
}

impl Glyph {
    pub fn from(c: char) -> Self {
        // this SUCKS but im *just* intoxicated enough to
        // think its permissable
        let offset = match c {
            '0' => [0, 0],
            '1' => [1, 0],
            '2' => [2, 0],
            '3' => [3, 0],
            '4' => [4, 0],
            '5' => [5, 0],
            '6' => [6, 0],
            '7' => [7, 0],
            '8' => [8, 0],
            '9' => [9, 0],

            'A' => [0, 1],
            'B' => [1, 1],
            'C' => [2, 1],
            'D' => [3, 1],
            'E' => [4, 1],
            'F' => [5, 1],
            'G' => [6, 1],
            'H' => [7, 1],
            'I' => [8, 1],
            'J' => [9, 1],
            'K' => [10, 1],
            'L' => [11, 1],
            'M' => [12, 1],
            'N' => [13, 1],
            'O' => [14, 1],
            'P' => [15, 1],
            'Q' => [16, 1],
            'R' => [17, 1],
            'S' => [18, 1],
            'T' => [19, 1],
            'U' => [20, 1],
            'V' => [21, 1],
            'W' => [22, 1],
            'X' => [23, 1],
            'Y' => [24, 1],
            'Z' => [25, 1],

            'a' => [0, 2],
            'b' => [1, 2],
            'c' => [2, 2],
            'd' => [3, 2],
            'e' => [4, 2],
            'f' => [5, 2],
            'g' => [6, 2],
            'h' => [7, 2],
            'i' => [8, 2],
            'j' => [9, 2],
            'k' => [10, 2],
            'l' => [11, 2],
            'm' => [12, 2],
            'n' => [13, 2],
            'o' => [14, 2],
            'p' => [15, 2],
            'q' => [16, 2],
            'r' => [17, 2],
            's' => [18, 2],
            't' => [19, 2],
            'u' => [20, 2],
            'v' => [21, 2],
            'w' => [22, 2],
            'x' => [23, 2],
            'y' => [24, 2],
            'z' => [25, 2],

            _ => [0,0],
        };
        Self {
            atlas_offset: [offset[0], offset[1], 0, 0],
            color: match c {
                ' ' => [0.0, 0.0, 0.0, 0.0],
                _ => [1.0, 0.0, 0.0, 0.0],
            },
        }
    }
}

pub struct Buffer {
    chars: String,
    glyphs: Vec<Glyph>,
}

impl Buffer {
    pub fn from(text: &str) -> Self {
        let glyphs: Vec<Glyph> = text
            .chars()
            .into_iter()
            .map(|c| Glyph::from(c))
            .collect();
        Self {
            chars: String::from(text),
            glyphs: glyphs,
        }
    }
    pub fn pack_glyphs(&self) -> &[u8] {
        bytemuck::cast_slice(self.glyphs.as_slice())
    }
    pub fn len(&self) -> usize {
        self.chars.len()
    }
}
