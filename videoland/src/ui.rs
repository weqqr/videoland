use egui::epaint::Shadow;
use egui::{
    Align2, ClippedPrimitive, Color32, Context, FontData, FontDefinitions, FontFamily, FontTweak,
    Frame, Margin, RichText, Rounding, Stroke, TexturesDelta, Vec2,
};
use indexmap::IndexMap;
use winit::event::WindowEvent;
use winit::window::Window;

pub struct Ui {
    ctx: egui::Context,
    winit_state: egui_winit::State,
}

pub struct RenderedUi {
    pub shapes: Vec<ClippedPrimitive>,
    pub textures_delta: TexturesDelta,
}

#[cfg(windows)]
fn load_font() -> Vec<u8> {
    // TODO: load native font
    std::fs::read("C:\\windows\\fonts\\segoeui.ttf").unwrap()
}

impl Ui {
    pub fn new(window: &Window) -> Self {
        let ctx = egui::Context::default();
        let winit_state = egui_winit::State::new(ctx.clone(), ctx.viewport_id(), window, None, None);

        let main = load_font();

        let mut fonts = FontDefinitions::default();
        fonts
            .font_data
            .insert("main".to_owned(), FontData::from_owned(main));

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "main".to_owned());

        fonts.families.insert(
            egui::FontFamily::Name("main".into()),
            vec!["main".to_owned()],
        );

        let codicon = std::fs::read("data/fonts/codicon.ttf").unwrap();
        let tweak = FontTweak {
            y_offset_factor: 0.15,
            ..Default::default()
        };

        fonts.font_data.insert(
            "codicon".to_owned(),
            FontData::from_owned(codicon).tweak(tweak),
        );

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .push("codicon".to_owned());

        fonts.families.insert(
            egui::FontFamily::Name("codicon".into()),
            vec!["codicon".to_owned()],
        );

        ctx.set_fonts(fonts);

        ctx.style_mut(|style| {
            style.visuals.widgets.noninteractive.fg_stroke.color =
                Color32::from_rgb(0xFA, 0xFA, 0xFA);
            style.visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(0xD6, 0xD6, 0xD6);
        });

        Self { ctx, winit_state }
    }

    pub fn on_event(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.winit_state.on_window_event(window, event);
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let input = self.winit_state.take_egui_input(window);
        self.ctx.begin_frame(egui::RawInput::default());
    }

    pub fn status_bar(&self, data: IndexMap<String, String>) {
        egui::Window::new("--videoland-status-bar")
            .anchor(Align2::LEFT_BOTTOM, Vec2::ZERO)
            .title_bar(false)
            .movable(false)
            .collapsible(false)
            .fixed_size(Vec2::new(100.0, 16.0))
            .frame(Frame {
                inner_margin: Margin::symmetric(4.0, 1.0),
                outer_margin: Margin::same(0.0),
                rounding: Rounding::ZERO,
                shadow: Shadow::NONE,
                fill: Palette::BLACK,
                stroke: Stroke::new(1.0, Palette::GREY),
            })
            .show(&self.ctx, |ui| {
                status_data(ui, &data);
            });
    }

    pub fn finish_frame(&mut self, window: &Window) -> RenderedUi {
        let output = self.ctx.end_frame();

        self.winit_state
            .handle_platform_output(window, output.platform_output);
        let shapes = self
            .ctx
            .tessellate(output.shapes, window.scale_factor() as f32);
        let textures_delta = output.textures_delta;

        RenderedUi {
            shapes,
            textures_delta,
        }
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

pub fn status_data(ui: &mut egui::Ui, data: &IndexMap<String, String>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let label = |ui: &mut egui::Ui, text, color| {
            ui.label(RichText::new(text).color(color));
        };

        for (index, (key, value)) in data.iter().enumerate() {
            label(ui, key, Palette::WHITE);
            ui.add_space(2.0);
            label(ui, value, Palette::HIGH_CYAN);

            if index < data.len() - 1 {
                ui.add_space(12.0);
            }
        }
    });
}

enum Palette {}

impl Palette {
    const BLACK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
    const GREY: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);
    const LIGHT_GREY: Color32 = Color32::from_rgb(0xaa, 0xaa, 0xaa);
    const WHITE: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);

    const LOW_BLUE: Color32 = Color32::from_rgb(0x00, 0x00, 0xaa);
    const HIGH_BLUE: Color32 = Color32::from_rgb(0x55, 0x55, 0xff);

    const LOW_GREEN: Color32 = Color32::from_rgb(0x00, 0xaa, 0x00);
    const HIGH_GREEN: Color32 = Color32::from_rgb(0x55, 0xff, 0x55);

    const LOW_CYAN: Color32 = Color32::from_rgb(0x00, 0xaa, 0xaa);
    const HIGH_CYAN: Color32 = Color32::from_rgb(0x55, 0xff, 0xff);

    const LOW_RED: Color32 = Color32::from_rgb(0xaa, 0x00, 0x00);
    const HIGH_RED: Color32 = Color32::from_rgb(0xff, 0x55, 0x55);

    const LOW_MAGENTA: Color32 = Color32::from_rgb(0xaa, 0x00, 0xaa);
    const HIGH_MAGENTA: Color32 = Color32::from_rgb(0xff, 0x55, 0xff);

    const BROWN: Color32 = Color32::from_rgb(0xaa, 0x55, 0x00);
    const YELLOW: Color32 = Color32::from_rgb(0xff, 0xff, 0x55);
}
