use app::prelude::*;
use ecs::{Components, Entities, Tables, UnsafeWorldCell, WinnyResource, *};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use gfx::{camera::Camera, gui::EguiRenderer};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&mut self, app: &mut App) {
        app.register_resource::<Editor>()
            .insert_resource(Editor::new())
            .add_systems(Schedule::PostUpdate, update_camera_viewport)
            .add_systems(AppSchedule::Render, render);
    }
}

fn update_camera_viewport(mut camera: Query<Mut<Camera>>, ui: Res<Editor>) {
    let Ok(camera) = camera.get_single_mut() else {
        return;
    };

    if ui.viewport_rect == egui::Rect::ZERO {
        return;
    }

    let viewport = ViewPort {
        min: [ui.viewport_rect.min.x, ui.viewport_rect.min.y].into(),
        max: [ui.viewport_rect.max.x, ui.viewport_rect.max.y].into(),
    };

    camera.viewport = Some(viewport);
}

fn render(world: &mut World) {
    // TODO: Might be UB, cannot test with miri.
    unsafe {
        let world = world.as_unsafe_world();
        let context = world.get_resource::<EguiRenderer>().egui_context();
        world
            .get_resource_mut::<Editor>()
            .draw_editor(context, world);
    }
}

#[derive(WinnyResource, Clone)]
pub struct Editor {
    state: DockState<EguiWindow>,
    viewport_rect: egui::Rect,
    // selected_entities: SelectedEntities,
    // selection: InspectorSelection,
    // gizmo_mode: GizmoMode,
}

impl Editor {
    pub fn new() -> Self {
        let mut state = DockState::new(vec![EguiWindow::GameView]);
        let tree = state.main_surface_mut();
        let [game, _inspector] =
            tree.split_right(NodeIndex::root(), 0.75, vec![EguiWindow::Inspector]);
        let [game, _entities] = tree.split_left(game, 0.2, vec![EguiWindow::Entities]);
        let [_game, _bottom] =
            tree.split_below(game, 0.8, vec![EguiWindow::Resources, EguiWindow::Assets]);

        // let size = window.winit_window.inner_size();
        // let screen_size_in_pixels = Vec2::new(size.width as f32, size.height as f32);
        //
        // let native_pixels_per_point = window.winit_window.scale_factor() as f32;
        // let screen_size_in_points = screen_size_in_pixels / (zoom_factor * native_pixels_per_point);
        //
        // let viewport_rect = (screen_size_in_points.x > 0.0 && screen_size_in_points.y > 0.0)
        //     .then(|| Rect::from_min_size(Default::default(), screen_size_in_points))
        //     .unwrap();

        let viewport_rect = egui::Rect::ZERO;

        Self {
            state,
            // selected_entities: SelectedEntities::default(),
            // selection: InspectorSelection::Entities,
            viewport_rect,
            // gizmo_mode: GizmoMode::Translate,
        }
    }

    fn draw_editor(&mut self, ctx: &egui::Context, world: UnsafeWorldCell<'_>) {
        let mut tab_viewer = TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            // selected_entities: &mut self.selected_entities,
            // selection: &mut self.selection,
            // gizmo_mode: self.gizmo_mode,
        };

        DockArea::new(&mut self.state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Debug, Clone, Copy)]
enum EguiWindow {
    GameView,
    Entities,
    Resources,
    Assets,
    Inspector,
}

struct TabViewer<'a> {
    world: UnsafeWorldCell<'a>,
    // selected_entities: &'a mut SelectedEntities,
    // selection: &'a mut InspectorSelection,
    viewport_rect: &'a mut egui::Rect,
    // gizmo_mode: GizmoMode,
}

fn draw_entities(
    components: &Components,
    entities: &Entities,
    tables: &mut Tables,
    ui: &mut egui_dock::egui::Ui,
) {
    for (entity, meta) in entities.iter() {
        let entity_table = tables.get_mut(meta.location.table_id).unwrap();
        egui::CollapsingHeader::new(format!("Entity ({})", entity.index())).show(ui, |ui| {
            for (component_id, column) in entity_table.iter_mut() {
                let component = unsafe { column.get_row_ptr_unchecked(meta.location.table_row) };
                components.display_component(component_id, component, ui);
            }
        });
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        // let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        // let type_registry = type_registry.read();

        match window {
            EguiWindow::GameView => {
                *self.viewport_rect = ui.clip_rect();

                // draw_gizmo(ui, self.world, self.selected_entities, self.gizmo_mode);
            }
            EguiWindow::Entities => {
                // let selected = hierarchy_ui(self.world, ui, self.selected_entities);
                // if selected {
                //     *self.selection = InspectorSelection::Entities;
                // }
                unsafe {
                    draw_entities(
                        self.world.components(),
                        self.world.entities(),
                        self.world.tables_mut(),
                        ui,
                    );
                }
            }
            // EguiWindow::Resources => select_resource(ui, &type_registry, self.selection),
            // EguiWindow::Assets => select_asset(ui, &type_registry, self.world, self.selection),
            // EguiWindow::Inspector => match *self.selection {
            //     InspectorSelection::Entities => match self.selected_entities.as_slice() {
            //         &[entity] => ui_for_entity_with_children(self.world, entity, ui),
            //         entities => ui_for_entities_shared_components(self.world, entities, ui),
            //     },
            //     InspectorSelection::Resource(type_id, ref name) => {
            //         ui.label(name);
            //         bevy_inspector::by_type_id::ui_for_resource(
            //             self.world,
            //             type_id,
            //             ui,
            //             name,
            //             &type_registry,
            //         )
            //     }
            //     InspectorSelection::Asset(type_id, ref name, handle) => {
            //         ui.label(name);
            //         bevy_inspector::by_type_id::ui_for_asset(
            //             self.world,
            //             type_id,
            //             handle,
            //             ui,
            //             &type_registry,
            //         );
            //     }
            // },
            _ => {}
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        format!("{window:?}").into()
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, EguiWindow::GameView)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miri() {
        let mut world = World::default();
        world.insert_resource(Editor::new());
        render(&mut world);
    }
}
