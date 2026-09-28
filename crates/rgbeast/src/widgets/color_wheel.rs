//! A hue ring around a saturation/value square. Drag on either part; arrow
//! keys nudge the hue (left/right) and value (up/down). Emits `changed`
//! only for user interaction, so programmatic updates never echo back.

use std::cell::{Cell, RefCell};

use gtk::{gdk, glib, graphene, gsk, prelude::*, subclass::prelude::*};
use rgbeast_core::Rgb;

const RING_WIDTH: f32 = 22.0;
const GAP: f32 = 10.0;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ColorWheel {
        pub hue: Cell<f64>,
        pub saturation: Cell<f64>,
        pub value: Cell<f64>,
        pub ring_cache: RefCell<Option<(i32, i32, i32, gdk::Texture)>>,
        pub drag_target: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ColorWheel {
        const NAME: &'static str = "RGBeastColorWheel";
        type Type = super::ColorWheel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("colorwheel");
            klass.set_accessible_role(gtk::AccessibleRole::Slider);
        }
    }

    impl ObjectImpl for ColorWheel {
        fn signals() -> &'static [glib::subclass::Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![glib::subclass::Signal::builder("changed").build()])
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.saturation.set(1.0);
            self.value.set(1.0);
            let obj = self.obj();
            obj.set_focusable(true);
            obj.set_cursor_from_name(Some("crosshair"));

            let drag = gtk::GestureDrag::new();
            drag.set_button(gdk::BUTTON_PRIMARY);
            drag.connect_drag_begin(glib::clone!(
                #[weak]
                obj,
                move |_, x, y| {
                    obj.grab_focus();
                    obj.begin_drag(x as f32, y as f32);
                }
            ));
            drag.connect_drag_update(glib::clone!(
                #[weak]
                obj,
                move |g, dx, dy| {
                    if let Some((sx, sy)) = g.start_point() {
                        obj.update_drag((sx + dx) as f32, (sy + dy) as f32);
                    }
                }
            ));
            obj.add_controller(drag);

            let keys = gtk::EventControllerKey::new();
            keys.connect_key_pressed(glib::clone!(
                #[weak]
                obj,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, key, _, state| {
                    let big = state.contains(gdk::ModifierType::SHIFT_MASK);
                    let step = if big { 10.0 } else { 2.0 };
                    let handled = match key {
                        gdk::Key::Left => {
                            obj.nudge_hue(-step);
                            true
                        }
                        gdk::Key::Right => {
                            obj.nudge_hue(step);
                            true
                        }
                        gdk::Key::Up => {
                            obj.nudge_value(step / 100.0);
                            true
                        }
                        gdk::Key::Down => {
                            obj.nudge_value(-step / 100.0);
                            true
                        }
                        _ => false,
                    };
                    if handled {
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                }
            ));
            obj.add_controller(keys);
        }
    }

    impl WidgetImpl for ColorWheel {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (160, 220, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let w = obj.width() as f32;
            let h = obj.height() as f32;
            let size = w.min(h);
            let cx = w / 2.0;
            let cy = h / 2.0;
            let outer = size / 2.0 - 2.0;
            let inner = outer - RING_WIDTH;
            let scale = obj.scale_factor();

            // Hue ring, rasterised once per size.
            let tex = self.ring_texture(size as i32, scale, outer, inner);
            snapshot.append_texture(
                &tex,
                &graphene::Rect::new(cx - size / 2.0, cy - size / 2.0, size, size),
            );

            // Saturation/value square inside the ring.
            let half = (inner - GAP) / 2.0_f32.sqrt();
            let rect = graphene::Rect::new(cx - half, cy - half, half * 2.0, half * 2.0);
            let rounded = gsk::RoundedRect::from_rect(rect, 10.0);
            snapshot.push_rounded_clip(&rounded);
            let hue_color = crate::widgets::rgba(Rgb::from_hsv(self.hue.get(), 1.0, 1.0), 1.0);
            snapshot.append_color(&hue_color, &rect);
            snapshot.append_linear_gradient(
                &rect,
                &graphene::Point::new(rect.x(), rect.y()),
                &graphene::Point::new(rect.x() + rect.width(), rect.y()),
                &[
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(1.0, 1.0, 1.0, 1.0)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.0)),
                ],
            );
            snapshot.append_linear_gradient(
                &rect,
                &graphene::Point::new(rect.x(), rect.y()),
                &graphene::Point::new(rect.x(), rect.y() + rect.height()),
                &[
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                ],
            );
            snapshot.pop();
            snapshot.append_border(
                &rounded,
                &[1.0; 4],
                &[gdk::RGBA::new(0.0, 0.0, 0.0, 0.18); 4],
            );

            // Markers.
            let angle = (self.hue.get() as f32).to_radians();
            let r_mid = (outer + inner) / 2.0;
            let (mx, my) = (cx + r_mid * angle.cos(), cy - r_mid * angle.sin());
            Self::marker(snapshot, mx, my, 8.0, hue_color);
            let sx = rect.x() + self.saturation.get() as f32 * rect.width();
            let sy = rect.y() + (1.0 - self.value.get() as f32) * rect.height();
            Self::marker(
                snapshot,
                sx,
                sy,
                7.0,
                crate::widgets::rgba(obj.color(), 1.0),
            );

            if obj.has_focus() {
                let focus = gsk::RoundedRect::from_rect(
                    graphene::Rect::new(
                        cx - outer - 2.0,
                        cy - outer - 2.0,
                        outer * 2.0 + 4.0,
                        outer * 2.0 + 4.0,
                    ),
                    outer + 2.0,
                );
                let accent = obj.color_for_accent();
                snapshot.append_border(&focus, &[2.0; 4], &[accent; 4]);
            }
        }
    }

    impl ColorWheel {
        fn marker(snapshot: &gtk::Snapshot, x: f32, y: f32, r: f32, fill: gdk::RGBA) {
            let rect = graphene::Rect::new(x - r, y - r, r * 2.0, r * 2.0);
            let rr = gsk::RoundedRect::from_rect(rect, r);
            snapshot.push_rounded_clip(&rr);
            snapshot.append_color(&fill, &rect);
            snapshot.pop();
            snapshot.append_border(&rr, &[2.5; 4], &[gdk::RGBA::new(1.0, 1.0, 1.0, 1.0); 4]);
            let outer = gsk::RoundedRect::from_rect(
                graphene::Rect::new(x - r - 1.0, y - r - 1.0, r * 2.0 + 2.0, r * 2.0 + 2.0),
                r + 1.0,
            );
            snapshot.append_border(&outer, &[1.0; 4], &[gdk::RGBA::new(0.0, 0.0, 0.0, 0.35); 4]);
        }

        fn ring_texture(&self, size: i32, scale: i32, outer: f32, inner: f32) -> gdk::Texture {
            if let Some((s, sc, _, tex)) = &*self.ring_cache.borrow()
                && *s == size
                && *sc == scale
            {
                return tex.clone();
            }
            let px = (size * scale).max(1) as usize;
            let mut data = vec![0u8; px * px * 4];
            let c = px as f32 / 2.0;
            let outer_px = outer * scale as f32;
            let inner_px = inner * scale as f32;
            for y in 0..px {
                for x in 0..px {
                    let dx = x as f32 + 0.5 - c;
                    let dy = c - (y as f32 + 0.5);
                    let d = (dx * dx + dy * dy).sqrt();
                    // Smooth 1px edges.
                    let a_out = ((outer_px - d) + 0.5).clamp(0.0, 1.0);
                    let a_in = ((d - inner_px) + 0.5).clamp(0.0, 1.0);
                    let a = a_out * a_in;
                    if a <= 0.0 {
                        continue;
                    }
                    let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                    let rgb = Rgb::from_hsv(hue as f64, 1.0, 1.0);
                    let i = (y * px + x) * 4;
                    data[i] = rgb.r;
                    data[i + 1] = rgb.g;
                    data[i + 2] = rgb.b;
                    data[i + 3] = (a * 255.0) as u8;
                }
            }
            let bytes = glib::Bytes::from_owned(data);
            let tex: gdk::Texture = gdk::MemoryTexture::new(
                px as i32,
                px as i32,
                gdk::MemoryFormat::R8g8b8a8,
                &bytes,
                px * 4,
            )
            .upcast();
            self.ring_cache.replace(Some((size, scale, 0, tex.clone())));
            tex
        }
    }
}

