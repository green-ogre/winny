#![allow(unused)]

pub mod camera;
#[cfg(feature = "egui")]
pub mod gui;
pub mod model;
pub mod particle;
// pub mod primitives;
pub mod render;
pub mod render_pipeline;
pub mod sprite;
#[cfg(feature = "text")]
pub mod text;
pub mod texture;
pub mod transform;

pub use crate::{gui::*, model::*, particle::*, render::*, sprite::*, texture::*, transform::*};

#[allow(unused)]
pub use crate::render_pipeline::{
    bind_group::*, material::*, pipeline::*, render_assets::*, shader::*, vertex::*,
    vertex_buffer::*,
};

#[cfg(feature = "text")]
pub use crate::text::*;

pub extern crate bytemuck;
pub extern crate cgmath;
#[cfg(feature = "egui")]
pub extern crate egui;
pub extern crate wgpu;
#[cfg(feature = "text")]
pub extern crate wgpu_text;
