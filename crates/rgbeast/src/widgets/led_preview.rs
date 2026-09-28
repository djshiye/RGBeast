//! The hero of the editor: every LED of a device drawn as a glowing dot on a
//! dark card. Fans on addressable headers are rings of 12, memory sticks are
//! vertical bars, everything else is a strip. The selected effect plays as a
//! software approximation, at most 30 fps, never with reduced motion.

use std::cell::{Cell, RefCell};

use gtk::{gdk, glib, graphene, gsk, pango, prelude::*, subclass::prelude::*};
use rgbeast_core::{DeviceState, Rgb};

use crate::i18n::gettext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneShape {
    /// Rings of 12 LEDs (fans).
    Fans,
    /// A vertical bar (memory stick).
    Stick,
    /// A horizontal run of dots.
    Strip,
}

#[derive(Clone, Debug)]
pub struct ZoneLayout {
    pub name: String,
    pub shape: ZoneShape,
    pub colors: Vec<Rgb>,
    /// Start a new row before this zone (used to keep each device's zones together).
    pub row_break: bool,
}

/// One drawn LED: position, radius, and its zone/led indices.
#[derive(Clone, Copy, Debug)]
pub struct Dot {
    x: f32,
    y: f32,
    r: f32,
    zone: usize,
    led: usize,
    /// 0..1 position along the zone, for travelling effects.
    t: f32,
}

#[derive(Clone, Debug)]
pub struct Frame {
    /// Faint outlines drawn under the dots: (x, y, w, h, radius, is_ring).
    frames: Vec<(f32, f32, f32, f32, f32, bool)>,
    labels: Vec<(f32, f32, f32, String)>,
    dots: Vec<Dot>,
    height: f32,
}

const PAD: f32 = 18.0;
const FAN_D: f32 = 74.0;
const STICK_W: f32 = 24.0;
const STICK_H: f32 = 140.0;
const DOT: f32 = 6.0;
const LABEL_H: f32 = 18.0;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct LedPreview {
        pub zones: RefCell<Vec<ZoneLayout>>,
        pub mode: RefCell<String>,
        pub speed: Cell<u32>,
        pub brightness: Cell<u32>,
        pub mode_colors: RefCell<Vec<Rgb>>,
        pub reverse: Cell<bool>,
        pub random: Cell<bool>,
        pub animate: Cell<bool>,
        pub tick: RefCell<Option<gtk::TickCallbackId>>,
        pub start: Cell<i64>,
        pub layout: RefCell<Option<(i32, Frame)>>,
        pub paint_enabled: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LedPreview {
        const NAME: &'static str = "RGBeastLedPreview";
        type Type = super::LedPreview;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("ledpreview");
            klass.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    impl ObjectImpl for LedPreview {
        fn signals() -> &'static [glib::subclass::Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("led-clicked")
                        .param_types([u32::static_type(), u32::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.animate.set(true);
            self.brightness.set(100);
            let obj = self.obj();
            obj.update_property(&[gtk::accessible::Property::Label(&gettext(
                "Lighting preview",
            ))]);
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak]
                obj,
                move |_, _, x, y| obj.clicked(x as f32, y as f32)
            ));
            obj.add_controller(click);
            obj.connect_map(|o| o.update_animation());
            obj.connect_unmap(|o| o.stop_animation());
        }

        fn dispose(&self) {
            self.obj().stop_animation();
        }
    }

    impl WidgetImpl for LedPreview {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => (240, 320, -1, -1),
                _ => {
                    let width = if for_size > 0 { for_size } else { 320 };
                    let h = self.obj().frame_for(width).height.ceil() as i32;
                    (h.max(160), h.max(160), -1, -1)
                }
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let width = obj.width();
            let frame = obj.frame_for(width);
            let t = obj.elapsed();
            let zones = self.zones.borrow();

            let line = gdk::RGBA::new(1.0, 1.0, 1.0, 0.09);
            for (x, y, w, h, r, _ring) in &frame.frames {
                let rr = gsk::RoundedRect::from_rect(graphene::Rect::new(*x, *y, *w, *h), *r);
                snapshot.append_border(&rr, &[1.0; 4], &[line; 4]);
            }

            // Glow pass, then core pass, so glows never sit on top of cores.
            for d in &frame.dots {
                let c = obj.color_at(&zones, d, t);
                if c.is_black() {
                    continue;
                }
                let gr = d.r * 3.2;
                let bounds = graphene::Rect::new(d.x - gr, d.y - gr, gr * 2.0, gr * 2.0);
                snapshot.append_radial_gradient(
                    &bounds,
                    &graphene::Point::new(d.x, d.y),
                    gr,
                    gr,
                    0.0,
                    1.0,
                    &[
                        gsk::ColorStop::new(0.0, crate::widgets::rgba(c, 0.55)),
                        gsk::ColorStop::new(0.45, crate::widgets::rgba(c, 0.18)),
                        gsk::ColorStop::new(1.0, crate::widgets::rgba(c, 0.0)),
                    ],
                );
            }
            for d in &frame.dots {
                let c = obj.color_at(&zones, d, t);
                let rect = graphene::Rect::new(d.x - d.r, d.y - d.r, d.r * 2.0, d.r * 2.0);
                let rr = gsk::RoundedRect::from_rect(rect, d.r);
                snapshot.push_rounded_clip(&rr);
                if c.is_black() {
                    snapshot.append_color(&gdk::RGBA::new(1.0, 1.0, 1.0, 0.07), &rect);
                } else {
                    snapshot.append_color(&crate::widgets::rgba(c, 1.0), &rect);
                    // A small specular so bright colours still read as a lens.
                    let hr = d.r * 0.45;
                    snapshot.append_radial_gradient(
                        &rect,
                        &graphene::Point::new(d.x - d.r * 0.3, d.y - d.r * 0.3),
                        hr,
                        hr,
                        0.0,
                        1.0,
                        &[
                            gsk::ColorStop::new(0.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.7)),
                            gsk::ColorStop::new(1.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.0)),
                        ],
                    );
                }
                snapshot.pop();
            }

            let label_color = gdk::RGBA::new(1.0, 1.0, 1.0, 0.55);
            for (x, y, w, text) in &frame.labels {
                let layout = obj.create_pango_layout(Some(text));
                layout.set_width((*w * pango::SCALE as f32) as i32);
                layout.set_alignment(pango::Alignment::Center);
                layout.set_ellipsize(pango::EllipsizeMode::End);
                let mut attrs = pango::AttrList::new();
                attrs.insert(pango::AttrSize::new(9 * pango::SCALE));
                let _ = &mut attrs;
                layout.set_attributes(Some(&attrs));
                snapshot.save();
                snapshot.translate(&graphene::Point::new(*x, *y));
                snapshot.append_layout(&layout, &label_color);
                snapshot.restore();
            }
        }
    }
}

