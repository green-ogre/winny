use std::{cmp::Ordering, fmt::Debug, marker::PhantomData};

use crate::{
    camera::{Camera, CameraUniform},
    render_pipeline::buffer::AsGpuBuffer,
    AsBindGroup, AsVertexBuffer, AsWgpuResources, BindGroup, FragmentShader, FragmentShaderSource,
    Image, Material, RenderAsset, RenderAssetApp, RenderAssets, RenderEncoder, RenderPipeline2d,
    RenderView, Texture, Transform, Vertex, VertexBuffer, VertexShader, VertexUv, WgpuResource,
};
use app::{
    core::{AppSchedule, Schedule},
    plugins::Plugin,
    render_util::RenderContext,
    window::Window,
};
use asset::{server::AssetServer, Asset, AssetApp, AssetLoader, Assets, Handle};
use cereal::{Deserialize, Deserializer, Serialize, WinnyDeserialize, WinnySerialize};
use ecs::*;
use ecs::{egui_widget::AsEgui, WinnyAsEgui};
use math::{
    matrix::Matrix4x4f,
    vector::{Vec2f, Vec4f},
};
use util::info;
use wgpu::core::command::compute_commands::wgpu_compute_pass_push_debug_group;

#[derive(Debug)]
pub struct Mesh2dPlugin;

impl Plugin for Mesh2dPlugin {
    fn build(&mut self, app: &mut app::prelude::App) {
        app.egui_component::<BindedGpuMesh2d>()
            .register_asset::<Mesh2d>()
            .register_render_asset::<GpuMesh2d>()
            .register_asset_loader::<Mesh2d>(Mesh2dAssetLoader);
    }
}

pub struct Mesh2dMatPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Debug for Mesh2dMatPlugin<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Mesh2dPlugin")
    }
}

impl<M: Material> Plugin for Mesh2dMatPlugin<M> {
    fn build(&mut self, app: &mut app::prelude::App) {
        app.register_resource::<Mesh2dPipeline<M>>()
            .add_systems(Schedule::StartUp, startup::<M>)
            .add_systems(
                AppSchedule::Render,
                (
                    bind_new_mesh_bundles::<M>,
                    prepare_render_pass::<M>,
                    render_pass::<M>,
                ),
            );
    }
}

impl<M: Material> Mesh2dMatPlugin<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

struct Mesh2dAssetLoader;

impl AssetLoader for Mesh2dAssetLoader {
    type Asset = Mesh2d;
    type Settings = ();

    fn extensions(&self) -> &'static [&'static str] {
        &["msh"]
    }

    async fn load(
        mut reader: asset::ByteReader<std::io::Cursor<Vec<u8>>>,
        _settings: Self::Settings,
        _path: String,
        _ext: &str,
    ) -> Result<Self::Asset, asset::AssetLoaderError> {
        let mut bytes = reader.read_all()?;
        let mut d = Deserializer::new(&mut bytes);
        Ok(Mesh2d::deserialize(&mut d).unwrap())
    }
}

#[derive(WinnyAsEgui, WinnySerialize, WinnyDeserialize, Default, Debug, Clone)]
pub struct Mesh2d {
    pub triangles: Vec<Triangle>,
}

impl Asset for Mesh2d {}

impl Mesh2d {
    pub fn from_points(points: Points) -> Option<Self> {
        points.into_triangles().map(|t| Mesh2d { triangles: t })
    }

    pub fn as_verts(&self) -> Vec<Vertex> {
        self.triangles
            .iter()
            .map(|t| t.points)
            .flatten()
            .map(|p| p.into())
            .collect()
    }
}

#[derive(Debug)]
pub struct GpuMesh2d {
    buffer: VertexBuffer,
    len: u32,
}

impl RenderAsset for GpuMesh2d {
    type Asset = Mesh2d;
    type Params<'w> = Res<'w, RenderContext>;

    fn prepare_asset<'w>(asset: &Self::Asset, context: &Self::Params<'w>) -> Self {
        let verts = asset.as_verts();
        let buffer = <Vertex as AsVertexBuffer<0>>::as_entire_buffer(
            &context,
            &verts,
            wgpu::BufferUsages::VERTEX,
        );
        let len = verts.len() as u32;

        Self { buffer, len }
    }
}

#[derive(WinnyAsEgui, WinnySerialize, WinnyDeserialize, Default, Debug, Copy, Clone)]
pub struct Triangle {
    points: [Point; 3],
}

#[derive(WinnyAsEgui, WinnySerialize, WinnyDeserialize, Default, Debug, Clone, Copy, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}

impl From<Point> for Vertex {
    fn from(value: Point) -> Self {
        Self {
            position: [value.x, value.y, 0.0, 1.0].into(),
        }
    }
}

