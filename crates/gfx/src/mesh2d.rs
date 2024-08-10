use crate::{render_pipeline::buffer::AsGpuBuffer, AsVertexBuffer, Vertex, VertexUv};
use ecs::WinnyAsEgui;
use math::vector::{Vec2f, Vec4f};

#[derive(WinnyAsEgui, Debug, Clone)]
pub struct Mesh2d {
    triangles: Vec<Triangle>,
}

impl Mesh2d {
    pub fn from_points(points: Points) -> Option<Self> {
        points.into_triangles().map(|t| Mesh2d { triangles: t })
    }

    pub fn as_verts(&self) -> Vec<Vertex> {
        self.triangles.iter().map(|t| t.points).flatten().collect()
    }
}

#[repr(C)]
#[derive(WinnyAsEgui, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Triangle {
    points: [Vertex; 3],
}

unsafe impl AsGpuBuffer for Triangle {}

#[derive(Debug, Clone, Copy)]
pub struct Point(Vec2f);

impl From<Point> for Vertex {
    fn from(value: Point) -> Self {
        Self {
            position: [value.0.x, value.0.y, 0.0, 1.0].into(),
        }
    }
}

impl From<Vertex> for Point {
    fn from(value: Vertex) -> Self {
        Point(Vec2f::new(value.position.x, value.position.y))
    }
}

impl From<Vec2f> for Point {
    fn from(value: Vec2f) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Points(Vec<Point>);

impl Points {
    pub fn add(&mut self, point: impl Into<Point>) {
        self.0.push(point.into());
    }

    pub fn into_triangles(mut self) -> Option<Vec<Triangle>> {
        if self.0.len() >= 3 {
            let points = order_ccw(
                self.0.pop().unwrap().into(),
                self.0.pop().unwrap().into(),
                self.0.pop().unwrap().into(),
            );
            let mut triangles = vec![Triangle {
                points: [points[0].into(), points[1].into(), points[2].into()],
            }];

            for point in self.0.into_iter() {
                let mut t = triangles
                    .iter()
                    .map(|t| t.points)
                    .flatten()
                    .collect::<Vec<_>>();
                t.sort_by(|p1, p2| {
                    let p1 = Vec2f::new(p1.position.x, p1.position.y);
                    let p2 = Vec2f::new(p2.position.x, p2.position.y);
                    (p1.dist2(&point.0)).total_cmp(&p1.dist2(&point.0))
                });

                let points = order_ccw(
                    t.pop().unwrap().into(),
                    t.pop().unwrap().into(),
                    t.pop().unwrap().into(),
                );

                triangles.push(Triangle {
                    points: [points[0].into(), points[1].into(), points[2].into()],
                });
            }

            Some(triangles)
        } else {
            None
        }
    }
}

fn order_ccw(p1: Point, p2: Point, p3: Point) -> [Point; 3] {
    // Calculate cross product
    let cross_product =
        (p2.0.x - p1.0.x) * (p3.0.y - p1.0.y) - (p2.0.y - p1.0.y) * (p3.0.x - p1.0.x);

    if cross_product > 0.0 {
        [p1, p2, p3] // Already counter-clockwise
    } else if cross_product < 0.0 {
        [p1, p3, p2] // Swap p2 and p3 to make counter-clockwise
    } else {
        [p1, p2, p3] // Collinear, original order
    }
}
