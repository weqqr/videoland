use egui::{
    menu, pos2, Align, CentralPanel, Color32, Frame, Layout, Rect, Sense, SidePanel, TopBottomPanel,
};

use crate::core::{Defer, Res, ResMut};
use crate::render::{Extent2D, Renderer};
use crate::scene::{SceneGraph, SceneHandle};
use crate::ui::Ui;

pub enum EditorState {
    Show,
    Hide,
}

enum EditorPane {
    Viewport {
        scene_id: SceneHandle,
        texture_id: egui::TextureId,
    },
}

impl EditorPane {
    fn title(&self) -> String {
        match self {
            EditorPane::Viewport { scene_id, .. } => "scene".to_owned(),
        }
    }
}

struct Behavior<'a> {
    renderer: &'a mut Renderer,
    sg: &'a mut SceneGraph,
}

impl<'a> egui_tiles::Behavior<EditorPane> for Behavior<'a> {
    fn tab_title_for_pane(&mut self, pane: &EditorPane) -> egui::WidgetText {
        pane.title().into()
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut EditorPane,
    ) -> egui_tiles::UiResponse {
        match pane {
            EditorPane::Viewport {
                scene_id,
                texture_id,
            } => {
                let (resp, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

                let extent = Extent2D {
                    width: resp.rect.width() as u32,
                    height: resp.rect.height() as u32,
                };

                let scene = self.sg.scene(*scene_id).unwrap();

                self.renderer
                    .render_scene_to_egui_texture(*texture_id, extent, scene);

                let uv = Rect {
                    min: pos2(0.0, 0.0),
                    max: pos2(1.0, 1.0),
                };

                painter.image(*texture_id, resp.rect, uv, Color32::WHITE);

                ui.allocate_ui_at_rect(resp.rect, |ui: &mut egui::Ui| ui.button("text"));
            }
        }

        Default::default()
    }
}

pub struct Editor {
    tree: egui_tiles::Tree<EditorPane>,
    search: String,
}

pub fn init(mut defer: Defer, mut renderer: ResMut<Renderer>, g: Res<SceneGraph>) {
    let mut tiles = egui_tiles::Tiles::default();

    let main_panes = g
        .scenes()
        .map(|(scene_id, _)| {
            tiles.insert_pane(EditorPane::Viewport {
                scene_id,
                texture_id: renderer.create_egui_render_target(Extent2D {
                    width: 256,
                    height: 256,
                }),
            })
        })
        .collect();

    let root = tiles.insert_tab_tile(main_panes);
    let tree = egui_tiles::Tree::new("vl-editor-root", root, tiles);

    defer.insert(Editor {
        tree,
        search: "".to_owned(),
    });
    defer.insert(EditorState::Show);
}

pub fn show(
    mut editor_state: ResMut<EditorState>,
    mut editor: ResMut<Editor>,
    mut renderer: ResMut<Renderer>,
    mut sg: ResMut<SceneGraph>,
    ui: Res<Ui>,
) {
    if let EditorState::Hide = *editor_state {
        return;
    }

    TopBottomPanel::top("vl-editor-top-panel").show(ui.ctx(), |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("hide").clicked() {
                *editor_state = EditorState::Hide;
            }

            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        let _ = ui.button("New");
                        let _ = ui.button("Open");
                    });

                    ui.menu_button("Edit", |ui| {});

                    ui.menu_button("Scene", |ui| {
                        let _ = ui.button("Test 1");
                        let _ = ui.button("Test 2");
                    });
                });
            });
        });
    });

    SidePanel::left("vl-explorer").show(ui.ctx(), |ui| {
        ui.label("do stuff");
    });

    CentralPanel::default()
        .frame(Frame::none())
        .show(ui.ctx(), |ui| {
            editor.tree.ui(
                &mut Behavior {
                    renderer: &mut renderer,
                    sg: &mut sg,
                },
                ui,
            )
        });
}
