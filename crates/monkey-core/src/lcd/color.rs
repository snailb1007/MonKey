//! RGB888/RGB565 conversion primitives used by the LCD pipeline.

/// Wire byte order for a packed RGB565 pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorFormat {
    /// Low byte first, the byte order used by the Monka LCD interface.
    #[default]
    LittleEndian,
    /// High byte first, useful for display-controller diagnostics.
    BigEndian,
}

impl ColorFormat {
    pub const fn little_endian(self) -> bool {
        matches!(self, Self::LittleEndian)
    }
}

/// Quantize an 8-bit RGB color to the RGB565 bit layout.
pub const fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3)
}

/// Serialize an RGB565 pixel in the requested wire byte order.
pub const fn rgb565_to_bytes(pixel: u16, little_endian: bool) -> [u8; 2] {
    if little_endian {
        pixel.to_le_bytes()
    } else {
        pixel.to_be_bytes()
    }
}

/// Serialize an RGB565 pixel using the typed color format.
pub const fn rgb565_to_format_bytes(pixel: u16, format: ColorFormat) -> [u8; 2] {
    rgb565_to_bytes(pixel, format.little_endian())
}

/// Backwards-compatible descriptive alias for the boolean API.
pub const fn rgb565_bytes(pixel: u16, little_endian: bool) -> [u8; 2] {
    rgb565_to_bytes(pixel, little_endian)
}

pub const RED: u16 = 0xF800;
pub const GREEN: u16 = 0x07E0;
pub const BLUE: u16 = 0x001F;
pub const WHITE: u16 = 0xFFFF;
pub const BLACK: u16 = 0x0000;
