use std::num::TryFromIntError;
use std::time::Duration;

use ani::{Ani, Image};
use ani2xcur_core::Size;
use image::RgbaImage;
use image::imageops::{self, FilterType};
use xcur::Xcursor;

/// Request to convert a cursor from ANI to Xcursor.
pub struct ConvertCursorRequest {
    pub ani: Ani,
    pub sizes: Vec<Size>,
}

/// Errors that can occur while converting a cursor from ANI to Xcursor.
#[derive(Debug, thiserror::Error)]
pub enum ConvertCursorError {
    #[error("cursor width/height too large")]
    SizeTooLarge(#[source] TryFromIntError),

    #[error("invalid Xcursor image")]
    InvalidXcursorImage(#[source] xcur::ParseError),

    // TODO: This is an implementation detail; error needs to be caught earlier.
    #[error("frame not found at index {index}")]
    InvalidFrameIndex { index: usize },
}

struct AnimationMetadata {
    delays: Vec<Duration>,
    sequence: Vec<usize>,
}

impl AnimationMetadata {
    fn from_ani(ani: &Ani) -> Self {
        let delays = ani
            .rates_or_default()
            .into_iter()
            .map(|r| Duration::from_millis(u64::from(r) * 1000 / 60))
            .collect();

        let sequence = ani
            .sequence_or_default()
            .into_iter()
            .map(|s| usize::try_from(s).expect("u32 overflowed usize"))
            .collect();

        Self { delays, sequence }
    }
}

#[derive(Debug, Clone)]
struct CursorSource {
    rgba: RgbaImage,
    width: u32,
    height: u32,
    hotspot_x: u16,
    hotspot_y: u16,
}

impl CursorSource {
    fn from_ani_frame(image: &Image) -> Self {
        let width = image.width();
        let height = image.height();
        let (hotspot_x, hotspot_y) = image.cursor_hotspot().unwrap_or((0, 0));
        let rgba = RgbaImage::from_raw(image.width(), image.height(), image.rgba_data().to_vec())
            .expect("width/height derived from image buffer so container should always fit");

        CursorSource {
            rgba,
            width,
            height,
            hotspot_x,
            hotspot_y,
        }
    }

    fn into_xcursor_image(
        self,
        size: Size,
        delay: Duration,
    ) -> Result<xcur::Image, ConvertCursorError> {
        let target_size = u16::from(size);

        let (hotspot_x, hotspot_y) = self.hotspot_at(size);
        let hotspot_x = hotspot_x
            .try_into()
            .map_err(ConvertCursorError::SizeTooLarge)?;
        let hotspot_y = hotspot_y
            .try_into()
            .map_err(ConvertCursorError::SizeTooLarge)?;

        let rgba = if u32::from(size.into_inner()) == self.nominal() {
            self.rgba
        } else {
            self.resize(size)
        };

        xcur::Image::new(
            target_size,
            target_size,
            hotspot_x,
            hotspot_y,
            delay,
            rgba_to_argb(rgba.as_raw()).collect(),
        )
        .map_err(ConvertCursorError::InvalidXcursorImage)
    }

    fn nominal(&self) -> u32 {
        u32::max(self.width, self.height)
    }

    fn hotspot_at(&self, target: Size) -> (u32, u32) {
        let source_size = self.width.max(self.height);
        let target_size = u32::from(target);

        let hotspot_x = u32::from(self.hotspot_x) * target_size / source_size;
        let hotspot_y = u32::from(self.hotspot_y) * target_size / source_size;

        (hotspot_x, hotspot_y)
    }

    fn resize(&self, target: Size) -> RgbaImage {
        let target_size = u32::from(target);
        imageops::resize(&self.rgba, target_size, target_size, FilterType::Lanczos3)
    }
}

/// Converts a cursor from ANI to Xcursor format.
///
/// # Errors
///
/// Returns an error if:
pub fn xcursor_from_ani(request: ConvertCursorRequest) -> Result<Xcursor, ConvertCursorError> {
    let ani = request.ani;
    let metadata = AnimationMetadata::from_ani(&ani);

    // Contains one of each of the available animation frames.
    let mut frames = Vec::<xcur::Image>::new();

    for (frame, delay) in ani.frames().iter().zip(metadata.delays) {
        let sizes = request.sizes.clone();

        let sources = frame
            .iter()
            .map(CursorSource::from_ani_frame)
            .collect::<Vec<_>>();

        let largest = sources
            .into_iter()
            .max_by_key(|source| source.width.max(source.height));

        if let Some(source) = largest {
            for target in sizes {
                let xcursor = source.clone().into_xcursor_image(target, delay)?;
                frames.push(xcursor);
            }
        }
    }

    frames.sort_by_key(|image| u16::max(image.width(), image.height()));
    debug_assert!(frames.len().is_multiple_of(request.sizes.len()));

    // Contains the actual frames needed for the animation.
    let capacity = metadata.sequence.len() * request.sizes.len();
    let mut images = Vec::<xcur::Image>::with_capacity(capacity);

    let frames_per_size = frames.len() / request.sizes.len();

    for size_idx in 0..request.sizes.len() {
        let offset = size_idx * frames_per_size;

        for sequence_idx in &metadata.sequence {
            let frame_idx = sequence_idx + offset;

            let frame = frames
                .get(frame_idx)
                .cloned()
                .ok_or(ConvertCursorError::InvalidFrameIndex { index: frame_idx })?;

            images.push(frame);
        }
    }

    let comments = vec![]; // Comments are ignored.

    Ok(Xcursor::new(images, comments))
}

/// Shifts the bytes from RGBA to ARGB format.
fn rgba_to_argb(bytes: &[u8]) -> impl Iterator<Item = u32> {
    let (chunks, remainder) = bytes.as_chunks::<4>();
    debug_assert!(remainder.is_empty());

    chunks.iter().map(|&[r, g, b, a]| {
        (u32::from(a) << 24) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
}
