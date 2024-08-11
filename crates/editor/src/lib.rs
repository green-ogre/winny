use app::prelude::*;
use ecs::{
    egui_widget::EguiRegistery, Components, Entities, Tables, UnsafeWorldCell, WinnyResource, *,
};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use gfx::{camera::Camera, gui::EguiRenderer};
use std::any::TypeId;

#[derive(Debug)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&mut self, app: &mut App) {
        app.egui_blacklist::<EguiRegistery>()
            .egui_blacklist::<Editor>()
            .register_resource::<Editor>()
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
    selection: Selection,
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

        let viewport_rect = egui::Rect::ZERO;

        Self {
            state,
            selection: Selection::None,
            viewport_rect,
        }
    }

    fn draw_editor(&mut self, ctx: &egui::Context, world: UnsafeWorldCell<'_>) {
        let mut tab_viewer = TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            selection: &mut self.selection,
        };

        DockArea::new(&mut self.state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Clone, Copy)]
enum Selection {
    None,
    Resource(TypeId),
    Entities(Entity),
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
    selection: &'a mut Selection,
    viewport_rect: &'a mut egui::Rect,
}

fn draw_entities(ui: &mut egui_dock::egui::Ui, entities: &Entities, selection: &mut Selection) {
    for (entity, _) in entities.iter() {
        let checked = match selection {
            Selection::Entities(e) => *e == entity,
            _ => false,
        };
        if ui
            .selectable_label(checked, format!("Entity({})", entity.index()))
            .clicked()
        {
            *selection = Selection::Entities(entity);
        }
    }
}

fn draw_entity(
    ui: &mut egui_dock::egui::Ui,
    registery: &mut EguiRegistery,
    components: &mut Components,
    tables: &mut Tables,
    entities: &Entities,
    entity: Entity,
) {
    if let Some(meta) = entities.meta(entity) {
        let table = tables.get_mut(meta.location.table_id).unwrap();
        for (component_id, column) in table.iter_mut() {
            let m = components.meta_from_id(*component_id).unwrap();
            if registery.black_listed.contains_key(&m.type_id) {
                continue;
            }

            if let Some(drawer) = registery.components.get(&m.type_id) {
                let component = unsafe { column.get_row_ptr_unchecked(meta.location.table_row) };

                ecs::egui::CollapsingHeader::new(m.name)
                    .open(Some(true))
                    .show(ui, |ui| {
                        drawer.display(component, ui);
                    });
            }
        }
    }
}

fn draw_resources(
    ui: &mut egui_dock::egui::Ui,
    registery: &mut EguiRegistery,
    resources: &mut Resources,
    selection: &mut Selection,
) {
    for (index, _) in resources.resources.iter_indexed_mut() {
        let resource_id = ResourceId::new(index);
        let meta = resources.resource_id_table.get(&resource_id).unwrap();

        if registery.black_listed.contains_key(&meta.type_id) {
            continue;
        }

        let checked = match selection {
            Selection::Resource(r) => *r == meta.type_id,
            _ => false,
        };
        if ui
            .selectable_label(
                checked,
                format!("({}) {}", meta.resource_id.index(), meta.name),
            )
            .clicked()
        {
            *selection = Selection::Resource(meta.type_id);
        }
    }
}

/// Blacklisted types cannot be selected, therefore there is no need to check.
fn draw_selected(
    ui: &mut egui_dock::egui::Ui,
    registery: &mut EguiRegistery,
    resources: &mut Resources,
    selection: &mut Selection,
    components: &mut Components,
    tables: &mut Tables,
    entities: &Entities,
) {
    match selection {
        Selection::None => {}
        Selection::Resource(type_id) => {
            if let Some(meta) = resources.meta(type_id) {
                if let Some(drawer) = registery.resources.get(type_id) {
                    let resource = unsafe { resources.get_raw_ptr(meta.resource_id).unwrap() };
                    drawer.display(resource, ui);
                }
            }
        }
        Selection::Entities(entity) => {
            draw_entity(ui, registery, components, tables, entities, *entity);
        }
    }
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        match window {
            EguiWindow::GameView => {
                *self.viewport_rect = ui.clip_rect();
            }
            EguiWindow::Entities => unsafe {
                draw_entities(ui, self.world.entities(), self.selection);
            },
            EguiWindow::Resources => {
                unsafe {
                    draw_resources(
                        ui,
                        self.world.get_resource_mut::<EguiRegistery>(),
                        self.world.resources_mut(),
                        self.selection,
                    )
                };
            }
            // EguiWindow::Assets => select_asset(ui, &type_registry, self.world, self.selection),
            EguiWindow::Inspector => unsafe {
                draw_selected(
                    ui,
                    self.world.get_resource_mut::<EguiRegistery>(),
                    self.world.resources_mut(),
                    self.selection,
                    self.world.components_mut(),
                    self.world.tables_mut(),
                    self.world.entities(),
                );
            },
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
