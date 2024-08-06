use crate::Entity;
use cgmath::Quaternion;
use std::{ops::Range, path::PathBuf, ptr::NonNull};

pub trait ComponentEgui {
    fn display_component(&self, component: NonNull<u8>, ui: &mut egui::Ui);
}

pub trait Widget {
    fn display(&mut self, ui: &mut egui::Ui);
}

macro_rules! impl_widget {
    ($t:ident) => {
        impl Widget for $t {
            fn display(&mut self, ui: &mut egui::Ui) {
                ui.add(egui::DragValue::new(self).speed(0.1));
            }
        }
    };
}

impl_widget!(usize);
impl_widget!(u64);
impl_widget!(u32);
impl_widget!(u16);
impl_widget!(u8);

impl_widget!(isize);
impl_widget!(i64);
impl_widget!(i32);
impl_widget!(i16);
impl_widget!(i8);

impl_widget!(f64);
impl_widget!(f32);

impl<T: Widget> Widget for Option<T> {
    fn display(&mut self, ui: &mut egui::Ui) {
        match self {
            Some(v) => v.display(ui),
            None => {
                ui.label("None");
            }
        }
    }
}

impl<T: Widget> Widget for Vec<T> {
    fn display(&mut self, ui: &mut egui::Ui) {
        if self.len() > 5 {
            for element in self[..5].iter_mut() {
                element.display(ui);
            }
        } else {
            for element in self.iter_mut() {
                element.display(ui);
            }
        }
    }
}

impl Widget for String {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.label(self.as_str());
    }
}

impl Widget for bool {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(self, "NA");
    }
}

impl Widget for Quaternion<f32> {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.label("quaternion");
    }
}

impl<T: Widget> Widget for Range<T> {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.label("RANGE");
    }
}

impl Widget for Entity {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Entity(generation: {}, index: {})",
            self.generation(),
            self.index(),
        ));
    }
}

impl Widget for PathBuf {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("{:?}", self).as_str());
    }
}