impl From<Vertex> for Point {
    fn from(value: Vertex) -> Self {
        Point {
            x: value.position.x,
            y: value.position.y,
        }
    }
}

impl From<Vec2f> for Point {
    fn from(value: Vec2f) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Points(Vec<Point>);

impl Points {
    pub fn add(&mut self, point: impl Into<Point>) {
        self.0.push(point.into());
    }

    pub fn pop(&mut self) {
        let _ = self.0.pop();
    }

    pub fn into_triangles(mut self) -> Option<Vec<Triangle>> {
        minimal_triangulation_with_convex_hull(self.0)
    }
}

fn cross_product(o: &Point, a: &Point, b: &Point) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    if points.len() <= 3 {
        return points;
    }

    // Sort points lexicographically
    points.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal))
    });

    let mut lower = Vec::new();
    for p in &points {
        while lower.len() >= 2
            && cross_product(&lower[lower.len() - 2], &lower[lower.len() - 1], p) <= 0.0
        {
            lower.pop();
        }
        lower.push(p.clone());
    }

    let mut upper = Vec::new();
    for p in points.iter().rev() {
        while upper.len() >= 2
            && cross_product(&upper[upper.len() - 2], &upper[upper.len() - 1], p) <= 0.0
        {
            upper.pop();
        }
        upper.push(p.clone());
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn minimal_triangulation_with_convex_hull(points: Vec<Point>) -> Option<Vec<Triangle>> {
    if points.len() < 3 {
        return None;
    }

    let hull = convex_hull(points.clone());
    let mut triangles = Vec::new();

    // Triangulate the convex hull
    for i in 1..hull.len() - 1 {
        triangles.push(Triangle {
            points: [hull[0].clone(), hull[i].clone(), hull[i + 1].clone()],
        });
    }

    // Triangulate interior points
    for point in points.iter() {
        if !hull.contains(point) {
            // Find the visible edges of the existing triangulation
            let mut visible_edges = Vec::new();
            for triangle in &triangles {
                for i in 0..3 {
                    let j = (i + 1) % 3;
                    if cross_product(&triangle.points[i], &triangle.points[j], point) > 0.0 {
                        visible_edges
                            .push((triangle.points[i].clone(), triangle.points[j].clone()));
                    }
                }
            }

            // Create new triangles
            for (p1, p2) in visible_edges {
                triangles.push(Triangle {
                    points: [p1, p2, point.clone()],
                });
            }
        }
    }

    Some(triangles)
}

fn startup<M: Material>(mut commands: Commands, context: Res<RenderContext>) {
    // commands.insert_resource(Mesh2dPipeline::<M>::new(&context, ));
}

#[derive(WinnyResource)]
pub struct Mesh2dPipeline<M: Material> {
    pipeline: RenderPipeline2d,
    camera: BindGroup,
    material: BindGroup,
    transforms: VertexBuffer,
    _phantom: PhantomData<M>,
}

impl<M: Material> Mesh2dPipeline<M> {
    pub fn new(
        context: &RenderContext,
        material: M,
        shaders: &mut Assets<FragmentShaderSource>,
        server: &AssetServer,
        state: <M as AsWgpuResources>::State<'_>,
    ) -> Self {
        let (vert, frag) = get_shaders(context, &material, shaders, server);

        let camera = <&[CameraUniform] as AsBindGroup>::as_entire_binding_empty(
            context,
            &[],
            std::mem::size_of::<CameraUniform>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let material = <M as AsBindGroup>::as_entire_binding(context, material.clone(), state);

        let transforms = <Matrix4x4f as AsVertexBuffer<1>>::as_entire_buffer_empty(
            context,
            std::mem::size_of::<Matrix4x4f>() as u64,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        let pipeline = RenderPipeline2d::new(
            "mesh",
            context,
            &[camera.layout(), material.layout()],
            &[
                <Vertex as AsVertexBuffer<0>>::vertex_layout(),
                <Matrix4x4f as AsVertexBuffer<1>>::vertex_layout(),
            ],
            &vert,
            &frag,
            wgpu::BlendState::ALPHA_BLENDING,
            None,
        );

        Mesh2dPipeline {
            pipeline,
            camera,
            material,
            transforms,
            _phantom: PhantomData,
        }
    }
}

fn get_shaders<'s, M: Material>(
    context: &RenderContext,
    material: &M,
    shaders: &'s mut Assets<FragmentShaderSource>,
    server: &AssetServer,
) -> (VertexShader, &'s FragmentShader) {
    (
        VertexShader({
            let shader = wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../res/shaders/mesh2d.wgsl").into(),
                ),
            };
            context.device.create_shader_module(shader)
        }),
        RenderPipeline2d::material_frag(
            material,
            server,
            crate::FragmentType::Mesh2d,
            shaders,
            context,
        ),
    )
}

