//! A title that uses the available space without making long names enlarge the window.
use gtk::{glib, pango, prelude::*, subclass::prelude::*};
mod imp {
    use super::*;
    use std::cell::{Cell, OnceCell};
    #[derive(Default)]
    pub struct FocusTitle {
        pub label: OnceCell<gtk::Label>,
        pub size: Cell<i32>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for FocusTitle {
        const NAME: &'static str = "BosuFocusTitle";
        type Type = super::FocusTitle;
        type ParentType = gtk::Widget;
    }
    impl ObjectImpl for FocusTitle {
        fn constructed(&self) {
            self.parent_constructed();
            let label = gtk::Label::builder()
                .wrap(true)
                .wrap_mode(pango::WrapMode::WordChar)
                .justify(gtk::Justification::Center)
                .xalign(0.5)
                .yalign(0.5)
                .build();
            label.set_parent(&*self.obj());
            self.label.set(label).unwrap();
        }
        fn dispose(&self) {
            if let Some(label) = self.label.get() {
                label.unparent();
            }
        }
    }
    impl WidgetImpl for FocusTitle {
        fn measure(&self, orientation: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                (180, 480, -1, -1)
            } else {
                (100, 360, -1, -1)
            }
        }
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let label = self.label.get().unwrap();
            let layout = label.create_pango_layout(Some(&label.text()));
            layout.set_width((width.max(1) - 8).max(1) * pango::SCALE);
            layout.set_wrap(pango::WrapMode::WordChar);
            let mut font = label.pango_context().font_description().unwrap_or_default();
            font.set_weight(pango::Weight::Medium);
            let mut lo = 1;
            let mut hi = (width.min(height).max(1) as f64 * 0.22).min(112.0) as i32;
            while lo < hi {
                let mid = (lo + hi + 1) / 2;
                font.set_absolute_size((mid * pango::SCALE) as f64);
                layout.set_font_description(Some(&font));
                let (w, h) = layout.pixel_size();
                if h <= (height - 8).max(1) && w <= width.max(1) {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            if self.size.replace(lo) != lo {
                let attributes = pango::AttrList::new();
                attributes.insert(pango::AttrSize::new_size_absolute(lo * pango::SCALE));
                attributes.insert(pango::AttrInt::new_weight(pango::Weight::Medium));
                label.set_attributes(Some(&attributes));
            }
            label.allocate(width, height, baseline, None);
        }
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.obj()
                .snapshot_child(self.label.get().unwrap(), snapshot);
        }
    }
}
glib::wrapper! {
    pub struct FocusTitle(ObjectSubclass<imp::FocusTitle>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl FocusTitle {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
    pub fn set_text(&self, text: &str) {
        let label = self.imp().label.get().unwrap();
        if label.text() != text {
            label.set_text(text);
            self.queue_allocate();
        }
    }
}
