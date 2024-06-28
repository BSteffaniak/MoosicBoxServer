use bytes::Bytes;
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType};
use thiserror::Error;

use crate::Encoding;

pub fn try_resize_local_file(
    width: u32,
    height: u32,
    path: &str,
    encoding: Encoding,
    quality: u8,
) -> Result<Option<Bytes>, image::error::ImageError> {
    let img = image::open(path)?;
    let resized = img.resize(width, height, FilterType::Lanczos3);
    match encoding {
        Encoding::Jpeg => {
            let mut buffer = Vec::new();
            let mut encoder = JpegEncoder::new_with_quality(&mut buffer, quality);
            encoder.encode_image(&resized)?;
            Ok(Some(buffer.into()))
        }
        Encoding::Webp => {
            if let Ok(encoder) = webp::Encoder::from_image(&resized) {
                let memory = encoder.encode(quality.into());
                let bytes = memory.to_vec();
                Ok(Some(bytes.into()))
            } else {
                Ok(None)
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ResizeImageError {
    #[error(transparent)]
    Image(#[from] image::error::ImageError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}

pub async fn try_resize_local_file_async(
    width: u32,
    height: u32,
    path: &str,
    encoding: Encoding,
    quality: u8,
) -> Result<Option<Bytes>, ResizeImageError> {
    let path = path.to_owned();
    Ok(tokio::task::spawn_blocking(move || {
        try_resize_local_file(width, height, &path, encoding, quality)
    })
    .await??)
}