glib::wrapper! {
    pub struct ColorWheel(ObjectSubclass<imp::ColorWheel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ColorWheel {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ColorWheel {
    pub fn color(&self) -> Rgb {
        let imp = self.imp();
        Rgb::from_hsv(imp.hue.get(), imp.saturation.get(), imp.value.get())
    }

    /// Programmatic update: no `changed` signal.
    pub fn set_color(&self, c: Rgb) {
        let (h, s, v) = c.to_hsv();
        let imp = self.imp();
        // Keep the hue when the colour is grey so the ring marker stays put.
        if s > 0.001 && v > 0.001 {
            imp.hue.set(h);
        }
        imp.saturation.set(s);
        imp.value.set(v);
        self.queue_draw();
    }

    pub fn hue(&self) -> f64 {
        self.imp().hue.get()
    }

    fn geometry(&self) -> (f32, f32, f32, f32, f32) {
        let w = self.width() as f32;
        let h = self.height() as f32;
        let size = w.min(h);
        let outer = size / 2.0 - 2.0;
        let inner = outer - RING_WIDTH;
        (
            w / 2.0,
            h / 2.0,
            outer,
            inner,
            (inner - GAP) / 2.0_f32.sqrt(),
        )
    }

    fn begin_drag(&self, x: f32, y: f32) {
        let (cx, cy, outer, _inner, half) = self.geometry();
        let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        let in_square = (x - cx).abs() <= half && (y - cy).abs() <= half;
        let target = if in_square {
            2
        } else if d <= outer + 6.0 {
            1
        } else {
            0
        };
        self.imp().drag_target.set(target);
        self.update_drag(x, y);
    }

    fn update_drag(&self, x: f32, y: f32) {
        let (cx, cy, _outer, _inner, half) = self.geometry();
        let imp = self.imp();
        match imp.drag_target.get() {
            1 => {
                let hue = (cy - y).atan2(x - cx).to_degrees().rem_euclid(360.0);
                imp.hue.set(hue as f64);
            }
            2 => {
                let s = ((x - (cx - half)) / (half * 2.0)).clamp(0.0, 1.0);
                let v = 1.0 - ((y - (cy - half)) / (half * 2.0)).clamp(0.0, 1.0);
                imp.saturation.set(s as f64);
                imp.value.set(v as f64);
            }
            _ => return,
        }
        self.queue_draw();
        self.emit_by_name::<()>("changed", &[]);
    }

    fn nudge_hue(&self, delta: f64) {
        let imp = self.imp();
        imp.hue.set((imp.hue.get() + delta).rem_euclid(360.0));
        self.queue_draw();
        self.emit_by_name::<()>("changed", &[]);
    }

    fn nudge_value(&self, delta: f64) {
        let imp = self.imp();
        imp.value.set((imp.value.get() + delta).clamp(0.0, 1.0));
        self.queue_draw();
        self.emit_by_name::<()>("changed", &[]);
    }

    pub fn connect_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |args| {
            let obj = args[0].get::<Self>().expect("wheel");
            f(&obj);
            None
        })
    }

    fn color_for_accent(&self) -> gdk::RGBA {
        self.color_at_css("accent-bg-color")
            .unwrap_or(gdk::RGBA::new(0.2, 0.5, 0.9, 1.0))
    }

    fn color_at_css(&self, _name: &str) -> Option<gdk::RGBA> {
        // GTK 4.14 has no public API to read a CSS variable; use the style
        // context colour of the widget, which follows the accent through
        // the `accent` class set by the page.
        #[allow(deprecated)]
        Some(self.style_context().color())
    }
}
