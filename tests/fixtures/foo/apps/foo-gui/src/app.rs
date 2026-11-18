//! The window. It only shows what the `core` crate computes; the logic lives there.

use foo_core::Greeting;

pub struct App {
    greeting: Greeting,
}

impl Default for App {
    fn default() -> Self {
        Self {
            greeting: Greeting::new("world"),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(self.greeting.to_string());
        });
    }
}
