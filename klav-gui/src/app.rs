use eframe::egui;

/// Klav settings GUI application.
///
/// Phase 0: Minimal placeholder. Full GUI in Phase 3.
pub struct KlavApp {
    enabled: bool,
    current_language: String,
}

impl KlavApp {
    pub fn new() -> Self {
        Self {
            enabled: false,
            current_language: "Japanese".to_string(),
        }
    }
}

impl eframe::App for KlavApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Klav");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Engine:");
                let label = if self.enabled { "ON" } else { "OFF" };
                if ui.toggle_value(&mut self.enabled, label).changed() {
                    log::info!("engine toggled: {}", self.enabled);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Language:");
                ui.label(&self.current_language);
            });

            ui.separator();
            ui.label("Settings GUI will be implemented in Phase 3.");
        });
    }
}
