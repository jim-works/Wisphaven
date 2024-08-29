use bevy::{prelude::*, render::{render_resource::*, texture::*}};

//sourced from https://github.com/bevyengine/bevy/pull/10392 until bevy gets proper image sampling
#[derive(Debug)]
pub enum TextureAccessError {
    OutOfBounds { x: u32, y: u32, z: u32 },
    UnsupportedTextureFormat(TextureFormat),
    WrongDimension,
}

pub trait ImageExtension {
    fn get_color_at(&self, x: u32, y: u32) -> Result<Color, TextureAccessError>;
    fn get_color_at_3d(&self, x: u32, y: u32, z: u32) -> Result<Color, TextureAccessError>;
    fn pixel_bytes(&self, coords: UVec3) -> Option<&[u8]>;
    fn pixel_data_offset(&self, coords: UVec3) -> Option<usize>;
    fn get_color_at_internal(&self, coords: UVec3) -> Result<Color, TextureAccessError>;
}

impl ImageExtension for Image {
    fn get_color_at(&self, x: u32, y: u32) -> Result<Color, TextureAccessError> {
        if self.texture_descriptor.dimension != TextureDimension::D2 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.get_color_at_internal(UVec3::new(x, y, 0))
    }

    fn get_color_at_3d(&self, x: u32, y: u32, z: u32) -> Result<Color, TextureAccessError> {
        if self.texture_descriptor.dimension != TextureDimension::D3 {
            return Err(TextureAccessError::WrongDimension);
        }
        self.get_color_at_internal(UVec3::new(x, y, z))
    }

    fn pixel_bytes(&self, coords: UVec3) -> Option<&[u8]> {
        let len = self.texture_descriptor.format.pixel_size();
        self.pixel_data_offset(coords)
            .map(|start| &self.data[start..(start + len)])
    }

    fn pixel_data_offset(&self, coords: UVec3) -> Option<usize> {
        let width = self.texture_descriptor.size.width;
        let height = self.texture_descriptor.size.height;
        let depth = self.texture_descriptor.size.depth_or_array_layers;

        let pixel_size = self.texture_descriptor.format.pixel_size();
        let pixel_offset = match self.texture_descriptor.dimension {
            TextureDimension::D3 => {
                if coords.x > width || coords.y > height || coords.z > depth {
                    return None;
                }
                coords.z * height * width + coords.y * width + coords.x
            }
            TextureDimension::D2 => {
                if coords.x > width || coords.y > height {
                    return None;
                }
                coords.y * width + coords.x
            }
            TextureDimension::D1 => {
                if coords.x > width {
                    return None;
                }
                coords.x
            }
        };

        Some(pixel_offset as usize * pixel_size)
    }

    fn get_color_at_internal(&self, coords: UVec3) -> Result<Color, TextureAccessError> {
        let Some(bytes) = self.pixel_bytes(coords) else {
            return Err(TextureAccessError::OutOfBounds {
                x: coords.x,
                y: coords.y,
                z: coords.z,
            });
        };

        match self.texture_descriptor.format {
            TextureFormat::Rgba8UnormSrgb => Ok(Color::rgba(
                bytes[0] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[2] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8Uint => Ok(Color::rgba_linear(
                bytes[0] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[2] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Bgra8UnormSrgb => Ok(Color::rgba(
                bytes[2] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[0] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Bgra8Unorm => Ok(Color::rgba_linear(
                bytes[2] as f32 / u8::MAX as f32,
                bytes[1] as f32 / u8::MAX as f32,
                bytes[0] as f32 / u8::MAX as f32,
                bytes[3] as f32 / u8::MAX as f32,
            )),
            TextureFormat::Rgba32Float => Ok(Color::rgba_linear(
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            )),
            TextureFormat::Rgba16Unorm | TextureFormat::Rgba16Uint => {
                let (r, g, b, a) = (
                    u16::from_le_bytes([bytes[0], bytes[1]]),
                    u16::from_le_bytes([bytes[2], bytes[3]]),
                    u16::from_le_bytes([bytes[4], bytes[5]]),
                    u16::from_le_bytes([bytes[6], bytes[7]]),
                );
                Ok(Color::rgba_linear(
                    // going via f64 to avoid rounding errors with large numbers and division
                    (r as f64 / u16::MAX as f64) as f32,
                    (g as f64 / u16::MAX as f64) as f32,
                    (b as f64 / u16::MAX as f64) as f32,
                    (a as f64 / u16::MAX as f64) as f32,
                ))
            }
            TextureFormat::Rgba32Uint => {
                let (r, g, b, a) = (
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                    u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                    u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                    u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
                );
                Ok(Color::rgba_linear(
                    // going via f64 to avoid rounding errors with large numbers and division
                    (r as f64 / u32::MAX as f64) as f32,
                    (g as f64 / u32::MAX as f64) as f32,
                    (b as f64 / u32::MAX as f64) as f32,
                    (a as f64 / u32::MAX as f64) as f32,
                ))
            }
            // assume R-only texture format means grayscale (linear)
            // copy value to all of RGB in Color
            TextureFormat::R8Unorm | TextureFormat::R8Uint => {
                let x = bytes[0] as f32 / u8::MAX as f32;
                Ok(Color::rgba_linear(x, x, x, 1.0))
            }
            TextureFormat::R16Unorm | TextureFormat::R16Uint => {
                let x = u16::from_le_bytes([bytes[0], bytes[1]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let x = (x as f64 / u16::MAX as f64) as f32;
                Ok(Color::rgba_linear(x, x, x, 1.0))
            }
            TextureFormat::R32Uint => {
                let x = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let x = (x as f64 / u32::MAX as f64) as f32;
                Ok(Color::rgba_linear(x, x, x, 1.0))
            }
            TextureFormat::R32Float => {
                let x = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok(Color::rgba_linear(x, x, x, 1.0))
            }
            TextureFormat::Rg8Unorm | TextureFormat::Rg8Uint => {
                let r = bytes[0] as f32 / u8::MAX as f32;
                let g = bytes[1] as f32 / u8::MAX as f32;
                Ok(Color::rgba_linear(r, g, 0.0, 1.0))
            }
            TextureFormat::Rg16Unorm | TextureFormat::Rg16Uint => {
                let r = u16::from_le_bytes([bytes[0], bytes[1]]);
                let g = u16::from_le_bytes([bytes[2], bytes[3]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let r = (r as f64 / u16::MAX as f64) as f32;
                let g = (g as f64 / u16::MAX as f64) as f32;
                Ok(Color::rgba_linear(r, g, 0.0, 1.0))
            }
            TextureFormat::Rg32Uint => {
                let r = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let g = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                // going via f64 to avoid rounding errors with large numbers and division
                let r = (r as f64 / u32::MAX as f64) as f32;
                let g = (g as f64 / u32::MAX as f64) as f32;
                Ok(Color::rgba_linear(r, g, 0.0, 1.0))
            }
            TextureFormat::Rg32Float => {
                let r = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let g = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                Ok(Color::rgba_linear(r, g, 0.0, 1.0))
            }
            _ => Err(TextureAccessError::UnsupportedTextureFormat(
                self.texture_descriptor.format,
            )),
        }
    }
}
