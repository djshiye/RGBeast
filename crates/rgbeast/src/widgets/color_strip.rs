//! A thin pill showing a run of colours as a gradient. Used in the sidebar
//! so every device row previews what it is showing.

use std::cell::RefCell;

use gtk::{gdk, glib, graphene, gsk, prelude::*, subclass::prelude::*};
use rgbeast_core::Rgb;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ColorStrip {
        pub colors: RefCell<Vec<Rgb>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ColorStrip {
        const NAME: &'static str = "RGBeastColorStrip";
        type Type = super::ColorStrip;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("colorstrip");
        }
    }

    impl ObjectImpl for ColorStrip {}

    impl WidgetImpl for ColorStrip {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => (24, 24, -1, -1),
                _ => (5, 5, -1, -1),
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let w = obj.width() as f32;
            let h = obj.height() as f32;
            let rect = graphene::Rect::new(0.0, 0.0, w, h);
            let rr = gsk::RoundedRect::from_rect(rect, h / 2.0);
            let colors = self.colors.borrow();
            snapshot.push_rounded_clip(&rr);
            match colors.len() {
                0 => snapshot.append_color(&gdk::RGBA::new(0.5, 0.5, 0.5, 0.25), &rect),
                1 => snapshot.append_color(&crate::widgets::rgba(colors[0], 1.0), &rect),
                n => {
                    let stops: Vec<gsk::ColorStop> = colors
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            gsk::ColorStop::new(
                                i as f32 / (n - 1) as f32,
                                crate::widgets::rgba(*c, 1.0),
                            )
                        })
                        .collect();
                    snapshot.append_linear_gradient(
                        &rect,
                        &graphene::Point::new(0.0, 0.0),
                        &graphene::Point::new(w, 0.0),
                        &stops,
                    );
                }
            }
            snapshot.pop();
            snapshot.append_border(&rr, &[1.0; 4], &[gdk::RGBA::new(0.0, 0.0, 0.0, 0.12); 4]);
        }
    }
}

glib::wrapper! {
    pub struct ColorStrip(ObjectSubclass<imp::ColorStrip>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ColorStrip {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ColorStrip {
    /// Show up to 24 evenly sampled colours.
    pub fn set_colors(&self, colors: &[Rgb]) {
        let sampled: Vec<Rgb> = if colors.len() <= 24 {
            colors.to_vec()
        } else {
            (0..24).map(|i| colors[i * colors.len() / 24]).collect()
        };
        self.imp().colors.replace(sampled);
        self.queue_draw();
    }
}
