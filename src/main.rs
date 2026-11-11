mod config;
mod date_view;
mod folder_view;
mod library;
mod location_view;
mod map_source;
mod thumbnail;

use adw::prelude::*;
use gtk::{gdk, glib};

fn main() -> glib::ExitCode {
    let application = adw::Application::builder()
        .application_id("io.github.siliconwitch.Photoes")
        .build();

    application.connect_startup(|_| {
        let style = gtk::CssProvider::new();
        style.load_from_string(include_str!("style.css"));

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("a display"),
            &style,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    application.connect_activate(|application| {
        let photos = library::scan(&config::load());

        let thumbnails = thumbnail::Store::new();

        let date_view = date_view::new(&thumbnails);
        let location_view = location_view::new(&thumbnails);
        let folder_view = folder_view::new(&thumbnails);

        let views = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(120)
            .build();
        views.add_named(&date_view.widget, Some("date"));
        views.add_named(&location_view.widget, Some("location"));
        views.add_named(&folder_view.widget, Some("folders"));

        let tabs = gtk::Box::builder()
            .css_classes(["glass"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .margin_bottom(16)
            .spacing(4)
            .build();

        let mut first_tab: Option<gtk::ToggleButton> = None;
        for (view, icon, tooltip) in [
            ("date", "preferences-system-time-symbolic", "Date"),
            ("location", "mark-location-symbolic", "Location"),
            ("folders", "folder-symbolic", "Folders"),
        ] {
            let tab = gtk::ToggleButton::builder()
                .icon_name(icon)
                .tooltip_text(tooltip)
                .css_classes(["flat"])
                .build();

            tab.connect_toggled({
                let views = views.clone();
                move |tab| {
                    if tab.is_active() {
                        views.set_visible_child_name(view);
                    }
                }
            });

            match &first_tab {
                Some(first_tab) => tab.set_group(Some(first_tab)),
                None => {
                    tab.set_active(true);
                    first_tab = Some(tab.clone());
                }
            }

            tabs.append(&tab);
        }

        let favourites = gtk::ToggleButton::builder()
            .icon_name("starred-symbolic")
            .tooltip_text("Favourites")
            .css_classes(["glass"])
            .build();
        let inbox = gtk::ToggleButton::builder()
            .icon_name("folder-download-symbolic")
            .tooltip_text("Inbox")
            .css_classes(["glass"])
            .build();

        let filters = gtk::Box::builder()
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .margin_start(16)
            .margin_top(16)
            .spacing(8)
            .build();
        filters.append(&favourites);
        filters.append(&inbox);

        let query = gtk::Text::builder()
            .placeholder_text("Search")
            .width_chars(18)
            .build();
        let query_revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .transition_duration(160)
            .child(&query)
            .build();
        let search = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search")
            .css_classes(["flat"])
            .build();

        search
            .bind_property("active", &query_revealer, "reveal-child")
            .build();
        search.connect_toggled(move |search| {
            if search.is_active() {
                query.grab_focus();
            }
        });

        let search_box = gtk::Box::builder()
            .css_classes(["glass"])
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_end(16)
            .margin_top(16)
            .build();
        search_box.append(&query_revealer);
        search_box.append(&search);

        let overlay = gtk::Overlay::builder().child(&views).build();
        overlay.add_overlay(&filters);
        overlay.add_overlay(&search_box);
        overlay.add_overlay(&tabs);

        let show = move |inbox_only: bool| {
            let shown: Vec<library::Photo> = photos
                .iter()
                .filter(|photo| photo.unsorted || !inbox_only)
                .cloned()
                .collect();

            date_view.show(&shown);
            location_view.show(&shown);
            folder_view.show(&shown);
        };

        show(false);
        inbox.connect_toggled(move |inbox| show(inbox.is_active()));

        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("Photoes")
            .default_width(1200)
            .default_height(800)
            .width_request(360)
            .height_request(480)
            .content(&overlay)
            .build();

        window.present();
    });

    application.run()
}
