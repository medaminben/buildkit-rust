//! `--capture=<path>`: lets the window draw at least 10 frames, takes one screenshot, writes it as
//! PNG and closes the window. A failure prints `capture failed: <reason>` and exits 1 (see `main`).
//! `cargo bk manual capture` drives this.

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The window of a capture run, in logical pixels.
pub const WINDOW: [f32; 2] = [1280.0, 800.0];
/// Frames drawn before the screenshot is asked for.
pub const MIN_FRAMES: u32 = 10;
/// The screenshot is given up as lost this many frames after the request, so a run cannot hang.
const IMAGE_LIMIT: u32 = 600;

/// What the capture does after a frame.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Wait,
    /// Ask the window for a screenshot now.
    Request,
    /// The screenshot is there: write it and close.
    Write,
    Fail(&'static str),
}

/// The capture's progress, frame by frame. It knows nothing of the window.
#[derive(Debug, Default)]
pub struct Progress {
    frames: u32,
    requested_at: Option<u32>,
}

impl Progress {
    /// Call once per drawn frame. `image_ready` says the screenshot has arrived.
    pub fn frame(&mut self, image_ready: bool) -> Step {
        self.frames += 1;
        match self.requested_at {
            Some(_) if image_ready => Step::Write,
            Some(at) if self.frames - at > IMAGE_LIMIT => {
                Step::Fail("the window gave no screenshot")
            }
            None if self.frames >= MIN_FRAMES => {
                self.requested_at = Some(self.frames);
                Step::Request
            }
            Some(_) | None => Step::Wait,
        }
    }
}

/// How a capture run ended: `Ok` once the PNG is written.
pub type Outcome = Arc<Mutex<Option<Result<(), String>>>>;

/// What `main` reports: a window error, or a window closed early, is a failure too.
pub fn finish(window: Result<(), String>, outcome: &Outcome) -> Result<(), String> {
    window?;
    outcome
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .unwrap_or_else(|| Err("the window closed before the screenshot was taken".to_owned()))
}

/// A capture run inside the window: the progress, the screenshot slot and the outcome.
pub struct Run {
    path: PathBuf,
    progress: Progress,
    image: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
    outcome: Outcome,
}

impl Run {
    pub fn new(path: PathBuf, outcome: Outcome) -> Self {
        Self {
            path,
            progress: Progress::default(),
            image: Arc::default(),
            outcome,
        }
    }

    /// Call at the end of every frame. Keeps the window drawing, asks for the screenshot, writes
    /// it and closes the window.
    pub fn frame(&mut self, ctx: &egui::Context) {
        let ready = self.image.lock().is_ok_and(|slot| slot.is_some());
        ctx.request_repaint();
        let result = match self.progress.frame(ready) {
            Step::Wait => return,
            Step::Request => {
                let slot = Arc::clone(&self.image);
                let wake = ctx.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                    egui::ScreenshotCallback::new(move |image| {
                        if let Ok(mut slot) = slot.lock() {
                            *slot = Some(image);
                        }
                        wake.request_repaint();
                    }),
                ));
                return;
            }
            Step::Write => self.write(),
            Step::Fail(reason) => Err(reason.to_owned()),
        };
        if let Ok(mut outcome) = self.outcome.lock() {
            *outcome = Some(result);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn write(&self) -> Result<(), String> {
        let image = self
            .image
            .lock()
            .map_err(|e| e.to_string())?
            .take()
            .ok_or("the screenshot is gone")?;
        write_png(&self.path, &image)
    }
}

/// The pixels as opaque 8-bit RGB rows.
pub fn rgb_bytes(image: &egui::ColorImage) -> Vec<u8> {
    image
        .pixels
        .iter()
        .flat_map(|p| [p.r(), p.g(), p.b()])
        .collect()
}

fn encode(image: &egui::ColorImage, file: std::fs::File) -> Result<(), String> {
    let [width, height] = image.size;
    let dimension = |n: usize| u32::try_from(n).map_err(|e| e.to_string());
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        dimension(width)?,
        dimension(height)?,
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&rgb_bytes(image))
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

/// Where the PNG is written first; renaming it onto `path` makes the file appear whole or not at all.
pub fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Writes `image` as PNG to a temporary file next to `path`, then renames it to `path`. A failure
/// leaves no temporary file behind.
pub fn write_png(path: &Path, image: &egui::ColorImage) -> Result<(), String> {
    let temp = temp_path(path);
    let result = std::fs::File::create(&temp)
        .map_err(|e| format!("cannot write {}: {e}", temp.display()))
        .and_then(|file| encode(image, file))
        .and_then(|()| {
            std::fs::rename(&temp, path)
                .map_err(|e| format!("cannot write {}: {e}", path.display()))
        });
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screenshot_is_asked_for_once_after_ten_frames() {
        let mut progress = Progress::default();
        for _ in 1..MIN_FRAMES {
            assert_eq!(progress.frame(false), Step::Wait);
        }
        assert_eq!(progress.frame(false), Step::Request);
        assert_eq!(progress.frame(false), Step::Wait);
        assert_eq!(progress.frame(true), Step::Write);
    }

    #[test]
    fn a_lost_screenshot_fails_instead_of_hanging() {
        let mut progress = Progress::default();
        for _ in 1..MIN_FRAMES {
            progress.frame(false);
        }
        assert_eq!(progress.frame(false), Step::Request);
        for _ in 0..IMAGE_LIMIT {
            assert_eq!(progress.frame(false), Step::Wait);
        }
        assert_eq!(
            progress.frame(false),
            Step::Fail("the window gave no screenshot")
        );
    }

    #[test]
    fn write_png_makes_a_png_and_leaves_no_temp_file() {
        let dir = tempdir("ok");
        let path = dir.join("shot.png");
        let pixels = vec![egui::Color32::from_rgb(1, 2, 3); 6];
        write_png(&path, &egui::ColorImage::new([3, 2], pixels)).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert!(!temp_path(&path).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_write_error_names_the_path_and_leaves_nothing() {
        let dir = tempdir("missing");
        let path = dir.join("no-such-dir").join("shot.png");
        let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
        let error = write_png(&path, &image).unwrap_err();
        assert!(error.starts_with("cannot write "), "{error}");
        assert!(!path.exists() && !temp_path(&path).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn finish_reports_the_outcome_or_the_early_close() {
        let outcome: Outcome = Arc::default();
        assert_eq!(
            finish(Ok(()), &outcome),
            Err("the window closed before the screenshot was taken".to_owned())
        );
        *outcome.lock().unwrap() = Some(Ok(()));
        assert_eq!(finish(Ok(()), &outcome), Ok(()));
        assert_eq!(finish(Err("gpu".into()), &outcome), Err("gpu".to_owned()));
    }

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("capture-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
