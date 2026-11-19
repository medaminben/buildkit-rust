//! The window. It only shows what the `core` crate computes; the logic lives there.

use crate::capture;
use crate::cli::Screen;
use foo_core::Greeting;

pub struct App {
    screen: Screen,
    greeting: Greeting,
    capture: Option<capture::Run>,
}

impl App {
    pub fn new(screen: Screen) -> Self {
        Self {
            screen,
            greeting: Greeting::new("world"),
            capture: None,
        }
    }

    /// Makes this a capture run: the window takes a screenshot, writes it and closes.
    #[must_use]
    pub fn with_capture(mut self, run: capture::Run) -> Self {
        self.capture = Some(run);
        self
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Main => {
                eframe::egui::CentralPanel::default().show(ui, |ui| {
                    ui.heading(self.greeting.to_string());
                });
            }
        }
        if let Some(run) = &mut self.capture {
            run.frame(&ui.ctx().clone());
        }
    }
}
