mod request;

use crate::request::load;
use eframe::{egui, epi};
use std::{env, error::Error};

#[derive(Default)]
struct Browser {
    url: String,
    body: Option<Result<String, Box<dyn Error>>>,
}

impl Browser {
    fn selectable_text(&mut self, ui: &mut egui::Ui) {
        let body_str: String;
        let text = match self.body.as_ref().unwrap() {
            Ok(t) => t,
            Err(e) => {
                body_str = e.to_string();
                &body_str
            }
        };
        ui.label(egui::RichText::new(text));
    }
}

impl epi::App for Browser {
    fn name(&self) -> &str {
        "My Browser App"
    }

    fn setup(
        &mut self,
        ctx: &egui::Context,
        frame: &epi::Frame,
        storage: Option<&dyn epi::Storage>,
    ) {
        ctx.begin_frame(egui::RawInput::default());
        self.body = Some(load(&self.url));
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(&self.url);
                ui.add_space(10.0);
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.selectable_text(ui);
                });
            });
        });
    }
}

fn main() {
    let app = Browser {
        url: env::args().nth(1).unwrap(),
        ..Default::default()
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
