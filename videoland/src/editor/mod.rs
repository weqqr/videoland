mod viewport;

use egui::{
    vec2, Align, CollapsingHeader, Color32, Frame, Layout, Margin, Pos2, Rect, Response, RichText,
    ScrollArea, Sense, Stroke, Style, Vec2, Id,
};
use indexmap::IndexMap;

use crate::editor::viewport::Viewport;
use crate::render2::Renderer;
use crate::ui::Ui;

#[derive(serde::Serialize, serde::Deserialize)]
enum Pane {
    Outline,
    Assets,
    Viewport,
    Node,
}

impl Pane {
    fn title(&self) -> &str {
        match self {
            Pane::Outline => "Outline",
            Pane::Assets => "Assets",
            Pane::Viewport => "Viewport",
            Pane::Node => "Node",
        }
    }
}

pub struct EditorData<'a> {
    pub renderer: &'a mut Renderer,
    pub stats: &'a IndexMap<String, String>,
}

struct Behavior<'a> {
    data: EditorData<'a>,
    viewport: &'a mut Viewport,
}

impl<'a> egui_tiles::Behavior<Pane> for Behavior<'a> {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title().into()
    }

    fn gap_width(&self, _style: &Style) -> f32 {
        4.0
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        egui::Frame::none()
            .fill(Color32::from_rgb(0x19, 0x19, 0x19))
            .stroke(Stroke::new(2.0, Color32::from_rgb(0x48, 0x48, 0x48)))
            .show(ui, |ui| {
                let resp = tile_header(ui, pane.title());

                match pane {
                    Pane::Outline => {
                        let size = ui.available_size();
                        ScrollArea::new([true, true]).show(ui, |ui| {
                            ui.allocate_space(vec2(size.x, 0.0));
                            CollapsingHeader::new(" Scene")
                                .default_open(true)
                                .show(ui, |ui| {
                                    // for (_id, node) in self.data.scene.nodes() {
                                    //     match &node.object {
                                    //         SceneObject::Model(model) => {
                                    //             ui.collapsing(format!(" {}", model.name), |ui| {
                                    //                 for mesh in &model.meshes {
                                    //                     ui.label(&mesh.name);
                                    //                 }
                                    //             })
                                    //         }
                                    //     };
                                    // }
                                });
                        });
                    }
                    Pane::Assets => {
                        Frame::none().inner_margin(6.0).show(ui, |ui| {
                            for entry in walkdir::WalkDir::new("data") {
                                let path = entry.unwrap();
                                let path = path.path();
                                ui.label(path.to_string_lossy());
                            }
                        });
                    }
                    Pane::Viewport => self.viewport.ui(ui, &mut self.data),
                    Pane::Node => {}
                }

                if resp.drag_started() {
                    return egui_tiles::UiResponse::DragStarted;
                }

                ui.allocate_space(egui::Vec2::new(ui.available_width(), ui.available_height()));

                egui_tiles::UiResponse::None
            })
            .inner
    }
}

pub struct Editor {
    tree: egui_tiles::Tree<Pane>,

    pub viewport: Viewport,
}

impl Editor {
    const FILE_PATH: &str = "ui-settings.json";

    pub fn new() -> Self {
        let tree = std::fs::read_to_string(Self::FILE_PATH)
            .map(|data| serde_json::from_str(&data).unwrap())
            .unwrap_or_else(|_| create_default_tree());

        let viewport = Viewport::new();

        Self { tree, viewport }
    }

    pub fn save_layout(&mut self) {
        let data = serde_json::to_string_pretty(&self.tree).unwrap();
        std::fs::write("ui-settings.json", data).unwrap();
    }

    pub fn show(&mut self, ui: &Ui, data: EditorData) {
        egui::TopBottomPanel::top("--videoland-editor-top-panel").show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        let _ = ui.button("New Scene");
                        let _ = ui.button("Settings");
                    });
                    ui.menu_button("Edit", |_| ());
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let label = |ui: &mut egui::Ui, text, color| {
                            ui.label(RichText::new(text).color(color));
                        };

                        for (index, (key, value)) in data.stats.iter().rev().enumerate() {
                            label(ui, value, Color32::LIGHT_BLUE);
                            ui.add_space(2.0);
                            label(ui, key, Color32::WHITE);

                            if index < data.stats.len() - 1 {
                                ui.add_space(12.0);
                            }
                        }
                    });
                });
            });
        });

        egui::TopBottomPanel::bottom("--videoland-editor-bottom-panel").show(ui.ctx(), |ui| {
            ui.label(" bottom text");
        });

        egui::CentralPanel::default()
            .frame(Frame::none().outer_margin(4.0))
            .show(ui.ctx(), |ui| {
                let mut behavior = Behavior {
                    data,
                    viewport: &mut self.viewport,
                };
                self.tree.ui(&mut behavior, ui);
            });
    }
}

fn create_default_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let main_panes = vec![
        tiles.insert_pane(Pane::Outline),
        tiles.insert_pane(Pane::Viewport),
        tiles.insert_pane(Pane::Assets),
        tiles.insert_pane(Pane::Node),
    ];

    let root = tiles.insert_horizontal_tile(main_panes);

    egui_tiles::Tree::new(Id::new("--videoland-editor-tree-root"), root, tiles)
}

fn tile_header(ui: &mut egui::Ui, title: &str) -> Response {
    let resp = ui.allocate_response(Vec2::new(ui.available_width(), 16.0), Sense::drag());
    let mut child = ui.child_ui(resp.rect, Layout::left_to_right(Align::Center));

    Frame::none()
        .inner_margin(Margin::symmetric(6.0, 2.0))
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
            ui.label(title);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label("");
                ui.label("");

                let (response, painter) =
                    ui.allocate_painter(Vec2::new(ui.available_width(), 5.0), Sense::hover());
                let rect = response.rect;

                for i in rect.left() as i32..rect.right() as i32 {
                    if i % 4 == 0 {
                        let rec = Rect::from_min_size(
                            Pos2::new(i as f32, rect.top()),
                            Vec2::new(1.0, 1.0),
                        );

                        painter.rect_filled(rec, 0.0, Color32::from_rgb(0x60, 0x60, 0x60));

                        let rec = Rect::from_min_size(
                            Pos2::new(i as f32, rect.top() + 4.0),
                            Vec2::new(1.0, 1.0),
                        );
                        painter.rect_filled(rec, 0.0, Color32::from_rgb(0x60, 0x60, 0x60));
                    }

                    if i % 4 == 2 {
                        let rec = Rect::from_min_size(
                            Pos2::new(i as f32, rect.top() + 2.0),
                            Vec2::new(1.0, 1.0),
                        );
                        painter.rect_filled(rec, 0.0, Color32::from_rgb(0x60, 0x60, 0x60));
                    }
                }
            })
        });

    resp
}
