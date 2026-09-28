//! Sidebar card for one device (or the "All Devices" group).

use std::cell::RefCell;

use gtk::{CompositeTemplate, glib, prelude::*, subclass::prelude::*};

use crate::{model::Device, widgets::ColorStrip};

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/io/github/djshiye/RGBeast/ui/device_row.ui")]
    pub struct DeviceRow {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub name: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub strip: TemplateChild<ColorStrip>,
        pub device: RefCell<Option<Device>>,
        pub handler: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceRow {
        const NAME: &'static str = "RGBeastDeviceRow";
        type Type = super::DeviceRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            ColorStrip::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DeviceRow {
        fn dispose(&self) {
            self.obj().unbind();
        }
    }
    impl WidgetImpl for DeviceRow {}
    impl BoxImpl for DeviceRow {}
}

glib::wrapper! {
    pub struct DeviceRow(ObjectSubclass<imp::DeviceRow>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl DeviceRow {
    pub fn new(device: &Device) -> Self {
        let row: Self = glib::Object::new();
        row.bind(device);
        row
    }

    pub fn device(&self) -> Option<Device> {
        self.imp().device.borrow().clone()
    }

    fn bind(&self, device: &Device) {
        self.unbind();
        let id = device.connect_changed(glib::clone!(
            #[weak(rename_to = row)]
            self,
            move |d| row.refresh(d)
        ));
        self.imp().handler.replace(Some(id));
        self.imp().device.replace(Some(device.clone()));
        self.refresh(device);
    }

    fn unbind(&self) {
        if let (Some(d), Some(h)) = (self.imp().device.take(), self.imp().handler.take()) {
            d.disconnect(h);
        }
    }

    fn refresh(&self, d: &Device) {
        let imp = self.imp();
        imp.icon.set_icon_name(Some(d.icon_name()));
        imp.name.set_label(&d.info().name);
        imp.subtitle.set_label(&d.subtitle());
        if d.is_group() {
            imp.strip.set_visible(false);
        } else {
            imp.strip.set_visible(true);
            imp.strip.set_colors(&d.strip_colors());
        }
        self.update_property(&[gtk::accessible::Property::Label(&format!(
            "{}, {}",
            d.info().name,
            d.subtitle()
        ))]);
    }
}