#[derive(WinnyComponent, WinnyAsEgui)]
struct BindedGpuMesh2d;

fn bind_new_mesh_bundles<M: Material>(
    mut commands: Commands,
    pipeline: Option<Res<Mesh2dPipeline<M>>>,
    context: Res<RenderContext>,
    mut textures: ResMut<RenderAssets<Texture>>,
    images: Res<Assets<Image>>,
    mesh_bundles: Query<(Entity, Handle<Mesh2d>, M), (With<Transform>, Without<BindedGpuMesh2d>)>,
    meshes: Res<Assets<Mesh2d>>,
    mut gpu_meshes: ResMut<RenderAssets<GpuMesh2d>>,
    params: <GpuMesh2d as RenderAsset>::Params<'_>,
    server: Res<AssetServer>,
    mut shaders: ResMut<Assets<FragmentShaderSource>>,
) {
    let mut generated_pipeline = false;
    for (entity, handle, material) in mesh_bundles.iter() {
        if pipeline.is_none() && !generated_pipeline {
            if shaders
                .get(&material.mesh_2d_fragment_shader(&server))
                .is_some()
            {
                if let Some(state) = material.resource_state(&mut textures, &images, &context) {
                    let pipeline = Mesh2dPipeline::new(
                        &context,
                        material.clone(),
                        &mut shaders,
                        &server,
                        state,
                    );
                    commands.insert_resource(pipeline);
                    generated_pipeline = true;
                } else {
                    return;
                }
            } else {
                return;
            }
        }

        if gpu_meshes.get(handle).is_some() {
            commands.get_entity(entity).insert(BindedGpuMesh2d);
            continue;
        } else {
            if let Some(mesh) = meshes.get(handle) {
                info!("generating new gpu_mesh: [{mesh:?}]");
                let mesh = GpuMesh2d::prepare_asset(mesh, &params);
                gpu_meshes.insert(handle.clone(), mesh);
                commands.get_entity(entity).insert(BindedGpuMesh2d);
            }
        }
    }
}

fn prepare_render_pass<M: Material>(
    mut commands: Commands,
    mut pipeline: Option<ResMut<Mesh2dPipeline<M>>>,
    context: Res<RenderContext>,
    meshes: Query<(Transform, M), With<(Handle<Mesh2d>, BindedGpuMesh2d)>>,
    mut gpu_meshes: ResMut<RenderAssets<GpuMesh2d>>,
    params: <GpuMesh2d as RenderAsset>::Params<'_>,
    camera: Query<(Camera, Transform)>,
    window: Res<Window>,
) {
    let Some(mut pipeline) = pipeline else {
        return;
    };

    if let Ok((camera, transform)) = camera.get_single() {
        CameraUniform::write_buffer(
            &context,
            pipeline.camera.single_buffer(),
            &[CameraUniform::from_camera(camera, transform, &window)],
        );
    }

    let transform_data = meshes
        .iter()
        .map(|(t, _)| t.as_matrix())
        .collect::<Vec<_>>();
    <Matrix4x4f as AsVertexBuffer<1>>::write_buffer_resize(
        &context,
        &mut pipeline.transforms,
        &transform_data,
    );

    // for (_, material) in meshes.iter() {
    //     M::update(material, &context, &pipeline.material);
    // }
}

fn render_pass<M: Material>(
    mut encoder: ResMut<RenderEncoder>,
    view: Res<RenderView>,
    pipeline: Option<Res<Mesh2dPipeline<M>>>,
    meshes: Query<(Handle<Mesh2d>, M), With<(BindedGpuMesh2d, Transform)>>,
    gpu_meshes: Res<RenderAssets<GpuMesh2d>>,
    context: Res<RenderContext>,
) {
    let Some(pipeline) = pipeline else {
        return;
    };

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("draw to output"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    render_pass.set_pipeline(&pipeline.pipeline.0);
    render_pass.set_vertex_buffer(1, pipeline.transforms.buffer().slice(..));
    render_pass.set_bind_group(0, pipeline.camera.binding(), &[]);
    render_pass.set_bind_group(1, pipeline.material.binding(), &[]);

    for (i, (mesh, material)) in meshes.iter().enumerate() {
        let gpu_mesh = gpu_meshes.get(mesh).unwrap();
        render_pass.set_vertex_buffer(0, gpu_mesh.buffer.buffer().slice(..));
        render_pass.draw(0..gpu_mesh.len, i as u32..i as u32 + 1);
    }
}
