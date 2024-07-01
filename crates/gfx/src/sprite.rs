use app::plugins::Plugin;
use asset::{Asset, AssetApp, AssetLoader, Assets, Handle};
use ecs::{Commands, Entity, IntoSystemStorage, Query, Res, ResMut, WinnyResource, With};
use renderer::RenderContext;
use texture::Sprites;
use wgpu::util::DeviceExt;

use self::renderer::Renderer;

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RGBA {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn clear() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.0,
        }
    }

    pub fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

fn create_sprite_render_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::RenderPipeline {
    let sprite_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("bind group layout for sprite"),
        });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&sprite_bind_group_layout],
        push_constant_ranges: &[],
    });

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite_shader.wgsl").into()),
    };

    crate::renderer::create_render_pipeline(
        &device,
        &render_pipeline_layout,
        config.format,
        // Some(wgpu::TextureFormat::Depth32Float),
        &[SpriteVertex::desc(), SpriteInstance::desc()],
        shader,
    )
}

const VERTICES: u32 = 3;

#[derive(Debug, WinnyResource)]
pub struct SpriteRenderer {
    vertex_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
    num_sprites: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let render_pipeline = create_sprite_render_pipeline(device, config);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite vertex buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let sprite_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Buffer"),
            contents: bytemuck::cast_slice::<SpriteInstance, u8>(&[]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertex_buffer,
            sprite_buffer,
            num_sprites: 0,
            render_pipeline,
        }
    }
}

pub fn update_sprite_data(
    sprites: Query<(Sprite, Handle<SpriteData>), With<SpriteIsBinded>>,
    renderer: ResMut<Renderer>,
    mut sprite_renderer: ResMut<SpriteRenderer>,
) {
    let sprite_data = sprites
        .iter()
        .map(|(s, _)| s.to_raw(&renderer))
        .collect::<Vec<_>>();

    sprite_renderer.sprite_buffer =
        renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sprite Buffer"),
                contents: bytemuck::cast_slice(&sprite_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

    let vertex_data: Vec<_> = sprites
        .iter()
        .map(|(s, _)| s.to_vertices())
        .flatten()
        .collect();

    sprite_renderer.num_sprites = vertex_data.len() as u32 / VERTICES;

    sprite_renderer.vertex_buffer =
        renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite vertex buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
}

fn render_sprites(
    mut renderer: ResMut<Renderer>,
    mut context: ResMut<RenderContext>,
    sprite_renderer: Res<SpriteRenderer>,
    textures: Res<Sprites>,
) {
    let view = renderer.view();
    let mut render_pass = context
        .encoder()
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

    let mut offset = 0;
    for binding in textures.iter_bindings() {
        render_pass.set_pipeline(&sprite_renderer.render_pipeline);
        render_pass.set_bind_group(0, &binding.bind_group, &[]);
        render_pass.set_vertex_buffer(0, sprite_renderer.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, sprite_renderer.sprite_buffer.slice(..));
        render_pass.draw(
            offset * VERTICES..offset * VERTICES + VERTICES,
            offset..offset + 1,
        );
        offset += 1;
    }

    drop(render_pass);
    context.finish_encoder();
}

#[derive(WinnyComponent)]
pub struct SpriteIsBinded;

#[derive(Debug, Clone, WinnyBundle)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub handle: Handle<SpriteData>,
}

pub fn bind_new_sprite_bundles(
    sprites: Query<(Entity, Handle<SpriteData>), With<Sprite>>,
    assets: ResMut<Assets<SpriteData>>,
    mut textures: ResMut<Sprites>,
    renderer: ResMut<Renderer>,
    mut commands: Commands,
) {
    for (entity, handle) in sprites.iter_mut() {
        if !textures.contains_key(&handle.id()) {
            if let Some(asset) = assets.get(&handle) {
                let texture = Texture::from_bytes(
                    &asset.bytes,
                    asset.dimensions,
                    &renderer.device,
                    &renderer.queue,
                );
                let binding = SpriteBinding::from_texture(&texture, &renderer);
                textures.insert(&handle, texture, binding);
            }
        }

        commands.get_entity(entity).insert(SpriteIsBinded);
    }
}

#[derive(Debug, WinnyComponent, Clone, Copy)]
pub struct Sprite {
    pub scale: f32,
    pub rotation: f32,
    pub position: Vec2f,
    pub mask: RGBA,
    pub offset: Vec2f,
    pub v_flip: bool,
    pub z: f32,
}

