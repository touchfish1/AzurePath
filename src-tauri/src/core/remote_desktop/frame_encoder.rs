//! FrameEncoder — encodes raw framebuffer pixels into JPEG tiles.
//!
//! - First frame: returns the entire framebuffer as a single JPEG-encoded DesktopFrame.
//! - Subsequent frames: splits into 64x64 tiles, compares pixel data against the
//!   previous frame, and only encodes changed tiles as JPEG.

use crate::types::remote_desktop::frame::DesktopFrame;

/// 4 bytes per pixel: R, G, B, padding
const BYTES_PER_PIXEL: usize = 4;

pub struct FrameEncoder {
    width: u16,
    height: u16,
    quality: u8,
    /// Previous frame pixel data for change detection (4 bpp)
    previous: Vec<u8>,
    /// Whether this is the very first frame (emit everything)
    first_frame: bool,
}

impl FrameEncoder {
    pub fn new(width: u16, height: u16, quality: u8) -> Self {
        let total = width as usize * height as usize * BYTES_PER_PIXEL;
        Self {
            width,
            height,
            quality,
            previous: vec![0u8; total],
            first_frame: true,
        }
    }

    /// Encode a raw framebuffer (4 bytes per pixel: R,G,B,pad) into JPEG tiles.
    /// Returns the list of changed tiles as DesktopFrames.
    /// The caller is responsible for setting `session_id` on each returned frame.
    pub fn encode_frame(&mut self, raw_4bpp: &[u8]) -> Vec<DesktopFrame> {
        let tile_size: u32 = 64;
        let w = self.width as u32;
        let h = self.height as u32;
        let tiles_x = (w + tile_size - 1) / tile_size;
        let tiles_y = (h + tile_size - 1) / tile_size;
        let mut frames = Vec::new();

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let tile_w = (tile_size).min(w - tx * tile_size);
                let tile_h = (tile_size).min(h - ty * tile_size);

                // Extract tile pixel data (4 bpp)
                let mut tile_4bpp = Vec::with_capacity((tile_w * tile_h) as usize * BYTES_PER_PIXEL);
                for row in 0..tile_h {
                    for col in 0..tile_w {
                        let px = (tx * tile_size + col) as usize;
                        let py = (ty * tile_size + row) as usize;
                        let src_idx = (py * w as usize + px) * BYTES_PER_PIXEL;
                        tile_4bpp.extend_from_slice(&raw_4bpp[src_idx..src_idx + BYTES_PER_PIXEL]);
                    }
                }

                let changed = self.first_frame
                    || tile_changed(&self.previous, &tile_4bpp, tx * tile_size, ty * tile_size, tile_w, tile_h, w);

                if changed {
                    // Convert 4bpp → 3bpp RGB for JPEG encoding
                    let rgb: Vec<u8> = tile_4bpp
                        .chunks(BYTES_PER_PIXEL)
                        .flat_map(|pix| vec![pix[0], pix[1], pix[2]])
                        .collect();

                    let jpeg_data = encode_jpeg(&rgb, tile_w, tile_h, self.quality);

                    frames.push(DesktopFrame {
                        session_id: String::new(), // filled by caller
                        x: (tx * tile_size) as u16,
                        y: (ty * tile_size) as u16,
                        width: tile_w as u16,
                        height: tile_h as u16,
                        data: jpeg_data,
                        encoding: "jpeg".to_string(),
                    });

                    // Update previous tile
                    update_previous(
                        &mut self.previous,
                        &tile_4bpp,
                        tx * tile_size,
                        ty * tile_size,
                        tile_w,
                        tile_h,
                        w,
                    );
                }
            }
        }

        self.first_frame = false;
        frames
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        let total = width as usize * height as usize * BYTES_PER_PIXEL;
        self.width = width;
        self.height = height;
        self.previous = vec![0u8; total];
        self.first_frame = true;
    }
}

/// Check whether a tile has changed compared to the stored previous frame.
fn tile_changed(
    previous: &[u8],
    tile_data: &[u8],
    px: u32,
    py: u32,
    tile_w: u32,
    tile_h: u32,
    fb_width: u32,
) -> bool {
    for row in 0..tile_h {
        for col in 0..tile_w {
            let prev_idx = ((py + row) * fb_width + (px + col)) as usize * BYTES_PER_PIXEL;
            let tile_idx = (row * tile_w + col) as usize * BYTES_PER_PIXEL;
            if previous[prev_idx..prev_idx + BYTES_PER_PIXEL] != tile_data[tile_idx..tile_idx + BYTES_PER_PIXEL] {
                return true;
            }
        }
    }
    false
}

/// Copy tile pixel data into the previous-frame buffer.
fn update_previous(
    previous: &mut [u8],
    tile_data: &[u8],
    px: u32,
    py: u32,
    tile_w: u32,
    tile_h: u32,
    fb_width: u32,
) {
    for row in 0..tile_h {
        for col in 0..tile_w {
            let prev_idx = ((py + row) * fb_width + (px + col)) as usize * BYTES_PER_PIXEL;
            let tile_idx = (row * tile_w + col) as usize * BYTES_PER_PIXEL;
            previous[prev_idx..prev_idx + BYTES_PER_PIXEL]
                .copy_from_slice(&tile_data[tile_idx..tile_idx + BYTES_PER_PIXEL]);
        }
    }
}

/// Encode raw RGB888 pixel data as JPEG.
fn encode_jpeg(data: &[u8], width: u32, height: u32, quality: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
        encoder
            .encode(data, width, height, image::ColorType::Rgb8.into())
            .ok();
    }));
    if result.is_err() {
        eprintln!("[azurepath] JPEG encode panicked for {width}x{height}");
    }
    buf
}
