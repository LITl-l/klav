mod app;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([480.0, 360.0])
            .with_title("Klav"),
        ..Default::default()
    };

    eframe::run_native(
        "Klav",
        options,
        Box::new(|_cc| Ok(Box::new(app::KlavApp::new()))),
    )
}
