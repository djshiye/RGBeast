//! The editor for one device: preview, effect chips, colour wheel, motion
//! controls and zones. Every change is applied live (debounced) through the
//! `apply` signal; the window talks to the daemon and feeds the confirmed
//! state back with `set_confirmed_state`.

use std::cell::{Cell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{CompositeTemplate, glib};
use rgbeast_core::{DeviceInfo, DeviceKind, DeviceState, Rgb, model::ColorMode};

use crate::{
    i18n::gettext,
    model::{Device, preview_colors},
    widgets::{ColorStrip, ColorWheel, LedPreview, ZoneShape, led_preview::ZoneLayout},
};

const PALETTE: [Rgb; 12] = [
    Rgb::new(0xF6, 0x61, 0x51),
    Rgb::new(0xFF, 0xA3, 0x48),
    Rgb::new(0xF8, 0xE4, 0x5C),
    Rgb::new(0x57, 0xE3, 0x89),
    Rgb::new(0x33, 0xD1, 0xC4),
    Rgb::new(0x62, 0xA0, 0xEA),
    Rgb::new(0x35, 0x84, 0xE4),
    Rgb::new(0x91, 0x41, 0xAC),
    Rgb::new(0xDC, 0x8A, 0xDD),
    Rgb::new(0xFF, 0x7B, 0xAC),
    Rgb::new(0xFF, 0xE4, 0xC4),
    Rgb::new(0xFF, 0xFF, 0xFF),
];

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/io/github/djshiye/RGBeast/ui/device_page.ui")]
    pub struct DevicePage {
        #[template_child]
        pub preview: TemplateChild<LedPreview>,
        #[template_child]
        pub paint_bar: TemplateChild<gtk::Box>,
        #[template_child]
        pub fill_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub mode_section: TemplateChild<gtk::Box>,
        #[template_child]
        pub mode_box: TemplateChild<adw::WrapBox>,
        #[template_child]
        pub color_section: TemplateChild<gtk::Box>,
        #[template_child]
        pub slot_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub add_color: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove_color: TemplateChild<gtk::Button>,
        #[template_child]
        pub wheel: TemplateChild<ColorWheel>,
        #[template_child]
        pub palette_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub hero_color: TemplateChild<ColorStrip>,
        #[template_child]
        pub hex_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub brightness_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub brightness_value: TemplateChild<gtk::Label>,
        #[template_child]
        pub brightness_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub motion_section: TemplateChild<gtk::Box>,
        #[template_child]
        pub speed_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub speed_value: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub direction_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub dir_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub random_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub random_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub zones_section: TemplateChild<gtk::Box>,
        #[template_child]
        pub zones_list: TemplateChild<gtk::ListBox>,

        pub device: RefCell<Option<Device>>,
        pub info: RefCell<DeviceInfo>,
        pub state: RefCell<DeviceState>,
        pub group_preview: RefCell<Vec<ZoneLayout>>,
        pub mode_buttons: RefCell<Vec<(String, gtk::ToggleButton)>>,
        pub slot_buttons: RefCell<Vec<gtk::ToggleButton>>,
        pub selected_slot: Cell<usize>,
        pub syncing: Cell<bool>,
        pub painted: Cell<bool>,
        pub apply_source: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DevicePage {
        const NAME: &'static str = "RGBeastDevicePage";
        type Type = super::DevicePage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            LedPreview::ensure_type();
            ColorWheel::ensure_type();
            ColorStrip::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DevicePage {
        fn signals() -> &'static [glib::subclass::Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("apply").build(),
                    glib::subclass::Signal::builder("resize-zone")
                        .param_types([String::static_type(), u32::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup();
        }
    }
    impl WidgetImpl for DevicePage {}
    impl BinImpl for DevicePage {}
}

glib::wrapper! {
    pub struct DevicePage(ObjectSubclass<imp::DevicePage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for DevicePage {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DevicePage {
    pub fn device(&self) -> Option<Device> {
        self.imp().device.borrow().clone()
    }

    /// The state as currently edited.
    pub fn state(&self) -> DeviceState {
        self.imp().state.borrow().clone()
    }

    pub fn connect_apply<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("apply", false, move |args| {
            let obj = args[0].get::<Self>().expect("page");
            f(&obj);
            None
        })
    }

    pub fn connect_resize_zone<F: Fn(&Self, String, u32) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("resize-zone", false, move |args| {
            let obj = args[0].get::<Self>().expect("page");
            let zone = args[1].get::<String>().expect("zone");
            let leds = args[2].get::<u32>().expect("leds");
            f(&obj, zone, leds);
            None
        })
    }

    pub fn set_animate(&self, on: bool) {
        self.imp().preview.set_animate(on);
    }

    /// Show a device. For the group, `group_preview` carries every real
    /// device's zones so the preview shows the whole machine.
    pub fn set_device(&self, device: &Device, group_preview: Vec<ZoneLayout>) {
        let imp = self.imp();
        imp.device.replace(Some(device.clone()));
        imp.info.replace(device.info());
        imp.state.replace(device.state());
        imp.group_preview.replace(group_preview);
        imp.painted.set(false);
        imp.selected_slot.set(0);
        self.cancel_apply();
        self.build_mode_chips();
        self.build_zones();
        self.sync_controls();
        self.refresh_preview();
    }

    /// The daemon confirmed (and normalised) a state. Only adopt it when no
    /// edit is in flight, so a drag never jumps back.
    pub fn set_confirmed_state(&self, state: DeviceState) {
        if self.imp().apply_source.borrow().is_some() {
            return;
        }
        self.imp().state.replace(state);
        self.sync_controls();
        self.refresh_preview();
    }

    /// Zone sizes changed on the daemon side (after a resize).
    pub fn set_info(&self, info: DeviceInfo) {
        self.imp().info.replace(info);
        self.build_zones();
        self.refresh_preview();
    }

    fn setup(&self) {
        let imp = self.imp();

        // Palette swatches.
        for c in PALETTE {
            let b = gtk::Button::new();
            b.add_css_class("swatch");
            let strip = ColorStrip::default();
            strip.set_colors(&[c]);
            strip.set_size_request(28, 28);
            b.set_child(Some(&strip));
            b.set_halign(gtk::Align::Center);
            b.set_tooltip_text(Some(&c.hex()));
            b.update_property(&[gtk::accessible::Property::Label(&c.hex())]);
            b.connect_clicked(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |_| page.pick_color(c, true)
            ));
            imp.palette_box.append(&b);
        }

        imp.wheel.connect_changed(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |w| page.pick_color(w.color(), false)
        ));

        imp.hex_entry.connect_activate(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |e| page.hex_entered(e)
        ));
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |_| page.hex_entered(&page.imp().hex_entry)
        ));
        imp.hex_entry.add_controller(focus);

        imp.brightness_scale.connect_value_changed(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |s| {
                let v = s.value().round() as u32;
                page.imp().brightness_value.set_label(&format!("{v}%"));
                if !page.imp().syncing.get() {
                    page.imp().state.borrow_mut().brightness = v;
                    page.refresh_preview();
                    page.schedule_apply();
                }
            }
        ));
        imp.speed_scale.connect_value_changed(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |s| {
                let v = s.value().round() as u32;
                page.imp().speed_value.set_label(&format!("{v}%"));
                if !page.imp().syncing.get() {
                    page.imp().state.borrow_mut().speed = v;
                    page.refresh_preview();
                    page.schedule_apply();
                }
            }
        ));
        imp.dir_group.connect_active_notify(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |g| {
                let idx = g.active();
                if page.imp().syncing.get() || idx == gtk::INVALID_LIST_POSITION {
                    return;
                }
                let dirs = page
                    .imp()
                    .info
                    .borrow()
                    .mode(&page.imp().state.borrow().mode)
                    .map(|m| m.directions.clone())
                    .unwrap_or_default();
                if let Some(d) = dirs.get(idx as usize) {
                    page.imp().state.borrow_mut().direction = d.clone();
                    page.refresh_preview();
                    page.schedule_apply();
                }
            }
        ));
        imp.random_switch.connect_active_notify(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |s| {
                if !page.imp().syncing.get() {
                    page.imp().state.borrow_mut().random = s.is_active();
                    page.refresh_preview();
                    page.schedule_apply();
                }
            }
        ));
        imp.add_color.connect_clicked(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |_| page.add_slot()
        ));
        imp.remove_color.connect_clicked(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |_| page.remove_slot()
        ));
        imp.fill_button.connect_clicked(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |_| {
                page.imp().painted.set(false);
                let c = page.imp().wheel.color();
                page.pick_color(c, false);
            }
        ));
        imp.preview.connect_led_clicked(glib::clone!(
            #[weak(rename_to = page)]
            self,
            move |_, zone, led| page.paint_led(zone as usize, led as usize)
        ));
    }

    fn current_mode(&self) -> Option<rgbeast_core::ModeInfo> {
        let imp = self.imp();
        let mode = imp.state.borrow().mode.clone();
        imp.info.borrow().mode(&mode).cloned()
    }

    fn build_mode_chips(&self) {
        let imp = self.imp();
        while let Some(child) = imp.mode_box.first_child() {
            imp.mode_box.remove(&child);
        }
        let mut buttons = Vec::new();
        let mut group: Option<gtk::ToggleButton> = None;
        let info = imp.info.borrow().clone();
        for m in &info.modes {
            let b = gtk::ToggleButton::with_label(&m.name);
            b.add_css_class("chip");
            if let Some(g) = &group {
                b.set_group(Some(g));
            } else {
                group = Some(b.clone());
            }
            let id = m.id.clone();
            b.connect_toggled(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |b| {
                    if b.is_active() && !page.imp().syncing.get() {
                        page.select_mode(&id);
                    }
                }
            ));
            imp.mode_box.append(&b);
            buttons.push((m.id.clone(), b));
        }
        imp.mode_buttons.replace(buttons);
        imp.mode_section.set_visible(!info.modes.is_empty());
    }

    fn build_zones(&self) {
        let imp = self.imp();
        while let Some(child) = imp.zones_list.first_child() {
            imp.zones_list.remove(&child);
        }
        let info = imp.info.borrow().clone();
        imp.zones_section
            .set_visible(info.zones.len() > 1 || info.zones.iter().any(|z| z.is_sizable()));
        for z in &info.zones {
            let row = adw::ActionRow::builder().title(&z.name).build();
            if z.is_sizable() {
                let spin = gtk::SpinButton::with_range(z.leds_min as f64, z.leds_max as f64, 1.0);
                spin.set_value(z.leds as f64);
                spin.set_valign(gtk::Align::Center);
                spin.set_tooltip_text(Some(&gettext("Number of LEDs connected to this header")));
                let unit = gtk::Label::new(Some(&gettext("LEDs")));
                unit.add_css_class("dim-label");
                row.set_subtitle(&gettext("Set the number of LEDs on the strip or fan chain"));
                row.add_suffix(&spin);
                row.add_suffix(&unit);
                let zone_id = z.id.clone();
                let pending: std::rc::Rc<RefCell<Option<glib::SourceId>>> = Default::default();
                spin.connect_value_changed(glib::clone!(
                    #[weak(rename_to = page)]
                    self,
                    move |s| {
                        let leds = s.value().round() as u32;
                        if let Some(id) = pending.borrow_mut().take() {
                            id.remove();
                        }
                        let zone_id = zone_id.clone();
                        let pending2 = pending.clone();
                        let id = glib::timeout_add_local_once(
                            std::time::Duration::from_millis(400),
                            glib::clone!(
                                #[weak]
                                page,
                                move || {
                                    pending2.borrow_mut().take();
                                    page.emit_by_name::<()>("resize-zone", &[&zone_id, &leds]);
                                }
                            ),
                        );
                        pending.borrow_mut().replace(id);
                    }
                ));
            } else {
                row.set_subtitle(&gettext("{n} LEDs").replace("{n}", &z.leds.to_string()));
            }
            imp.zones_list.append(&row);
        }
    }

    /// Push the state into every control without triggering handlers.
    fn sync_controls(&self) {
        let imp = self.imp();
        imp.syncing.set(true);
        let state = imp.state.borrow().clone();
        let mode = self.current_mode();

        for (id, b) in imp.mode_buttons.borrow().iter() {
            b.set_active(*id == state.mode);
        }

        let (uses_colors, per_led, multi, has_brightness, has_speed, dirs, has_random) = match &mode
        {
            Some(m) => (
                m.color_mode != ColorMode::None,
                m.color_mode == ColorMode::PerLed,
                m.color_mode == ColorMode::ModeColors && m.colors_max > 1,
                m.has_brightness,
                m.has_speed,
                m.directions.clone(),
                m.has_random,
            ),
            None => (false, false, false, false, false, vec![], false),
        };
        imp.color_section.set_visible(uses_colors);
        imp.brightness_box.set_visible(has_brightness);
        imp.motion_section
            .set_visible(has_speed || !dirs.is_empty() || has_random);
        imp.speed_box.set_visible(has_speed);
        imp.direction_box.set_visible(!dirs.is_empty());
        imp.random_box.set_visible(has_random);
        imp.paint_bar
            .set_visible(per_led && !imp.device.borrow().as_ref().is_some_and(|d| d.is_group()));
        imp.preview.set_paint_enabled(per_led);

        imp.brightness_scale.set_value(state.brightness as f64);
        imp.brightness_value
            .set_label(&format!("{}%", state.brightness));
        imp.speed_scale.set_value(state.speed as f64);
        imp.speed_value.set_label(&format!("{}%", state.speed));
        imp.random_switch.set_active(state.random);
        if dirs.len() >= 2 {
            for (i, d) in dirs.iter().take(2).enumerate() {
                if let Some(t) = imp.dir_group.toggle(i as u32) {
                    t.set_label(Some(&direction_label(d)));
                }
            }
            imp.dir_group
                .set_active(if state.direction == dirs[1] { 1 } else { 0 });
        }

        // Colour slots.
        self.rebuild_slots(multi, &state, mode.as_ref());
        let current = self.current_color(&state);
        imp.wheel.set_color(current);
        imp.hero_color.set_colors(&[current]);
        imp.hex_entry.set_text(&current.hex());
        imp.syncing.set(false);
    }

    fn current_color(&self, state: &DeviceState) -> Rgb {
        let slot = self.imp().selected_slot.get();
        state
            .colors
            .get(slot)
            .or(state.colors.first())
            .copied()
            .unwrap_or_else(|| state.primary_color())
    }

    fn rebuild_slots(
        &self,
        multi: bool,
        state: &DeviceState,
        mode: Option<&rgbeast_core::ModeInfo>,
    ) {
        let imp = self.imp();
        while let Some(child) = imp.slot_box.first_child() {
            imp.slot_box.remove(&child);
        }
        imp.slot_buttons.borrow_mut().clear();
        let (min, max) = mode.map(|m| (m.colors_min, m.colors_max)).unwrap_or((1, 1));
        imp.add_color
            .set_visible(multi && (state.colors.len() as u32) < max);
        imp.remove_color
            .set_visible(multi && (state.colors.len() as u32) > min.max(1));
        if !multi {
            return;
        }
        let n = state.colors.len();
        let selected = imp.selected_slot.get().min(n.saturating_sub(1));
        imp.selected_slot.set(selected);
        let mut group: Option<gtk::ToggleButton> = None;
        let background_last = min >= 2 && max == 11;
        for (i, c) in state.colors.iter().enumerate() {
            let b = gtk::ToggleButton::new();
            b.add_css_class("swatch");
            b.add_css_class("color-slot");
            let strip = ColorStrip::default();
            strip.set_colors(&[*c]);
            strip.set_size_request(30, 30);
            b.set_child(Some(&strip));
            let label = if background_last && i == n - 1 {
                gettext("Background colour")
            } else {
                gettext("Colour {n}").replace("{n}", &(i + 1).to_string())
            };
            b.set_tooltip_text(Some(&label));
            b.update_property(&[gtk::accessible::Property::Label(&label)]);
            if let Some(g) = &group {
                b.set_group(Some(g));
            } else {
                group = Some(b.clone());
            }
            b.set_active(i == selected);
            b.connect_toggled(glib::clone!(
                #[weak(rename_to = page)]
                self,
                move |b| {
                    if b.is_active() && !page.imp().syncing.get() {
                        page.imp().selected_slot.set(i);
                        let c = page.current_color(&page.imp().state.borrow());
                        page.imp().syncing.set(true);
                        page.imp().wheel.set_color(c);
                        page.imp().hero_color.set_colors(&[c]);
                        page.imp().hex_entry.set_text(&c.hex());
                        page.imp().syncing.set(false);
                    }
                }
            ));
            imp.slot_box.append(&b);
            imp.slot_buttons.borrow_mut().push(b);
        }
    }

    fn select_mode(&self, id: &str) {
        let imp = self.imp();
        let info = imp.info.borrow().clone();
        let Some(mode) = info.mode(id).cloned() else {
            return;
        };
        {
            let mut st = imp.state.borrow_mut();
            let current = self.current_color(&st);
            st.mode = id.to_string();
            match mode.color_mode {
                ColorMode::None => st.colors.clear(),
                ColorMode::ModeColors => {
                    if st.colors.is_empty() {
                        st.colors.push(current);
                    }
                    st.colors.truncate(mode.colors_max.max(1) as usize);
                    while (st.colors.len() as u32) < mode.colors_min {
                        let next = PALETTE[st.colors.len() % PALETTE.len()];
                        st.colors.push(next);
                    }
                }
                ColorMode::PerLed => {
                    if st.colors.is_empty() {
                        st.colors.push(current);
                    }
                    st.colors.truncate(1);
                    if st.zones.is_empty() {
                        let c = st.colors[0];
                        st.zones = info
                            .zones
                            .iter()
                            .map(|z| rgbeast_core::ZoneState {
                                id: z.id.clone(),
                                colors: vec![c; z.leds as usize],
                            })
                            .collect();
                    }
                }
            }
            if !mode.directions.is_empty() && !mode.directions.contains(&st.direction) {
                st.direction = mode.directions[0].clone();
            }
        }
        imp.painted.set(false);
        imp.selected_slot.set(0);
        self.sync_controls();
        self.refresh_preview();
        self.schedule_apply();
    }

    /// A colour was chosen on the wheel, a swatch or the hex field.
    fn pick_color(&self, c: Rgb, from_swatch: bool) {
        let imp = self.imp();
        if imp.syncing.get() {
            return;
        }
        imp.syncing.set(true);
        if from_swatch {
            imp.wheel.set_color(c);
        }
        imp.hero_color.set_colors(&[c]);
        imp.hex_entry.set_text(&c.hex());
        imp.syncing.set(false);

        let per_led = self
            .current_mode()
            .map(|m| m.color_mode == ColorMode::PerLed)
            .unwrap_or(false);
        {
            let mut st = imp.state.borrow_mut();
            let slot = imp.selected_slot.get();
            if st.colors.is_empty() {
                st.colors.push(c);
            } else if slot < st.colors.len() {
                st.colors[slot] = c;
            } else {
                st.colors[0] = c;
            }
            if per_led && !imp.painted.get() {
                let info = imp.info.borrow();
                st.zones = info
                    .zones
                    .iter()
                    .map(|z| rgbeast_core::ZoneState {
                        id: z.id.clone(),
                        colors: vec![c; z.leds as usize],
                    })
                    .collect();
            }
        }
        if let Some(b) = imp.slot_buttons.borrow().get(imp.selected_slot.get())
            && let Some(strip) = b.child().and_downcast::<ColorStrip>()
        {
            strip.set_colors(&[c]);
        }
        self.refresh_preview();
        self.schedule_apply();
    }

    fn paint_led(&self, zone: usize, led: usize) {
        let imp = self.imp();
        if imp.device.borrow().as_ref().is_some_and(|d| d.is_group()) {
            return;
        }
        let c = imp.wheel.color();
        {
            let info = imp.info.borrow();
            let mut st = imp.state.borrow_mut();
            if st.zones.len() != info.zones.len() {
                let base = st.primary_color();
                st.zones = info
                    .zones
                    .iter()
                    .map(|z| rgbeast_core::ZoneState {
                        id: z.id.clone(),
                        colors: vec![base; z.leds as usize],
                    })
                    .collect();
            }
            if let Some(z) = st.zones.get_mut(zone) {
                let n = info.zones.get(zone).map(|z| z.leds as usize).unwrap_or(0);
                if z.colors.len() < n {
                    let last = z.colors.last().copied().unwrap_or(c);
                    z.colors.resize(n, last);
                }
                if let Some(slot) = z.colors.get_mut(led) {
                    *slot = c;
                }
            }
        }
        imp.painted.set(true);
        self.refresh_preview();
        self.schedule_apply();
    }

    fn hex_entered(&self, e: &gtk::Entry) {
        if self.imp().syncing.get() {
            return;
        }
        match Rgb::from_hex(&e.text()) {
            Some(c) => {
                e.remove_css_class("error");
                // Leaving the field with the same value is not an edit.
                let current = self.current_color(&self.imp().state.borrow());
                if c != current {
                    self.pick_color(c, true);
                }
            }
            None => e.add_css_class("error"),
        }
    }

    fn add_slot(&self) {
        let imp = self.imp();
        let max = self.current_mode().map(|m| m.colors_max).unwrap_or(1) as usize;
        {
            let mut st = imp.state.borrow_mut();
            if st.colors.len() >= max {
                return;
            }
            let next = PALETTE[st.colors.len() % PALETTE.len()];
            let insert_at = st.colors.len();
            st.colors.insert(insert_at, next);
            imp.selected_slot.set(insert_at);
        }
        self.sync_controls();
        self.refresh_preview();
        self.schedule_apply();
    }

    fn remove_slot(&self) {
        let imp = self.imp();
        let min = self
            .current_mode()
            .map(|m| m.colors_min)
            .unwrap_or(1)
            .max(1) as usize;
        {
            let mut st = imp.state.borrow_mut();
            if st.colors.len() <= min {
                return;
            }
            let i = imp.selected_slot.get().min(st.colors.len() - 1);
            st.colors.remove(i);
            imp.selected_slot.set(i.saturating_sub(1));
        }
        self.sync_controls();
        self.refresh_preview();
        self.schedule_apply();
    }

    pub fn refresh_preview(&self) {
        let imp = self.imp();
        let state = imp.state.borrow().clone();
        let info = imp.info.borrow().clone();
        let is_group = imp.device.borrow().as_ref().is_some_and(|d| d.is_group());
        let zones: Vec<ZoneLayout> = if is_group {
            imp.group_preview.borrow().clone()
        } else {
            preview_colors(&info, &state)
                .into_iter()
                .map(|(z, colors)| ZoneLayout {
                    name: z.name.clone(),
                    shape: zone_shape(&info, &z),
                    colors,
                    row_break: false,
                })
                .collect()
        };
        imp.preview.set_zones(zones);
        imp.preview.set_effect(&state);
    }

    fn schedule_apply(&self) {
        self.cancel_apply();
        let id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(70),
            glib::clone!(
                #[weak(rename_to = page)]
                self,
                move || {
                    page.imp().apply_source.borrow_mut().take();
                    page.emit_by_name::<()>("apply", &[]);
                }
            ),
        );
        self.imp().apply_source.replace(Some(id));
    }

    fn cancel_apply(&self) {
        if let Some(id) = self.imp().apply_source.borrow_mut().take() {
            id.remove();
        }
    }
}

fn direction_label(d: &str) -> String {
    match d {
        "forward" => gettext("Forward"),
        "reverse" => gettext("Reverse"),
        "up" => gettext("Up"),
        "down" => gettext("Down"),
        other => other.to_string(),
    }
}

/// Fans on addressable headers, sticks for memory, strips otherwise.
pub fn zone_shape(info: &DeviceInfo, z: &rgbeast_core::ZoneInfo) -> ZoneShape {
    if info.kind == DeviceKind::Dram {
        ZoneShape::Stick
    } else if z.id.starts_with("argb") && z.leds > 0 && z.leds.is_multiple_of(12) {
        ZoneShape::Fans
    } else {
        ZoneShape::Strip
    }
}
