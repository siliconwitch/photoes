use adw::prelude::*;

fn main() -> gtk::glib::ExitCode {
    let application = adw::Application::builder()
        .application_id("io.github.siliconwitch.Photoes")
        .build();

    application.connect_activate(|application| {
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("Photoes")
            .default_width(1200)
            .default_height(800)
            .build();

        window.present();
    });

    application.run()
}
