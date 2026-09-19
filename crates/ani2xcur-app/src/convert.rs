use std::num::TryFromIntError;
use std::time::Duration;

use ani::{Ani, Image};
use ani2xcur_core::Size;
use image::RgbaImage;
use image::imageops::{self, FilterType};
use tracing::warn;
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
    // size: Size,
    hotspot_x: u16,
    hotspot_y: u16,
}

impl CursorSource {
    fn from_ani_frame(image: &Image) -> Option<Self> {
        let (hotspot_x, hotspot_y) = image.cursor_hotspot().unwrap_or((0, 0));
        let rgba = RgbaImage::from_raw(image.width(), image.height(), image.rgba_data().to_vec())
            // TODO: Remove when this no longer panics - Figure out the most useful way to error in
            // this case - The original PNG/BMP had an invalid width/height for the given image,
            // which should _not_ have passed ICO validation (external library in our case).
            //
            // ---8<---
            //
            // This would only panic if `ico::IconDirEntry::decode` (from our `ani` dependency)
            // gives us the wrong height/width for our image data, causing our width/height
            // to claim less data than it actually holds.
            .expect("width/height derived from image buffer so container should always fit");

        Some(CursorSource {
            rgba,
            width: image.width(),
            height: image.height(),
            hotspot_x,
            hotspot_y,
        })
    }

    fn into_xcursor_image(
        self,
        size: Size,
        delay: Duration,
    ) -> Result<xcur::Image, ConvertCursorError> {
        let target_size = u16::from(size);

        let (hotspot_x, hotspot_y) = self.hotspot_at(size);

        let rgba = if u32::from(size.into_inner()) == self.width.max(self.height) {
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

    fn hotspot_at(&self, target: Size) -> (u16, u16) {
        // TODO: This was done very quickly -- The idea is to skip checking if the image is the
        // correct size and instead resize it. I guess `Size` should really only be used when
        // validating user input. Returning a `Result` here seems odd, so I think the boundary
        // should be somewhere else. This is just a quick fix before I head out of the house.
        let source_size = u16::try_from(self.width.max(self.height))
            .expect("width/height didn't fit within a u16");
        let target_size = u16::from(target);

        let hotspot_x = self.hotspot_x * target_size / source_size;
        let hotspot_y = self.hotspot_y * target_size / source_size;

        (hotspot_x, hotspot_y)
    }

    fn resize(&self, target: Size) -> RgbaImage {
        let target_size = u32::from(target);
        imageops::resize(&self.rgba, target_size, target_size, FilterType::Lanczos3)
    }
}

/// Converts a cursor from ANI to Xcursor format.
pub fn xcursor_from_ani(request: ConvertCursorRequest) -> Result<Xcursor, ConvertCursorError> {
    let ani = request.ani;
    let metadata = AnimationMetadata::from_ani(&ani);

    let mut frames = Vec::<xcur::Image>::new();

    for (frame, delay) in ani.frames().iter().zip(metadata.delays) {
        let sizes = request.sizes.clone();

        let mut sources = Vec::new();

        for image in frame {
            let Some(source) = CursorSource::from_ani_frame(image) else {
                let nominal = u64::max(image.width().into(), image.height().into());
                warn!("skipping image with non-standard size: {nominal}");
                continue;
            };

            sources.push(source);
        }

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

    let frames_per_size = frames.len() / request.sizes.len();
    // `frames` contains _one_of_each_ of the available animation frames.
    // `images` contains the actual frames needed for the animation (e.g., if the animation
    // asks for (6) Frame 1's, `images` should contain (6) copies of Frame 1).
    let mut images =
        Vec::<xcur::Image>::with_capacity(metadata.sequence.len() * request.sizes.len());

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
