use bytemuck::Pod;
#[cfg(feature = "editor")]
use ecs::egui_widget::Widget;
use ecs::{WinnyComponent, WinnyResource};
use std::{ops::Deref, sync::Arc};
use wgpu::TextureFormat;

#[cfg(feature = "editor")]
pub trait DimensionsUnit: 'static + Copy + Send + Sync + Pod + Widget {}
#[cfg(feature = "editor")]
impl<T: 'static + Copy + Send + Sync + Pod + Widget> DimensionsUnit for T {}

#[cfg(not(feature = "editor"))]
pub trait DimensionsUnit: 'static + Copy + Send + Sync + Pod {}
#[cfg(not(feature = "editor"))]
impl<T: 'static + Copy + Send + Sync + Pod> DimensionsUnit for T {}

/// Described a width and height of unit T
#[repr(transparent)]
#[derive(WinnyComponent, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Dimensions<T: DimensionsUnit> {
    dimensions: [T; 2],
}

#[cfg(feature = "editor")]
impl<T: DimensionsUnit> Widget for Dimensions<T> {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ecs::egui::CollapsingHeader::new("Dimensions").show(ui, |ui| {
            self.dimensions[0].display(ui);
            self.dimensions[1].display(ui);
        });
    }
}

impl<T: DimensionsUnit> Dimensions<T> {
    pub fn new(width: T, height: T) -> Self {
        Self {
            dimensions: [width, height],
        }
    }

    pub fn width(&self) -> T {
        self.dimensions[0]
    }

    pub fn height(&self) -> T {
        self.dimensions[1]
    }
}

/// Wraps the [`wgpu::SurfaceConfiguration`].
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub dimensions: Dimensions<u32>,
    pub format: wgpu::TextureFormat,
}

impl RenderConfig {
    pub fn from_config(value: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            dimensions: Dimensions::new(value.width, value.height),
            format: value.format,
        }
    }

    pub fn width(&self) -> u32 {
        self.dimensions.width()
    }

    pub fn height(&self) -> u32 {
        self.dimensions.height()
    }

    pub fn widthf(&self) -> f32 {
        self.dimensions.width() as f32
    }

    pub fn heightf(&self) -> f32 {
        self.dimensions.height() as f32
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

/// Wraps the [`wgpu::Queue`].
#[derive(Debug, Clone)]
pub struct RenderQueue(Arc<wgpu::Queue>);

impl RenderQueue {
    pub fn new(queue: wgpu::Queue) -> Self {
        Self(Arc::new(queue))
    }
}

impl Deref for RenderQueue {
    type Target = wgpu::Queue;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Wraps the [`wgpu::Device`].
#[derive(Debug, Clone)]
pub struct RenderDevice(Arc<wgpu::Device>);

impl RenderDevice {
    pub fn new(device: wgpu::Device) -> Self {
        Self(Arc::new(device))
    }
}

impl Deref for RenderDevice {
    type Target = wgpu::Device;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Handle to the resources required for wgpu resource aquisition.
#[derive(WinnyResource, Debug, Clone)]
pub struct RenderContext {
    pub queue: RenderQueue,
    pub device: RenderDevice,
    pub config: RenderConfig,
}
