use app::{plugins::Plugin, window::Window};
use ecs::{prelude::*, WinnyResource};
use egui::{Rect, Vec2};
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use render::{RenderConfig, RenderDevice};

use crate::gui::EguiRenderer;

fn draw(mut egui: ResMut<EguiRenderer>, ui_state: Res<UiState>) {
    let mut ui_state = ui_state.clone();
    egui.draw(move |ctx| {
        ui_state.ui(ctx);
    })
}

#[derive(WinnyResource, Clone)]
pub struct UiState {
    state: DockState<EguiWindow>,
    viewport_rect: egui::Rect,
    // selected_entities: SelectedEntities,
    // selection: InspectorSelection,
    // gizmo_mode: GizmoMode,
}

impl UiState {
    pub fn new(window: &Window, zoom_factor: f32) -> Self {
        let mut state = DockState::new(vec![EguiWindow::GameView]);
        let tree = state.main_surface_mut();
        let [game, _inspector] =
            tree.split_right(NodeIndex::root(), 0.75, vec![EguiWindow::Inspector]);
        let [game, _hierarchy] = tree.split_left(game, 0.2, vec![EguiWindow::Hierarchy]);
        let [_game, _bottom] =
            tree.split_below(game, 0.8, vec![EguiWindow::Resources, EguiWindow::Assets]);

        let size = window.winit_window.inner_size();
        let screen_size_in_pixels = Vec2::new(size.width as f32, size.height as f32);

        let native_pixels_per_point = window.winit_window.scale_factor() as f32;
        let screen_size_in_points = screen_size_in_pixels / (zoom_factor * native_pixels_per_point);

        let viewport_rect = (screen_size_in_points.x > 0.0 && screen_size_in_points.y > 0.0)
            .then(|| Rect::from_min_size(Default::default(), screen_size_in_points))
            .unwrap();

        Self {
            state,
            // selected_entities: SelectedEntities::default(),
            // selection: InspectorSelection::Entities,
            viewport_rect,
            // gizmo_mode: GizmoMode::Translate,
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        let mut tab_viewer = TabViewer {
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
    Hierarchy,
    Resources,
    Assets,
    Inspector,
}

struct TabViewer<'a> {
    // world: &'a mut World,
    // selected_entities: &'a mut SelectedEntities,
    // selection: &'a mut InspectorSelection,
    viewport_rect: &'a mut egui::Rect,
    // gizmo_mode: GizmoMode,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        // let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        // let type_registry = type_registry.read();

        match window {
            EguiWindow::GameView => {
                // *self.viewport_rect = ui.clip_rect();
                //
                // draw_gizmo(ui, self.world, self.selected_entities, self.gizmo_mode);
            }
            EguiWindow::Hierarchy => {
                // let selected = hierarchy_ui(self.world, ui, self.selected_entities);
                // if selected {
                //     *self.selection = InspectorSelection::Entities;
                // }
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

fn startup(
    mut commands: Commands,
    egui_renderer: Option<Res<EguiRenderer>>,
    device: Res<RenderDevice>,
    config: Res<RenderConfig>,
    window: Res<Window>,
) {
    let Some(egui) = egui_renderer else {
        panic!("The [`EditorPlugin`] was added before the [`EguiPlugin`]");
    };

    let ui_state = UiState::new(&window, egui.context.zoom_factor());
    commands.insert_resource(ui_state);
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.register_resource::<UiState>()
            .add_systems(ecs::Schedule::PostUpdate, draw);
    }
}
