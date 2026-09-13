//! Saving a generated image to disk (2.24.0).
//!
//! One command, and deliberately the dumbest possible one: it takes bytes and
//! writes them where the user's own save dialog said to. It exists because the
//! Recap's shareable card is drawn in the frontend (as SVG, rasterised on a
//! canvas) and a webview cannot write a file on its own, and because this app
//! does not carry `@tauri-apps/plugin-fs` - adding a whole filesystem plugin
//! to write one PNG would be a much bigger surface than this.
//!
//! It contains NO business logic and reads nothing: no database handle, no
//! state, no knowledge of what the picture is of. That is the point - the
//! Recap is a presentation layer over existing data, and this is the last inch
//! of it.

use crate::error::{AppError, AppResult};
use base64::Engine;

/// Writes a base64-encoded PNG to `dest_path`.
///
/// `data_base64` is the payload of a `data:image/png;base64,...` URL with the
/// prefix already stripped by the caller - the frontend produces exactly that
/// from `canvas.toDataURL`, the same way `lib/aiImport.ts` already does for
/// screenshots it sends to the model.
///
/// Refuses anything that is not a PNG rather than writing a file that will not
/// open: the first eight bytes of a PNG are a fixed signature, so this is a
/// real check, not a guess at the extension.
#[tauri::command(async)]
pub fn save_png_file(dest_path: String, data_base64: String) -> AppResult<()> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim())
        .map_err(|_| AppError::Validation("That image could not be read.".to_string()))?;
    if !is_png(&bytes) {
        return Err(AppError::Validation(
            "That file was not a PNG, so nothing was written.".to_string(),
        ));
    }
    std::fs::write(&dest_path, &bytes)
        .map_err(|e| AppError::Io(format!("Couldn't save the image: {e}")))?;
    Ok(())
}

/// The PNG signature, byte for byte (RFC 2083 §3.1).
fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_png_signature_is_accepted() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01];
        assert!(is_png(&png));
    }

    #[test]
    fn anything_that_is_not_a_png_is_refused_rather_than_written() {
        // A JPEG, an empty payload and a truncated signature are all "no".
        assert!(!is_png(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(!is_png(&[]));
        assert!(!is_png(&[0x89, b'P', b'N']));
        // And plain text, which is what a mis-stripped data URL would give.
        assert!(!is_png(b"data:image/png;base64,iVBOR"));
    }

    #[test]
    fn base64_that_is_not_base64_is_a_clear_refusal_not_a_panic() {
        let err = save_png_file("/tmp/tiqr-never-written.png".into(), "not base64 @@@".into());
        assert!(err.is_err());
        assert!(!std::path::Path::new("/tmp/tiqr-never-written.png").exists());
    }
}
