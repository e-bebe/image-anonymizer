pub mod ocr;
pub mod face;

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

use ocr::detection::detect_text_with_api;
use ocr::masking::mask_text;
use face::detection::detect_faces_with_api;
use face::masking::mask_faces;

/// Process an image to mask sensitive text and faces
///
/// # Arguments
///
/// * `input_path` - The path to the input image
/// * `output_dir` - The directory to save the output image
/// * `mask_texts` - The texts to mask
/// * `mask_faces` - Whether to mask faces in the image
///
/// # Returns
///
/// * `Result<()>` - The result of the image processing
///
/// # Errors
///
/// * `anyhow::Error` - If the image processing fails
///
pub fn process_image(
    input_path: &Path, 
    output_dir: &Path, 
    mask_texts: Option<&str>,
    mask_faces_flag: bool
) -> Result<()> {
    info!("Image processing started");
    info!("Reading input image: {:?}", input_path);
    let mut img = image::open(input_path).context("Failed to open input image")?;

    let file_name = input_path
        .file_name()
        .context("Invalid input filename")?
        .to_str()
        .context("Non-UTF8 filename")?;

    let output_path = output_dir.join(format!("masked_{}", file_name));

    // Create output directory if it doesn't exist
    if !output_dir.exists() {
        debug!("Creating output directory: {:?}", output_dir);
        fs::create_dir_all(output_dir).context("Failed to create output directory")?;
    }

    // Process text masking
    let additional_masks = if let Some(texts) = mask_texts {
        texts
            .split(',')
            .map(|text| text.trim().to_string())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let annotations = detect_text_with_api(input_path).context("Failed to detect text in image")?;

    if annotations.is_empty() {
        debug!("No text detected in the image");
    } else {
        debug!("Detected {} text annotations", annotations.len());
        mask_text(&mut img, &annotations, &additional_masks).context("Failed to mask text")?;
    }

    // Process face masking if enabled
    if mask_faces_flag {
        info!("Face detection enabled, detecting faces...");
        match detect_faces_with_api(input_path) {
            Ok(face_annotations) => {
                if face_annotations.is_empty() {
                    info!("No faces detected in the image");
                } else {
                    info!("Detected {} faces in the image", face_annotations.len());
                    mask_faces(&mut img, &face_annotations).context("Failed to mask faces")?;
                }
            },
            Err(e) => {
                debug!("Face detection failed: {}", e);
                info!("Skipping face masking due to detection error");
            }
        }
    }

    img.save(&output_path)
        .context("Failed to save output image")?;

    info!("Saved processed image to: {:?}", output_path);
    Ok(())
}
