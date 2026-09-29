//! The main window: a sidebar of devices and scenes, and the editor page.
//! It owns the D-Bus client and is the only place that calls it.

use std::cell::{Cell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{CompositeTemplate, gio, glib};
use rgbeast_core::{DeviceState, Rgb};

use crate::{
    client::{Client, ClientError},
    i18n::{gettext, ngettext},
    model::{Device, map_group_state, preview_colors},
    scenes::{Scene, SceneStore},
    settings::{self, settings},
    ui::{DevicePage, DeviceRow, device_page::zone_shape},
    widgets::{ColorStrip, led_preview::ZoneLayout},
};

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/io/github/djshiye/RGBeast/ui/window.ui")]
    pub struct RGBeastWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub device_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub scene_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub status_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub content_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub applied_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub view_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub device_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub retry_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub rescan_button: TemplateChild<gtk::Button>,

        pub client: RefCell<Option<Client>>,
        pub devices: RefCell<Vec<Device>>,
        pub group: RefCell<Option<Device>>,
        pub page: DevicePage,
        pub scenes: RefCell<Option<SceneStore>>,
        pub applied_source: RefCell<Option<glib::SourceId>>,
        pub reload_pending: Cell<bool>,
        pub selecting: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RGBeastWindow {
        const NAME: &'static str = "RGBeastWindow";
        type Type = super::RGBeastWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            DevicePage::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RGBeastWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.scenes.replace(Some(SceneStore::load()));
            self.group.replace(Some(Device::all_devices()));
            obj.setup_page();
            obj.setup_sidebar();
            obj.setup_actions();
            obj.setup_settings();
            obj.rebuild_scenes();
            obj.connect_daemon();
        }
    }
    impl WidgetImpl for RGBeastWindow {}
    impl WindowImpl for RGBeastWindow {}
    impl ApplicationWindowImpl for RGBeastWindow {}
    impl AdwApplicationWindowImpl for RGBeastWindow {}
}