impl Asset for Sprite {}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: 0.0,
            position: Vec2f::new(0.0, 0.0),
            mask: RGBA::clear(),
            offset: Vec2f::zero(),
            v_flip: false,
            z: 0.0,
        }
    }
}

impl Sprite {
    pub fn to_raw(&self, renderer: &Renderer) -> SpriteInstance {
        SpriteInstance {
            position: [
                self.position.x / renderer.virtual_size[0] as f32,
                self.position.y / renderer.virtual_size[0] as f32,
                self.z,
                0.0,
            ],
            mask: [self.mask.r, self.mask.g, self.mask.b, self.mask.a],
        }
    }

    pub fn to_vertices(&self) -> [SpriteVertex; 3] {
        let x = self.offset.x * self.scale;
        let y = self.offset.y * self.scale;

        if self.v_flip {
            [
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, -y), self.rotation),
                    Vec2f::zero(),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, 2.0 * self.scale - y), self.rotation),
                    Vec2f::new(0.0, 2.0),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(2.0 * self.scale - x, -y), self.rotation),
                    Vec2f::new(2.0, 0.0),
                ),
            ]
        } else {
            [
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, -y), self.rotation),
                    Vec2f::new(0.0, 1.0),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, 2.0 * self.scale - y), self.rotation),
                    Vec2f::new(0.0, -1.0),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(2.0 * self.scale - x, -y), self.rotation),
                    Vec2f::new(2.0, 1.0),
                ),
            ]
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 4],
    pub tex_coord: [f32; 2],
    pub _padding: [f32; 2],
}

impl SpriteVertex {
    pub fn new(position: Vec2f, tex_coord: Vec2f) -> Self {
        Self {
            position: [position.x, position.y, 0.0, 0.0],
            tex_coord: [tex_coord.x, tex_coord.y],
            _padding: [0.0, 0.0],
        }
    }
}

impl Vertex for SpriteVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct SpriteData {
    pub bytes: Vec<u8>,
    pub dimensions: (u32, u32),
}

impl Asset for SpriteData {}

impl SpriteData {
    // pub fn write_to_texture(&self, dimensions: (u32, u32), data: &[u8], renderer: &Renderer) {
    //     let size = wgpu::Extent3d {
    //         width: dimensions.0,
    //         height: dimensions.1,
    //         depth_or_array_layers: 1,
    //     };

    //     renderer.queue.write_texture(
    //         self.texture.tex.as_image_copy(),
    //         &data,
    //         wgpu::ImageDataLayout {
    //             offset: 0,
    //             bytes_per_row: Some(4 * dimensions.0),
    //             rows_per_image: Some(dimensions.1),
    //         },
    //         size,
    //     );
    //     renderer.queue.submit([]);
    // }
}

#[derive(Debug, WinnyComponent)]
pub struct SpriteBinding {
    pub bind_group: wgpu::BindGroup,
}

impl SpriteBinding {
    pub fn from_texture(texture: &Texture, renderer: &Renderer) -> Self {
        let layout = renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("bind group layout for sprite"),
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("bind group for sprite"),
            });

        Self { bind_group }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    position: [f32; 4],
    mask: [f32; 4],
}

impl Vertex for SpriteInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// TODO: this is coupled with the renderer right now, need to be pulled apart
pub struct SpriteAssetLoader;

impl AssetLoader for SpriteAssetLoader {
    type Asset = SpriteData;

    fn extensions(&self) -> Vec<String> {
        // TODO: feature flags
        vec!["png".into()]
    }

    fn load(
        reader: asset::reader::ByteReader<std::fs::File>,
        _ext: &str,
    ) -> Result<Self::Asset, ()> {
        let (bytes, dimensions) = png::to_bytes(reader)
            .map_err(|err| logger::error!("Could not load sprite: {:?}", err))?;
        Ok(SpriteData { bytes, dimensions })
    }
}

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        let loader = SpriteAssetLoader {};
        app.register_asset_loader::<SpriteData>(loader);

        let renderer = app.world_mut().resource_mut::<Renderer>();
        let sprite_renderer = SpriteRenderer::new(&renderer.device, &renderer.config);

        app.insert_resource(Sprites::new())
            .insert_resource(sprite_renderer)
            .add_systems(
                ecs::Schedule::PostUpdate,
                (bind_new_sprite_bundles, update_sprite_data).chain(),
            )
            .add_systems(ecs::Schedule::Render, render_sprites);
    }
}
