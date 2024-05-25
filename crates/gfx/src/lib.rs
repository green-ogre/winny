use std::{io, path::PathBuf};

use ecs::{WinnyBundle, WinnyComponent, WinnyResource};

use logger::info;
use winny_math::{Matrix2x2f, Vec2f};

use self::texture::Texture;

pub mod bitmap;
pub mod camera;
pub mod gui;
pub mod image_decoder;
pub mod renderer;
pub mod sprite;
pub mod texture;

pub use renderer::*;

pub extern crate egui;

// pub const NUM_INSTANCES_PER_ROW: u32 = 10;
// pub const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
//     NUM_INSTANCES_PER_ROW as f32 * 0.5,
//     0.0,
//     NUM_INSTANCES_PER_ROW as f32 * 0.5,
// );

#[derive(Debug, WinnyResource)]
pub struct DeltaT(pub f64);

#[derive(Debug, WinnyComponent)]
pub struct Transform2D {
    t: Vec2f,
}

impl Transform2D {
    pub fn zero() -> Self {
        Self { t: Vec2f::zero() }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            t: Vec2f::new(x, y),
        }
    }

    pub fn as_matrix(&self) -> [f32; 2] {
        self.t.as_matrix()
    }
}

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub async fn load_binary(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let data = std::fs::read(path.as_os_str())?;

    Ok(data)
}

pub async fn load_string(path: &str) -> Result<String, io::Error> {
    let mut new_path = PathBuf::new();
    new_path.push("res/");
    new_path.push(path);
    let txt = std::fs::read_to_string(new_path.as_os_str())?;

    Ok(txt)
}

pub async fn load_texture(
    file_name: PathBuf,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Texture, ()> {
    info!("Loading texture: {:?}", file_name);
    let data = load_binary(file_name).await.map_err(|_| ())?;
    Texture::from_image(&data, device, queue)
}
