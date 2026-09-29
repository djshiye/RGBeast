use std::{cell::RefCell, rc::Rc};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{CompositeTemplate, glib};

use crate::{
    i18n::gettext,
    settings::{self, settings},
    window::RGBeastWindow,
};

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/io/github/djshiye/RGBeast/ui/preferences.ui")]
    pub struct RGBeastPreferences {
        #[template_child]
        pub animate_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub resume_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub version_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub log_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub headers_group: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RGBeastPreferences {
        const NAME: &'static str = "RGBeastPreferences";
        type Type = super::RGBeastPreferences;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RGBeastPreferences {
        fn constructed(&self) {
            self.parent_constructed();
            settings()
                .bind(settings::ANIMATE_PREVIEW, &*self.animate_row, "active")
                .build();
        }
    }
    impl WidgetImpl for RGBeastPreferences {}
    impl AdwDialogImpl for RGBeastPreferences {}
    impl PreferencesDialogImpl for RGBeastPreferences {}
}

glib::wrapper! {
    pub struct RGBeastPreferences(ObjectSubclass<imp::RGBeastPreferences>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RGBeastPreferences {
    pub fn new(window: &RGBeastWindow) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.fill(window);
        dialog
    }

    fn fill(&self, window: &RGBeastWindow) {
        let imp = self.imp();
        let Some(client) = window.client() else {
            imp.version_row.set_subtitle(&gettext("Not connected"));
            imp.resume_row.set_sensitive(false);
            return;
        };

        // Service facts.
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = dialog)]
            self,
            #[strong]
            client,
            async move {
                let imp = dialog.imp();
                let version = client.version().await.unwrap_or_default();
                let simulated = client.simulated().await.unwrap_or(false);
                imp.version_row.set_subtitle(&if simulated {
                    gettext("{v} (simulated devices)").replace("{v}", &version)
                } else {
                    version
                });
                if let Ok(on) = client.restore_on_resume().await {
                    imp.resume_row.set_active(on);
                }
                imp.resume_row.connect_active_notify(glib::clone!(
                    #[strong]
                    client,
                    move |row| {
                        let on = row.is_active();
                        let client = client.clone();
                        glib::spawn_future_local(async move {
                            if let Err(e) = client.set_restore_on_resume(on).await {
                                tracing::warn!("restore-on-resume: {e}");
                            }
                        });
                    }
                ));
                if let Ok(lines) = client.discovery_log().await {
                    for line in lines {
                        let row = adw::ActionRow::builder()
                            .title(glib::markup_escape_text(&line))
                            .build();
                        row.add_css_class("property");
                        imp.log_row.add_row(&row);
                    }
                }
            }
        ));

        // Addressable header sizes for every device that has them.
        let mut any = false;
        for device in window.devices() {
            let info = device.info();
            for z in info.zones.iter().filter(|z| z.is_sizable()) {
                any = true;
                let row = adw::SpinRow::with_range(z.leds_min as f64, z.leds_max as f64, 1.0);
                row.set_title(&format!("{} · {}", info.name, z.name));
                row.set_subtitle(&gettext("LEDs connected"));
                row.set_value(z.leds as f64);
                let id = info.id.clone();
                let zone = z.id.clone();
                // Debounced like the page's zone rows: one daemon call per pause.
                let pending: Rc<RefCell<Option<glib::SourceId>>> = Default::default();
                row.connect_value_notify(glib::clone!(
                    #[weak]
                    window,
                    move |r| {
                        let leds = r.value().round() as u32;
                        if let Some(old) = pending.borrow_mut().take() {
                            old.remove();
                        }
                        let (id, zone, pending2) = (id.clone(), zone.clone(), pending.clone());
                        let source = glib::timeout_add_local_once(
                            std::time::Duration::from_millis(400),
                            glib::clone!(
                                #[weak]
                                window,
                                move || {
                                    pending2.borrow_mut().take();
                                    window.resize_zone(&id, &zone, leds);
                                }
                            ),
                        );
                        pending.borrow_mut().replace(source);
                    }
                ));
                imp.headers_group.add(&row);
            }
        }
        imp.headers_group.set_visible(any);
    }
}