glib::wrapper! {
    pub struct LedPreview(ObjectSubclass<imp::LedPreview>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LedPreview {
    fn default() -> Self {
        glib::Object::new()
    }
}

fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

impl LedPreview {
    pub fn set_zones(&self, zones: Vec<ZoneLayout>) {
        self.imp().zones.replace(zones);
        self.imp().layout.replace(None);
        self.queue_resize();
        self.queue_draw();
    }

    /// Update colours in place (same zone layout), for cheap live edits.
    pub fn set_zone_colors(&self, zone: usize, colors: Vec<Rgb>) {
        if let Some(z) = self.imp().zones.borrow_mut().get_mut(zone) {
            z.colors = colors;
        }
        self.queue_draw();
    }

    pub fn set_effect(&self, state: &DeviceState) {
        let imp = self.imp();
        imp.mode.replace(state.mode.clone());
        imp.speed.set(state.speed);
        imp.brightness.set(state.brightness);
        imp.mode_colors.replace(state.colors.clone());
        imp.reverse
            .set(matches!(state.direction.as_str(), "reverse" | "down"));
        imp.random.set(state.random);
        self.update_animation();
        self.queue_draw();
    }

    pub fn set_animate(&self, on: bool) {
        self.imp().animate.set(on);
        self.update_animation();
    }

    pub fn set_paint_enabled(&self, on: bool) {
        self.imp().paint_enabled.set(on);
        self.set_cursor_from_name(if on { Some("crosshair") } else { None });
    }

    pub fn connect_led_clicked<F: Fn(&Self, u32, u32) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("led-clicked", false, move |args| {
            let obj = args[0].get::<Self>().expect("preview");
            let z = args[1].get::<u32>().expect("zone");
            let l = args[2].get::<u32>().expect("led");
            f(&obj, z, l);
            None
        })
    }

    fn is_animated_mode(mode: &str) -> bool {
        !matches!(mode, "static" | "direct" | "off" | "")
    }

    fn animations_allowed(&self) -> bool {
        self.imp().animate.get()
            && gtk::Settings::default()
                .map(|s| s.is_gtk_enable_animations())
                .unwrap_or(true)
    }

    fn update_animation(&self) {
        let imp = self.imp();
        let want = self.is_mapped()
            && self.animations_allowed()
            && Self::is_animated_mode(&imp.mode.borrow());
        if want && imp.tick.borrow().is_none() {
            imp.start
                .set(self.frame_clock().map(|c| c.frame_time()).unwrap_or(0));
            let last = std::rc::Rc::new(Cell::new(0i64));
            let id = self.add_tick_callback(move |w, clock| {
                // Cap at ~30 fps: half the frames on a 60 Hz display, a quarter at 120 Hz.
                let now = clock.frame_time();
                if now - last.get() >= 33_000 {
                    last.set(now);
                    w.queue_draw();
                }
                glib::ControlFlow::Continue
            });
            imp.tick.replace(Some(id));
        } else if !want {
            self.stop_animation();
        }
    }

    fn stop_animation(&self) {
        if let Some(id) = self.imp().tick.take() {
            id.remove();
        }
    }

    fn elapsed(&self) -> f32 {
        if self.imp().tick.borrow().is_none() {
            return 0.0;
        }
        let now = self.frame_clock().map(|c| c.frame_time()).unwrap_or(0);
        (now - self.imp().start.get()) as f32 / 1_000_000.0
    }

    /// The colour of one dot at time `t`, applying the software effect.
    fn color_at(&self, zones: &[ZoneLayout], d: &Dot, t: f32) -> Rgb {
        let imp = self.imp();
        let base = zones
            .get(d.zone)
            .and_then(|z| z.colors.get(d.led).copied())
            .unwrap_or(Rgb::BLACK);
        let brightness = imp.brightness.get();
        let mode = imp.mode.borrow();
        let speed = imp.speed.get() as f32 / 100.0;
        let rate = 0.35 + speed * 1.9;
        let dir = if imp.reverse.get() { -1.0 } else { 1.0 };
        let mode_colors = imp.mode_colors.borrow();
        let n_zone = zones
            .get(d.zone)
            .map(|z| z.colors.len())
            .unwrap_or(1)
            .max(1) as f32;
        let palette_color = |k: usize| -> Rgb {
            if mode_colors.is_empty() {
                base
            } else {
                mode_colors[k % mode_colors.len()]
            }
        };
        let out = match mode.as_str() {
            "off" => Rgb::BLACK,
            "static" | "direct" => base,
            "breathing" | "breath" => {
                let k = 0.5 + 0.5 * (t * rate * 2.0).sin();
                let c = if imp.random.get() {
                    Rgb::from_hsv(((t * rate * 40.0) % 360.0) as f64, 1.0, 1.0)
                } else {
                    base
                };
                c.scaled((k * 100.0) as u32)
            }
            "flashing" => {
                if (t * rate * 1.5) % 1.0 < 0.5 {
                    base
                } else {
                    Rgb::BLACK
                }
            }
            "rainbow" => {
                let hue = (d.t * 360.0 + dir * t * rate * 80.0).rem_euclid(360.0);
                Rgb::from_hsv(hue as f64, 1.0, 1.0)
            }
            "spectrum" | "spectrum-cycle" | "prism" | "twilight" => {
                let hue = (t * rate * 50.0
                    + if mode.as_str() == "prism" {
                        d.t * 60.0
                    } else {
                        0.0
                    })
                .rem_euclid(360.0);
                Rgb::from_hsv(hue as f64, 1.0, 1.0)
            }
            "random-flicker" => {
                let step = (t * 6.0 * rate) as u32;
                let k = hash01(d.zone as u32 * 131 + d.led as u32, step);
                let hue = (hash01(d.led as u32, step / 3) * 360.0) as f64;
                Rgb::from_hsv(hue, 1.0, (0.2 + 0.8 * k) as f64)
            }
            "flame" => {
                let step = (t * 10.0 * rate) as u32;
                let k = hash01(d.led as u32 + d.zone as u32 * 977, step);
                Rgb::from_hsv((10.0 + 30.0 * k) as f64, 1.0, (0.35 + 0.65 * k) as f64)
            }
            "dynamic" | "rhythm" => {
                let lap = (t * rate * 0.5) as usize;
                let phase = (t * rate * 0.5) % 1.0;
                let a = palette_color(lap);
                let b = palette_color(lap + 1);
                let mix = |x: u8, y: u8| ((x as f32) * (1.0 - phase) + (y as f32) * phase) as u8;
                Rgb::new(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b))
            }
            _ => {
                // Travelling highlight: chase, comet, slide, wind, rain, firework, voltage, fury, teleport, slither.
                let cycle = t * rate * 1.2;
                let lap = cycle as usize;
                let pos = (cycle % 1.0) * (n_zone + 3.0) - 1.5;
                let led_pos = if dir > 0.0 {
                    d.t * n_zone
                } else {
                    (1.0 - d.t) * n_zone
                };
                let dist = (led_pos - pos).abs();
                let tail = if matches!(mode.as_str(), "comet" | "chase-fade" | "wind" | "fury") {
                    4.0
                } else {
                    2.0
                };
                let k = (1.0 - dist / tail).clamp(0.0, 1.0);
                let c = if imp.random.get() {
                    Rgb::from_hsv((lap as f32 * 47.0 % 360.0) as f64, 1.0, 1.0)
                } else {
                    palette_color(lap)
                };
                let floor = if mode_colors.len() >= 2
                    && matches!(
                        mode.as_str(),
                        "slide" | "wind" | "voltage" | "fury" | "teleport" | "slither" | "rhythm"
                    ) {
                    mode_colors[mode_colors.len() - 1]
                } else {
                    Rgb::BLACK
                };
                if k <= 0.0 {
                    floor
                } else {
                    c.scaled((k * 100.0) as u32)
                }
            }
        };
        out.scaled(brightness)
    }

    fn clicked(&self, x: f32, y: f32) {
        if !self.imp().paint_enabled.get() {
            return;
        }
        let frame = self.frame_for(self.width());
        let hit = frame.dots.iter().min_by(|a, b| {
            let da = (a.x - x).powi(2) + (a.y - y).powi(2);
            let db = (b.x - x).powi(2) + (b.y - y).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(d) = hit
            && ((d.x - x).powi(2) + (d.y - y).powi(2)).sqrt() <= d.r * 2.5
        {
            self.emit_by_name::<()>("led-clicked", &[&(d.zone as u32), &(d.led as u32)]);
        }
    }

    /// Lay the zones out for a width, cached until the zones or width change.
    fn frame_for(&self, width: i32) -> Frame {
        if let Some((w, f)) = &*self.imp().layout.borrow()
            && *w == width
        {
            return f.clone();
        }
        let frame = Self::layout(&self.imp().zones.borrow(), width as f32);
        self.imp().layout.replace(Some((width, frame.clone())));
        frame
    }

    fn layout(zones: &[ZoneLayout], width: f32) -> Frame {
        let mut frame = Frame {
            frames: Vec::new(),
            labels: Vec::new(),
            dots: Vec::new(),
            height: 0.0,
        };
        let usable = (width - PAD * 2.0).max(120.0);
        // Items: (w, h, zone index, kind-specific index)
        struct Item {
            w: f32,
            h: f32,
            zone: usize,
            shape: ZoneShape,
            first_led: usize,
            count: usize,
        }
        let mut items: Vec<Item> = Vec::new();
        for (zi, z) in zones.iter().enumerate() {
            let n = z.colors.len();
            if n == 0 {
                continue;
            }
            match z.shape {
                ZoneShape::Fans => {
                    let fans = n / 12;
                    for f in 0..fans {
                        items.push(Item {
                            w: FAN_D + 12.0,
                            h: FAN_D + LABEL_H + 6.0,
                            zone: zi,
                            shape: ZoneShape::Fans,
                            first_led: f * 12,
                            count: 12,
                        });
                    }
                    let rest = n % 12;
                    if rest > 0 {
                        let w = (rest as f32 * (DOT * 2.0 + 8.0)).max(60.0);
                        items.push(Item {
                            w,
                            h: DOT * 2.0 + 16.0 + LABEL_H,
                            zone: zi,
                            shape: ZoneShape::Strip,
                            first_led: fans * 12,
                            count: rest,
                        });
                    }
                }
                ZoneShape::Stick => items.push(Item {
                    w: STICK_W + 56.0,
                    h: STICK_H + LABEL_H + 6.0,
                    zone: zi,
                    shape: ZoneShape::Stick,
                    first_led: 0,
                    count: n,
                }),
                ZoneShape::Strip => {
                    let per_row = ((usable - 24.0) / (DOT * 2.0 + 8.0)).floor().max(1.0) as usize;
                    let mut start = 0;
                    while start < n {
                        let count = (n - start).min(per_row);
                        let w = count as f32 * (DOT * 2.0 + 8.0) + 24.0;
                        items.push(Item {
                            w,
                            h: DOT * 2.0 + 16.0 + LABEL_H,
                            zone: zi,
                            shape: ZoneShape::Strip,
                            first_led: start,
                            count,
                        });
                        start += count;
                    }
                }
            }
        }
        // Flow layout, rows centred. Labels are added per zone and row,
        // centred under everything that zone occupies in the row.
        let mut y = PAD;
        let mut i = 0;
        while i < items.len() {
            let mut row_w = 0.0;
            let mut j = i;
            while j < items.len()
                && (row_w == 0.0 || row_w + items[j].w <= usable)
                && (j == i
                    || !(zones[items[j].zone].row_break && items[j].zone != items[j - 1].zone))
            {
                row_w += items[j].w;
                j += 1;
            }
            let row_h = items[i..j].iter().map(|it| it.h).fold(0.0, f32::max);
            let mut x = PAD + (usable - row_w) / 2.0;
            // (zone, x0, x1, label baseline y)
            let mut spans: Vec<(usize, f32, f32, f32)> = Vec::new();
            for it in &items[i..j] {
                let z = &zones[it.zone];
                let cx = x + it.w / 2.0;
                let label_y = match it.shape {
                    ZoneShape::Fans => {
                        let cy = y + FAN_D / 2.0 + 2.0;
                        let r = FAN_D / 2.0;
                        frame.frames.push((cx - r, cy - r, FAN_D, FAN_D, r, true));
                        let hub = 10.0;
                        frame
                            .frames
                            .push((cx - hub, cy - hub, hub * 2.0, hub * 2.0, hub, true));
                        for k in 0..it.count {
                            let a = -std::f32::consts::FRAC_PI_2
                                + k as f32 / it.count as f32 * std::f32::consts::TAU;
                            let rr = r - 11.0;
                            frame.dots.push(Dot {
                                x: cx + rr * a.cos(),
                                y: cy + rr * a.sin(),
                                r: DOT,
                                zone: it.zone,
                                led: it.first_led + k,
                                t: (it.first_led + k) as f32 / z.colors.len() as f32,
                            });
                        }
                        y + FAN_D + 6.0
                    }
                    ZoneShape::Stick => {
                        let top = y + 2.0;
                        frame
                            .frames
                            .push((cx - STICK_W / 2.0, top, STICK_W, STICK_H, 6.0, false));
                        let n = it.count.max(1);
                        for k in 0..it.count {
                            let ly = top + 10.0 + (STICK_H - 20.0) * (k as f32 + 0.5) / n as f32;
                            frame.dots.push(Dot {
                                x: cx,
                                y: ly,
                                r: DOT - 1.0,
                                zone: it.zone,
                                led: k,
                                t: 1.0 - k as f32 / n as f32,
                            });
                        }
                        top + STICK_H + 6.0
                    }
                    ZoneShape::Strip => {
                        let cy = y + DOT + 8.0;
                        let pitch = DOT * 2.0 + 8.0;
                        let x0 = cx - (it.count as f32 * pitch) / 2.0 + pitch / 2.0;
                        for k in 0..it.count {
                            let led = it.first_led + k;
                            frame.dots.push(Dot {
                                x: x0 + k as f32 * pitch,
                                y: cy,
                                r: DOT,
                                zone: it.zone,
                                led,
                                t: led as f32 / z.colors.len().max(1) as f32,
                            });
                        }
                        cy + DOT + 8.0
                    }
                };
                match spans.last_mut() {
                    Some(sp) if sp.0 == it.zone => {
                        sp.2 = x + it.w;
                        sp.3 = sp.3.max(label_y);
                    }
                    _ => spans.push((it.zone, x, x + it.w, label_y)),
                }
                x += it.w;
            }
            for (zone, x0, x1, ly) in spans {
                // Widen single narrow items a little so short names fit, without crossing neighbours.
                let pad = ((x1 - x0) * 0.15).min(14.0);
                frame
                    .labels
                    .push((x0 - pad, ly, x1 - x0 + pad * 2.0, zones[zone].name.clone()));
            }
            y += row_h + 10.0;
            i = j;
        }
        frame.height = y + PAD - 10.0;
        frame
    }
}