glib::wrapper! {
    pub struct RGBeastWindow(ObjectSubclass<imp::RGBeastWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl RGBeastWindow {
    pub fn new(app: &impl IsA<gtk::Application>) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn client(&self) -> Option<Client> {
        self.imp().client.borrow().clone()
    }

    pub fn devices(&self) -> Vec<Device> {
        self.imp().devices.borrow().clone()
    }

    fn group(&self) -> Device {
        self.imp().group.borrow().clone().expect("group")
    }

    // ── Setup ────────────────────────────────────────────────────────────

    fn setup_page(&self) {
        let imp = self.imp();
        imp.device_bin.set_child(Some(&imp.page));
        imp.page.connect_apply(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |page| win.apply_from_page(page)
        ));
        imp.page.connect_resize_zone(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |page, zone, leds| {
                if let Some(d) = page.device() {
                    win.resize_zone(&d.id(), &zone, leds);
                }
            }
        ));
        imp.save_button.connect_clicked(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |_| win.save_to_device()
        ));
        imp.retry_button.connect_clicked(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |_| win.connect_daemon()
        ));
        imp.rescan_button.connect_clicked(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |_| win.rescan()
        ));
    }

    fn setup_sidebar(&self) {
        let imp = self.imp();
        imp.device_list.connect_row_selected(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |_, row| {
                if win.imp().selecting.get() {
                    return;
                }
                if let Some(d) = row
                    .and_then(|r| r.child())
                    .and_downcast::<DeviceRow>()
                    .and_then(|r| r.device())
                {
                    win.show_device(&d);
                }
            }
        ));
        imp.scene_list.connect_row_activated(glib::clone!(
            #[weak(rename_to = win)]
            self,
            move |_, row| {
                let name = row.widget_name();
                match name.as_str() {
                    "__save" => win.save_scene_dialog(),
                    "__off" => win.lights_off(),
                    n => win.apply_scene_named(n),
                }
            }
        ));
    }

    fn setup_actions(&self) {
        let delete = gio::ActionEntry::builder("scene-delete")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(|win: &Self, _, param| {
                if let Some(name) = param.and_then(|p| p.get::<String>()) {
                    win.delete_scene(&name);
                }
            })
            .build();
        let rename = gio::ActionEntry::builder("scene-rename")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(|win: &Self, _, param| {
                if let Some(name) = param.and_then(|p| p.get::<String>()) {
                    win.rename_scene_dialog(&name);
                }
            })
            .build();
        self.add_action_entries([delete, rename]);
    }

    fn setup_settings(&self) {
        let s = settings();
        let (mut w, mut h) = (
            s.int(settings::WINDOW_WIDTH),
            s.int(settings::WINDOW_HEIGHT),
        );
        #[cfg(debug_assertions)]
        if let Some((dw, dh)) = std::env::var("RGBEAST_DEBUG_SIZE").ok().and_then(|v| {
            let (a, b) = v.split_once('x')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        }) {
            w = dw;
            h = dh;
        }
        self.set_default_size(w, h);
        if s.boolean(settings::WINDOW_MAXIMIZED) {
            self.maximize();
        }
        self.imp()
            .page
            .set_animate(s.boolean(settings::ANIMATE_PREVIEW));
        s.connect_changed(
            Some(settings::ANIMATE_PREVIEW),
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                move |s, key| win.imp().page.set_animate(s.boolean(key))
            ),
        );
        // Debug builds: RGBEAST_DEBUG_SHOT=<file.png> renders the window to a
        // PNG after the first devices load and quits (docs screenshots, layout checks).
        #[cfg(debug_assertions)]
        if let Some(path) = std::env::var_os("RGBEAST_DEBUG_SHOT") {
            let path = std::path::PathBuf::from(path);
            glib::timeout_add_local_once(
                std::time::Duration::from_millis(1500),
                glib::clone!(
                    #[weak(rename_to = win)]
                    self,
                    move || {
                        win.debug_screenshot(&path);
                        win.close();
                    }
                ),
            );
        }
        self.connect_close_request(|win| {
            let s = settings();
            let (w, h) = win.default_size();
            s.set_int(settings::WINDOW_WIDTH, w).ok();
            s.set_int(settings::WINDOW_HEIGHT, h).ok();
            s.set_boolean(settings::WINDOW_MAXIMIZED, win.is_maximized())
                .ok();
            glib::Propagation::Proceed
        });
    }

    // ── Daemon connection ────────────────────────────────────────────────

    fn connect_daemon(&self) {
        let imp = self.imp();
        imp.view_stack.set_visible_child_name("loading");
        imp.status_label.set_label(&gettext("Connecting…"));
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            async move {
                match Client::connect().await {
                    Ok(client) => {
                        win.imp().client.replace(Some(client.clone()));
                        win.subscribe(client.clone());
                        win.load_devices().await;
                    }
                    Err(e) => {
                        tracing::warn!("daemon unavailable: {e}");
                        win.imp().client.replace(None);
                        win.imp().view_stack.set_visible_child_name("disconnected");
                        win.imp()
                            .status_icon
                            .set_icon_name(Some("network-offline-symbolic"));
                        win.imp()
                            .status_label
                            .set_label(&gettext("Lighting service not running"));
                    }
                }
            }
        ));
    }

    fn subscribe(&self, client: Client) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            #[strong]
            client,
            async move {
                use futures_lite::StreamExt;
                let Ok(mut stream) = client.state_changed().await else {
                    return;
                };
                while let Some(sig) = stream.next().await {
                    let Ok(args) = sig.args() else { continue };
                    if let Ok(state) = serde_json::from_str::<DeviceState>(args.state()) {
                        win.on_state_changed(args.id(), state);
                    }
                }
            }
        ));
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            async move {
                use futures_lite::StreamExt;
                let Ok(mut stream) = client.devices_changed().await else {
                    return;
                };
                while stream.next().await.is_some() {
                    win.schedule_reload();
                }
            }
        ));
    }

    fn schedule_reload(&self) {
        if self.imp().reload_pending.replace(true) {
            return;
        }
        glib::timeout_add_local_once(
            std::time::Duration::from_millis(300),
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                move || {
                    win.imp().reload_pending.set(false);
                    glib::spawn_future_local(async move { win.load_devices().await });
                }
            ),
        );
    }

    async fn load_devices(&self) {
        let imp = self.imp();
        let Some(client) = self.client() else { return };
        let infos = match client.list_devices().await {
            Ok(i) => i,
            Err(e) => {
                self.report(&e);
                return;
            }
        };
        let mut devices = Vec::with_capacity(infos.len());
        for info in infos {
            let state = client.get_state(&info.id).await.unwrap_or_default();
            // Reuse existing objects so rows and the page keep their identity.
            let existing = imp
                .devices
                .borrow()
                .iter()
                .find(|d| d.id() == info.id)
                .cloned();
            match existing {
                Some(d) => {
                    d.set_info(info);
                    d.set_state(state);
                    devices.push(d);
                }
                None => devices.push(Device::new(info, state)),
            }
        }
        imp.devices.replace(devices);

        let simulated = client.simulated().await.unwrap_or(false);
        let n = imp.devices.borrow().len();
        let mut status =
            ngettext("{n} device", "{n} devices", n as u32).replace("{n}", &n.to_string());
        if simulated {
            status.push_str(" · ");
            status.push_str(&gettext("simulated"));
        }
        imp.status_label.set_label(&status);
        imp.status_icon.set_icon_name(Some("emblem-ok-symbolic"));

        self.rebuild_sidebar();
        if n == 0 {
            imp.view_stack.set_visible_child_name("empty");
            return;
        }
        imp.view_stack.set_visible_child_name("device");
        let mut last = settings().string(settings::LAST_DEVICE).to_string();
        // Debug builds: RGBEAST_DEBUG_DEVICE=<id> opens on that device (screenshots, layout checks).
        #[cfg(debug_assertions)]
        if let Ok(v) = std::env::var("RGBEAST_DEBUG_DEVICE") {
            last = v;
        }
        let target = imp
            .devices
            .borrow()
            .iter()
            .position(|d| d.id() == last.as_str())
            .map(|i| i + 1)
            .unwrap_or(if last == crate::model::ALL_ID || n > 1 {
                0
            } else {
                1
            });
        if let Some(row) = imp.device_list.row_at_index(target as i32) {
            imp.device_list.select_row(Some(&row));
        }
    }

    fn rebuild_sidebar(&self) {
        let imp = self.imp();
        imp.selecting.set(true);
        while let Some(child) = imp.device_list.first_child() {
            imp.device_list.remove(&child);
        }
        let mut all = vec![self.group()];
        all.extend(imp.devices.borrow().iter().cloned());
        for d in all {
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&DeviceRow::new(&d)));
            imp.device_list.append(&row);
        }
        imp.selecting.set(false);
    }

    fn on_state_changed(&self, id: &str, state: DeviceState) {
        let imp = self.imp();
        if let Some(d) = imp.devices.borrow().iter().find(|d| d.id() == id) {
            d.set_state(state.clone());
            if imp.page.device().is_some_and(|p| p.id() == id) {
                imp.page.set_confirmed_state(state);
            }
        }
    }

    // ── Editing ──────────────────────────────────────────────────────────

    fn show_device(&self, device: &Device) {
        let imp = self.imp();
        imp.content_page.set_title(&device.info().name);
        imp.save_button.set_visible(device.info().can_save);
        let preview = if device.is_group() {
            self.group_preview()
        } else {
            Vec::new()
        };
        imp.page.set_device(device, preview);
        imp.view_stack.set_visible_child_name("device");
        settings()
            .set_string(settings::LAST_DEVICE, &device.id())
            .ok();
        if imp.split_view.is_collapsed() {
            imp.split_view.set_show_content(true);
        }
    }

    /// Every real device's zones, one row per device, with short labels.
    fn group_preview(&self) -> Vec<ZoneLayout> {
        let mut out = Vec::new();
        for d in self.imp().devices.borrow().iter() {
            let info = d.info();
            let state = d.state();
            let tag = match info.kind {
                rgbeast_core::DeviceKind::Motherboard => gettext("Board"),
                rgbeast_core::DeviceKind::Dram => gettext("RAM"),
                rgbeast_core::DeviceKind::Gpu => gettext("GPU"),
                _ => short_name(&info.name),
            };
            let mut first = true;
            for (z, colors) in preview_colors(&info, &state) {
                if colors.is_empty() {
                    continue;
                }
                let zone_name = z.name.replace("Addressable Header", &gettext("Header"));
                let name = if info.zones.len() <= 1 || z.id == "mainboard" {
                    tag.clone()
                } else if info.kind == rgbeast_core::DeviceKind::Dram {
                    zone_name
                } else {
                    format!("{tag} · {zone_name}")
                };
                out.push(ZoneLayout {
                    name,
                    shape: zone_shape(&info, &z),
                    colors,
                    row_break: first,
                });
                first = false;
            }
        }
        out
    }

    fn apply_from_page(&self, page: &DevicePage) {
        let Some(device) = page.device() else { return };
        let state = page.state();
        if device.is_group() {
            device.set_state(state.clone());
            self.apply_to_all(state);
        } else {
            self.apply_state(device, state, true);
        }
    }

    /// Send one state to the daemon and adopt the normalised result.
    fn apply_state(&self, device: Device, state: DeviceState, feedback: bool) {
        let Some(client) = self.client() else { return };
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            async move {
                match client.set_state(&device.id(), &state).await {
                    Ok(applied) => {
                        device.set_state(applied.clone());
                        let imp = win.imp();
                        if imp.page.device().is_some_and(|p| p.id() == device.id()) {
                            imp.page.set_confirmed_state(applied);
                        }
                        if feedback {
                            win.flash_applied();
                        }
                    }
                    Err(e) => win.report(&e),
                }
            }
        ));
    }

    fn apply_to_all(&self, group: DeviceState) {
        for d in self.devices() {
            if let Some(mapped) = map_group_state(&d.info(), &group) {
                self.apply_state(d, mapped, false);
            }
        }
        self.flash_applied();
        // Refresh the group preview once the devices report back.
        glib::timeout_add_local_once(
            std::time::Duration::from_millis(250),
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                move || {
                    let imp = win.imp();
                    if imp.page.device().is_some_and(|d| d.is_group()) {
                        imp.page.set_device(&win.group(), win.group_preview());
                    }
                }
            ),
        );
    }

    pub fn resize_zone(&self, id: &str, zone: &str, leds: u32) {
        let Some(client) = self.client() else { return };
        let id = id.to_string();
        let zone = zone.to_string();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            async move {
                match client.set_zone_leds(&id, &zone, leds).await {
                    Ok(info) => {
                        let imp = win.imp();
                        if let Some(d) = imp.devices.borrow().iter().find(|d| d.id() == id) {
                            d.set_info(info.clone());
                        }
                        if imp.page.device().is_some_and(|p| p.id() == id) {
                            imp.page.set_info(info);
                        }
                        if let Ok(state) = client.get_state(&id).await {
                            win.on_state_changed(&id, state);
                        }
                    }
                    Err(e) => win.report(&e),
                }
            }
        ));
    }

    fn save_to_device(&self) {
        let (Some(client), Some(device)) = (self.client(), self.imp().page.device()) else {
            return;
        };
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            async move {
                match client.save_to_device(&device.id()).await {
                    Ok(()) => win.toast(&gettext("Stored as the power-on lighting")),
                    Err(e) => win.report(&e),
                }
            }
        ));
    }

    pub fn rescan(&self) {
        let Some(client) = self.client() else {
            self.connect_daemon();
            return;
        };
        self.imp().status_label.set_label(&gettext("Scanning…"));
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = win)]
            self,
            async move {
                if let Err(e) = client.rescan().await {
                    win.report(&e);
                }
                win.load_devices().await;
            }
        ));
    }

    pub fn lights_off(&self) {
        self.group().set_state(DeviceState::off());
        self.apply_to_all(DeviceState::off());
    }

    // ── Scenes ───────────────────────────────────────────────────────────

    fn rebuild_scenes(&self) {
        let imp = self.imp();
        while let Some(child) = imp.scene_list.first_child() {
            imp.scene_list.remove(&child);
        }
        let off = self.scene_row(
            "__off",
            &gettext("Lights Off"),
            &[Rgb::new(0x24, 0x1F, 0x31)],
            false,
        );
        imp.scene_list.append(&off);
        let scenes: Vec<Scene> = imp
            .scenes
            .borrow()
            .as_ref()
            .map(|s| s.scenes.clone())
            .unwrap_or_default();
        for scene in &scenes {
            let row = self.scene_row(&scene.name, &scene.name, &scene.colors(), true);
            imp.scene_list.append(&row);
        }
        let save = self.scene_row("__save", &gettext("Save Current as Scene…"), &[], false);
        imp.scene_list.append(&save);
    }

    fn scene_row(
        &self,
        name: &str,
        label: &str,
        colors: &[Rgb],
        editable: bool,
    ) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(name);
        row.set_activatable(true);
        if name == "__save" {
            row.add_css_class("scene-add");
        }
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        if colors.is_empty() {
            let icon = gtk::Image::from_icon_name("list-add-symbolic");
            icon.set_pixel_size(16);
            icon.add_css_class("device-kind-icon");
            hbox.append(&icon);
        } else {
            let strip = ColorStrip::default();
            strip.set_colors(colors);
            strip.add_css_class("scene-swatch");
            strip.set_valign(gtk::Align::Center);
            hbox.append(&strip);
        }
        let l = gtk::Label::new(Some(label));
        l.set_xalign(0.0);
        l.set_hexpand(true);
        l.set_ellipsize(gtk::pango::EllipsizeMode::End);
        hbox.append(&l);
        if editable {
            let menu = gio::Menu::new();
            let target = name.to_variant();
            menu.append(
                Some(&gettext("Rename…")),
                Some(&gio::Action::print_detailed_name(
                    "win.scene-rename",
                    Some(&target),
                )),
            );
            menu.append(
                Some(&gettext("Delete")),
                Some(&gio::Action::print_detailed_name(
                    "win.scene-delete",
                    Some(&target),
                )),
            );
            let b = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .menu_model(&menu)
                .valign(gtk::Align::Center)
                .tooltip_text(gettext("Scene Actions"))
                .build();
            b.add_css_class("flat");
            b.add_css_class("circular");
            hbox.append(&b);
        }
        row.set_child(Some(&hbox));
        row
    }

    fn apply_scene_named(&self, name: &str) {
        let scene = self
            .imp()
            .scenes
            .borrow()
            .as_ref()
            .and_then(|s| s.scenes.iter().find(|s| s.name == name).cloned());
        let Some(scene) = scene else { return };
        for d in self.devices() {
            if let Some(state) = scene.states.get(&d.id()) {
                self.apply_state(d, state.clone(), false);
            }
        }
        self.flash_applied();
        self.toast(&gettext("Scene “{name}” applied").replace("{name}", name));
    }

    fn save_scene_dialog(&self) {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Save Scene"))
            .body(gettext(
                "The current lighting of every device will be saved under this name.",
            ))
            .default_response("save")
            .close_response("cancel")
            .build();
        let entry = gtk::Entry::builder()
            .placeholder_text(gettext("Scene name"))
            .activates_default(true)
            .build();
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", &gettext("_Cancel")), ("save", &gettext("_Save"))]);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                #[weak]
                entry,
                move |_, response| {
                    let name = entry.text().trim().to_string();
                    if response == "save" && !name.is_empty() {
                        let states = win.devices().iter().map(|d| (d.id(), d.state())).collect();
                        if let Some(store) = win.imp().scenes.borrow_mut().as_mut() {
                            store.add(Scene {
                                name: name.clone(),
                                states,
                            });
                        }
                        win.rebuild_scenes();
                        win.toast(&gettext("Scene “{name}” saved").replace("{name}", &name));
                    }
                }
            ),
        );
        dialog.present(Some(self));
    }

    fn rename_scene_dialog(&self, old: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Rename Scene"))
            .default_response("rename")
            .close_response("cancel")
            .build();
        let entry = gtk::Entry::builder()
            .text(old)
            .activates_default(true)
            .build();
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("rename", &gettext("_Rename")),
        ]);
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        let old = old.to_string();
        dialog.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                #[weak]
                entry,
                move |_, response| {
                    let name = entry.text().trim().to_string();
                    if response == "rename" && !name.is_empty() {
                        if let Some(store) = win.imp().scenes.borrow_mut().as_mut() {
                            store.rename(&old, &name);
                        }
                        win.rebuild_scenes();
                    }
                }
            ),
        );
        dialog.present(Some(self));
    }

    fn delete_scene(&self, name: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Delete Scene “{name}”?").replace("{name}", name))
            .body(gettext(
                "The scene will be removed. Your lights are not changed.",
            ))
            .default_response("cancel")
            .close_response("cancel")
            .build();
        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("delete", &gettext("_Delete")),
        ]);
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        let name = name.to_string();
        dialog.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                move |_, response| {
                    if response == "delete" {
                        if let Some(store) = win.imp().scenes.borrow_mut().as_mut() {
                            store.remove(&name);
                        }
                        win.rebuild_scenes();
                    }
                }
            ),
        );
        dialog.present(Some(self));
    }

    // ── Feedback ─────────────────────────────────────────────────────────

    /// The quiet "Applied" mark: fades in at once, fades out 1.2 s later.
    /// Both fades are CSS transitions, so they follow the system's
    /// animation setting.
    fn flash_applied(&self) {
        let imp = self.imp();
        imp.applied_box.add_css_class("shown");
        if let Some(id) = imp.applied_source.borrow_mut().take() {
            id.remove();
        }
        let id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(1200),
            glib::clone!(
                #[weak(rename_to = win)]
                self,
                move || {
                    win.imp().applied_source.borrow_mut().take();
                    win.imp().applied_box.remove_css_class("shown");
                }
            ),
        );
        imp.applied_source.replace(Some(id));
    }

    fn toast(&self, text: &str) {
        self.imp()
            .toast_overlay
            .add_toast(adw::Toast::builder().title(text).timeout(3).build());
    }

    fn report(&self, e: &ClientError) {
        tracing::warn!("{e}");
        let text = match e {
            ClientError::Unavailable(_) => {
                self.imp().view_stack.set_visible_child_name("disconnected");
                gettext("The lighting service is not running")
            }
            ClientError::Denied(_) => {
                gettext("Not allowed to change the lighting from this session")
            }
            ClientError::Failed(m) => gettext("Could not apply: {reason}").replace("{reason}", m),
        };
        self.toast(&text);
    }
}

#[cfg(debug_assertions)]
impl RGBeastWindow {
    fn debug_screenshot(&self, path: &std::path::Path) {
        let paintable = gtk::WidgetPaintable::new(Some(self));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, self.width() as f64, self.height() as f64);
        let (Some(node), Some(renderer)) = (snapshot.to_node(), self.renderer()) else {
            tracing::warn!("screenshot: nothing to render");
            return;
        };
        let texture = renderer.render_texture(&node, None);
        match texture.save_to_png(path) {
            Ok(()) => tracing::info!("screenshot saved to {}", path.display()),
            Err(e) => tracing::warn!("screenshot failed: {e}"),
        }
    }
}

fn short_name(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.len() <= 3 {
        name.to_string()
    } else {
        words[..3].join(" ")
    }
}
