use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};

use crate::{config, preferences::RGBeastPreferences, window::RGBeastWindow};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct RGBeastApplication {
        pub window: OnceCell<RGBeastWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RGBeastApplication {
        const NAME: &'static str = "RGBeastApplication";
        type Type = super::RGBeastApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for RGBeastApplication {}

    impl ApplicationImpl for RGBeastApplication {
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            app.setup_css();
            app.setup_actions();
        }

        fn activate(&self) {
            self.parent_activate();
            let app = self.obj();
            let window = self.window.get_or_init(|| RGBeastWindow::new(&*app));
            window.present();
        }
    }

    impl GtkApplicationImpl for RGBeastApplication {}
    impl AdwApplicationImpl for RGBeastApplication {}
}

glib::wrapper! {
    pub struct RGBeastApplication(ObjectSubclass<imp::RGBeastApplication>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for RGBeastApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl RGBeastApplication {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", config::APP_ID)
            .property("resource-base-path", config::RESOURCE_PREFIX)
            .build()
    }

    pub fn window(&self) -> Option<&RGBeastWindow> {
        self.imp().window.get()
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource(&format!("{}/style.css", config::RESOURCE_PREFIX));
        // One notch above the user stylesheet so third-party themes cannot
        // erase the card design; the rules only target RGBeast's own classes.
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().expect("no display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        );
    }

    fn setup_actions(&self) {
        let quit = gio::ActionEntry::builder("quit")
            .activate(|app: &Self, _, _| app.quit())
            .build();
        let about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| app.show_about())
            .build();
        let preferences = gio::ActionEntry::builder("preferences")
            .activate(|app: &Self, _, _| {
                if let Some(w) = app.window() {
                    RGBeastPreferences::new(w).present(Some(w));
                }
            })
            .build();
        let rescan = gio::ActionEntry::builder("rescan")
            .activate(|app: &Self, _, _| {
                if let Some(w) = app.window() {
                    w.rescan();
                }
            })
            .build();
        let lights_off = gio::ActionEntry::builder("lights-off")
            .activate(|app: &Self, _, _| {
                if let Some(w) = app.window() {
                    w.lights_off();
                }
            })
            .build();
        self.add_action_entries([quit, about, preferences, rescan, lights_off]);
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("app.preferences", &["<Control>comma"]);
        self.set_accels_for_action("app.rescan", &["<Control>r", "F5"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
    }

    fn show_about(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_name(config::APP_NAME)
            .application_icon(config::APP_ID)
            .version(config::VERSION)
            .developer_name("djshiye")
            .license_type(gtk::License::MitX11)
            .website("https://github.com/djshiye/RGBeast")
            .issue_url("https://github.com/djshiye/RGBeast/issues")
            .comments(crate::i18n::gettext("RGB lighting control for ASUS Aura motherboards, addressable fans, Kingston Fury memory and ASUS graphics cards."))
            .build();
        dialog.present(self.active_window().as_ref());
    }
}
