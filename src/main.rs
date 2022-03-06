mod request;

use crate::request::load;
use eframe::{egui, epi};
use std::{env, error::Error};

#[derive(Default)]
struct Browser {
    url: String,
}

fn selectable_text(ui: &mut egui::Ui, text: Result<String, Box<dyn Error>>) {
    let mut text = match text {
        Ok(t) => t,
        Err(e) => e.to_string(),
    };
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace),
    );
}

impl epi::App for Browser {
    fn name(&self) -> &str {
        "My Browser App"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(&self.url);
                ui.add_space(10.0);
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    selectable_text(ui, load(&self.url));
                });
            });
        });
    }
}

fn main() {
    let app = Browser {
        url: env::args().nth(1).unwrap(),
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
