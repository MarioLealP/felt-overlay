use std::path::Path;

pub const DEFAULT_SPRITE_PNG: &[u8] = include_bytes!("../assets/placeholder.png");

#[derive(Debug, Clone)]
pub struct Sprite {
    pub width: u32,
    pub height: u32,
    /// Premultiplied ARGB8888 in little-endian order (B, G, R, A)
    pub data: Vec<u8>,
}

impl Sprite {
    pub fn load_default() -> Result<Self, String> {
        Self::load_from_memory(DEFAULT_SPRITE_PNG)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read image file: {}", e))?;
        Self::load_from_memory(&bytes)
    }

    pub fn load_from_memory(bytes: &[u8]) -> Result<Self, String> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| format!("Failed to decode PNG image: {}", e))?
            .to_rgba8();

        let (width, height) = img.dimensions();
        let mut data = Vec::with_capacity((width * height * 4) as usize);

        for pixel in img.pixels() {
            let [r, g, b, a] = pixel.0;
            // Premultiply alpha for Wayland ARGB8888
            let alpha = a as u32;
            let pr = ((r as u32 * alpha + 127) / 255) as u8;
            let pg = ((g as u32 * alpha + 127) / 255) as u8;
            let pb = ((b as u32 * alpha + 127) / 255) as u8;

            // Little-endian byte order for wl_shm ARGB8888 is [B, G, R, A]
            data.push(pb);
            data.push(pg);
            data.push(pr);
            data.push(a);
        }

        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Blit sprite to an ARGB8888 canvas of given dimensions at (dest_x, dest_y).
    pub fn blit_to_canvas(
        &self,
        canvas: &mut [u8],
        canvas_width: u32,
        canvas_height: u32,
        dest_x: i32,
        dest_y: i32,
    ) {
        let sw = self.width as i32;
        let sh = self.height as i32;
        let cw = canvas_width as i32;
        let ch = canvas_height as i32;

        for sy in 0..sh {
            let dy = dest_y + sy;
            if dy < 0 || dy >= ch {
                continue;
            }

            let start_sx = if dest_x < 0 { -dest_x } else { 0 };
            let end_sx = (cw - dest_x).clamp(0, sw);
            if start_sx >= end_sx {
                continue;
            }

            let dx = dest_x + start_sx;
            let copy_pixels = (end_sx - start_sx) as usize;
            let copy_bytes = copy_pixels * 4;

            let src_offset = ((sy * sw + start_sx) * 4) as usize;
            let dst_offset = ((dy * cw + dx) * 4) as usize;

            canvas[dst_offset..dst_offset + copy_bytes]
                .copy_from_slice(&self.data[src_offset..src_offset + copy_bytes]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_sprite() {
        let sprite = Sprite::load_default().expect("load default sprite");
        assert_eq!(sprite.width, 200);
        assert_eq!(sprite.height, 200);
        assert_eq!(sprite.data.len(), 200 * 200 * 4);
    }

    #[test]
    fn test_blit_clipping() {
        let sprite = Sprite::load_default().expect("load sprite");
        let cw = 100u32;
        let ch = 100u32;
        let mut canvas = vec![0u8; (cw * ch * 4) as usize];

        // Blit completely offscreen top
        sprite.blit_to_canvas(&mut canvas, cw, ch, 0, -250);
        assert!(canvas.iter().all(|&b| b == 0));

        // Blit completely offscreen bottom
        sprite.blit_to_canvas(&mut canvas, cw, ch, 0, 150);
        assert!(canvas.iter().all(|&b| b == 0));

        // Blit partially overlapping
        sprite.blit_to_canvas(&mut canvas, cw, ch, 0, 0);
        assert!(canvas.iter().any(|&b| b != 0));
    }
}
