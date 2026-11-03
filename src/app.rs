use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, Frame, Label, Layout, Margin, Rect, Response,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2, ViewportCommand,
    pos2, vec2,
};

use crate::convert::{self, DEFAULT_SIZES, ICON_SIZES, IconSource, SourceFormat};
use crate::theme::{
    self, ACCENT, ACCENT_BRIGHT, AMBER, CYAN, DIM, MINT, MUTED, ROSE, TEXT, TEXT_SOFT,
};

const TITLEBAR_HEIGHT: f32 = 48.0;
const HEADER_HEIGHT: f32 = 48.0;
const MAIN_HEIGHT: f32 = 400.0;
const PREVIEW_HEIGHT: f32 = 196.0;
const SIZES_HEIGHT: f32 = 192.0;
const SOURCE_HEIGHT: f32 = 248.0;
const CANVAS_HEIGHT: f32 = 148.0;
const CARD_GAP: f32 = 12.0;
const OUTER_MARGIN: f32 = 24.0;
const TITLEBAR_PAD: f32 = 16.0;
const PADDING_MAX: f32 = 24.0;

const SOURCE_INNER_HEIGHT: f32 = SOURCE_HEIGHT - theme::CARD_PADDING * 2.0;
const CANVAS_INNER_HEIGHT: f32 = CANVAS_HEIGHT - theme::CARD_PADDING * 2.0;
const PREVIEW_INNER_HEIGHT: f32 = PREVIEW_HEIGHT - theme::CARD_PADDING * 2.0;
const SIZES_INNER_HEIGHT: f32 = SIZES_HEIGHT - theme::CARD_PADDING * 2.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoticeKind {
    Info,
    Success,
    Warning,
    Error,
}

impl NoticeKind {
    fn color(self) -> Color32 {
        match self {
            Self::Info => CYAN,
            Self::Success => MINT,
            Self::Warning => AMBER,
            Self::Error => ROSE,
        }
    }
}

struct Notice {
    kind: NoticeKind,
    text: String,
}

#[derive(Clone)]
struct SourceSummary {
    name: String,
    path: String,
    format: SourceFormat,
    native_size: String,
    file_size: String,
    low_resolution: bool,
}

pub struct IconApp {
    source: Option<IconSource>,
    enabled: [bool; ICON_SIZES.len()],
    previews: BTreeMap<u32, TextureHandle>,
    backdrop: theme::PreviewBackdrop,
    padding: f32,
    output_path: Option<PathBuf>,
    notice: Option<Notice>,
    armed_overwrite: Option<PathBuf>,
    refresh_at: Option<Instant>,
    texture_serial: u64,
    first_frame: bool,
}

impl IconApp {
    pub fn new(
        creation: &eframe::CreationContext<'_>,
        _zoom_factor: f32,
        initial_source: Option<PathBuf>,
    ) -> Self {
        theme::install(&creation.egui_ctx);

        let mut enabled = [false; ICON_SIZES.len()];
        for size in DEFAULT_SIZES {
            if let Some(index) = ICON_SIZES.iter().position(|candidate| *candidate == size) {
                enabled[index] = true;
            }
        }

        let mut app = Self {
            source: None,
            enabled,
            previews: BTreeMap::new(),
            backdrop: theme::PreviewBackdrop::Grid,
            padding: 8.0,
            output_path: None,
            notice: None,
            armed_overwrite: None,
            refresh_at: None,
            texture_serial: 0,
            first_frame: true,
        };

        if let Some(path) = initial_source {
            let _ = app.load_from_path(&path, &creation.egui_ctx);
        }

        app
    }

    fn source_summary(&self) -> Option<SourceSummary> {
        self.source.as_ref().map(|source| SourceSummary {
            name: source.file_name.clone(),
            path: source.path.display().to_string(),
            format: source.format,
            native_size: source.native_size_label(),
            file_size: convert::format_bytes(source.file_size),
            low_resolution: source.is_low_resolution(),
        })
    }

    fn selected_sizes(&self) -> Vec<u32> {
        ICON_SIZES
            .iter()
            .enumerate()
            .filter_map(|(index, size)| self.enabled[index].then_some(*size))
            .collect()
    }

    fn selected_count(&self) -> usize {
        self.enabled.iter().filter(|selected| **selected).count()
    }

    fn set_notice(&mut self, kind: NoticeKind, text: impl Into<String>) {
        self.notice = Some(Notice {
            kind,
            text: text.into(),
        });
    }

    fn load_from_path(&mut self, path: &Path, ctx: &egui::Context) -> Result<(), String> {
        let source = IconSource::load(path)?;
        let file_name = source.file_name.clone();
        let output = source.path.with_extension("ico");

        self.source = Some(source);
        self.output_path = Some(output);
        self.armed_overwrite = None;
        self.refresh_at = None;
        self.previews.clear();
        self.rebuild_previews(ctx);
        self.set_notice(NoticeKind::Success, format!("Loaded {file_name}."));
        Ok(())
    }

    fn clear_source(&mut self) {
        self.source = None;
        self.previews.clear();
        self.output_path = None;
        self.armed_overwrite = None;
        self.refresh_at = None;
        self.set_notice(NoticeKind::Info, "Source removed.");
    }

    fn rebuild_previews(&mut self, ctx: &egui::Context) {
        let Some(source) = self.source.as_ref() else {
            self.previews.clear();
            return;
        };

        let mut next = BTreeMap::new();
        let mut failure = None;
        self.texture_serial = self.texture_serial.wrapping_add(1);

        for size in ICON_SIZES {
            match source.render(size, self.padding) {
                Ok(image) => {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [size as usize, size as usize],
                        image.as_raw(),
                    );
                    let texture = ctx.load_texture(
                        format!("iconbv-preview-{}-{}-{}", size, self.padding as u32, self.texture_serial),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    next.insert(size, texture);
                }
                Err(error) => {
                    failure = Some(format!("Could not render the {size} px frame: {error}"));
                    break;
                }
            }
        }

        if let Some(error) = failure {
            self.set_notice(NoticeKind::Error, error);
        } else {
            self.previews = next;
        }
    }

    fn schedule_preview_refresh(&mut self, ctx: &egui::Context) {
        let at = Instant::now() + Duration::from_millis(90);
        self.refresh_at = Some(at);
        ctx.request_repaint_after(Duration::from_millis(90));
    }

    fn tick_preview_refresh(&mut self, ctx: &egui::Context) {
        let Some(at) = self.refresh_at else {
            return;
        };
        let now = Instant::now();
        if now >= at {
            self.refresh_at = None;
            self.rebuild_previews(ctx);
        } else {
            ctx.request_repaint_after(at.saturating_duration_since(now));
        }
    }

    fn open_source_dialog(&mut self, ctx: &egui::Context) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Choose SVG or PNG artwork")
            .add_filter("Icon artwork", &["svg", "png"]);

        if let Some(directory) = self
            .source
            .as_ref()
            .and_then(|source| source.path.parent())
            .or_else(|| self.output_path.as_ref().and_then(|path| path.parent()))
        {
            dialog = dialog.set_directory(directory);
        }

        if let Some(path) = dialog.pick_file() {
            if let Err(error) = self.load_from_path(&path, ctx) {
                self.set_notice(NoticeKind::Error, error);
            }
        }
    }

    fn choose_output_path(&mut self) {
        let suggestion = self
            .source
            .as_ref()
            .and_then(|source| source.path.file_stem())
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .map(|stem| format!("{stem}.ico"))
            .unwrap_or_else(|| "icon.ico".to_owned());

        let mut dialog = rfd::FileDialog::new()
            .set_title("Save Windows icon")
            .add_filter("Windows icon", &["ico"])
            .set_file_name(&suggestion);

        if let Some(directory) = self
            .output_path
            .as_ref()
            .and_then(|path| path.parent())
            .or_else(|| self.source.as_ref().and_then(|source| source.path.parent()))
        {
            dialog = dialog.set_directory(directory);
        }

        if let Some(mut path) = dialog.save_file() {
            let is_ico = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ico"));
            if !is_ico {
                path.set_extension("ico");
            }
            self.output_path = Some(path);
            self.armed_overwrite = None;
            self.notice = None;
        }
    }

    fn export_now(&mut self) {
        let Some(source) = self.source.as_ref() else {
            self.set_notice(NoticeKind::Warning, "Load an SVG or PNG before exporting.");
            return;
        };

        let sizes = self.selected_sizes();
        if sizes.is_empty() {
            self.set_notice(NoticeKind::Warning, "Select at least one icon size to export.");
            return;
        }

        let Some(destination) = self.output_path.clone() else {
            self.choose_output_path();
            return;
        };

        if destination.exists() && self.armed_overwrite.as_ref() != Some(&destination) {
            self.armed_overwrite = Some(destination.clone());
            self.set_notice(
                NoticeKind::Warning,
                "That file already exists. Press Replace ICO to overwrite it.",
            );
            return;
        }

        match convert::export_ico(source, &sizes, self.padding, &destination) {
            Ok(bytes) => {
                self.armed_overwrite = None;
                self.set_notice(
                    NoticeKind::Success,
                    format!(
                        "Saved {} frames to {} ({})",
                        sizes.len(),
                        destination
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("icon.ico"),
                        convert::format_bytes(bytes)
                    ),
                );
            }
            Err(error) => self.set_notice(NoticeKind::Error, error),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (open, export, save_as) = ctx.input(|input| {
            let command = input.modifiers.command;
            (
                command && !input.modifiers.shift && input.key_pressed(egui::Key::O),
                command && !input.modifiers.shift && input.key_pressed(egui::Key::S),
                command && input.modifiers.shift && input.key_pressed(egui::Key::S),
            )
        });

        if open {
            self.open_source_dialog(ctx);
        } else if save_as {
            self.choose_output_path();
        } else if export {
            self.export_now();
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }

        let mut loaded = false;
        let mut last_error = None;
        for file in dropped {
            let Some(path) = file.path else {
                continue;
            };
            match self.load_from_path(&path, ctx) {
                Ok(()) => {
                    loaded = true;
                    break;
                }
                Err(error) => last_error = Some(error),
            }
        }

        if !loaded {
            if let Some(error) = last_error {
                self.set_notice(NoticeKind::Error, error);
            }
        }
    }

    fn draw_titlebar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("iconbv-titlebar")
            .exact_height(TITLEBAR_HEIGHT)
            .frame(Frame::NONE.inner_margin(Margin {
                left: TITLEBAR_PAD as i8,
                right: TITLEBAR_PAD as i8,
                top: 6,
                bottom: 6,
            }))
            .show(ctx, |ui| {
                let bar_rect = ui.max_rect();

                ui.horizontal(|ui| {
                    ui.add_space(8.0);

                    let (logo_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                    theme::paint_logo_mark(ui.painter(), logo_rect);

                    ui.add_space(11.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("ICONBV")
                                .font(theme::display(13.0))
                                .color(TEXT)
                                .line_height(Some(15.0)),
                        );
                        ui.label(
                            RichText::new("ICON STUDIO")
                                .font(theme::display(7.5))
                                .color(DIM)
                                .line_height(Some(9.0)),
                        );
                    });

                    ui.add_space(20.0);
                    let (divider_rect, _) =
                        ui.allocate_exact_size(vec2(1.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(
                        divider_rect,
                        0.0,
                        theme::rgba(255, 255, 255, 10),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(12.0);

                        if window_control(ui, WindowControl::Close).clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                        if window_control(ui, WindowControl::Minimize).clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                        }

                        ui.add_space(14.0);
                        if bar_rect.width() > 640.0 {
                            ui.label(
                                RichText::new("SVG / PNG   \u{2192}   ICO")
                                    .font(theme::body(9.0))
                                    .color(DIM),
                            );
                        }
                    });
                });

                let drag_rect = Rect::from_min_max(
                    pos2(bar_rect.left() + 4.0, bar_rect.top() + 2.0),
                    pos2(bar_rect.right() - 100.0, bar_rect.bottom() - 2.0),
                );
                let drag_response = ui.interact(
                    drag_rect,
                    ui.id().with("titlebar-drag-region"),
                    Sense::click_and_drag(),
                );

                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag_response.double_clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                if drag_response.hovered() {
                    ctx.set_cursor_icon(CursorIcon::Grab);
                }
            });
    }

    fn draw_content(&mut self, ctx: &egui::Context) {

        egui::TopBottomPanel::bottom("iconbv-export")
            .exact_height(84.0 + 26.0)
            .frame(Frame::NONE.inner_margin(Margin {
                left: OUTER_MARGIN as i8,
                right: OUTER_MARGIN as i8,
                top: 8,
                bottom: 18,
            }))
            .show(ctx, |ui| {
                let full_width = ui.available_width();
                let content_width = full_width.min(1180.0);
                let side_space = ((full_width - content_width) * 0.5).max(0.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.add_space(side_space);
                    ui.vertical(|ui| {
                        ui.set_width(content_width);
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        self.draw_export_card(ui, ctx);
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin {
                left: OUTER_MARGIN as i8,
                right: OUTER_MARGIN as i8,
                top: 16,
                bottom: 6,
            }))
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let full_width = ui.available_width();
                        let content_width = full_width.min(1180.0);
                        let side_space = ((full_width - content_width) * 0.5).max(0.0);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            ui.add_space(side_space);
                            ui.vertical(|ui| {
                                ui.set_width(content_width);
                                ui.spacing_mut().item_spacing = Vec2::ZERO;

                                self.draw_header(ui);
                                ui.add_space(8.0);

                                let left_width = (content_width * 0.33).clamp(352.0, 392.0);
                                let right_width = content_width - left_width - CARD_GAP;

                                ui.horizontal_top(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                                    ui.allocate_ui_with_layout(
                                        vec2(left_width, MAIN_HEIGHT),
                                        Layout::top_down(Align::Min),
                                        |ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                                            self.draw_source_card(ui, ctx);
                                            ui.add_space(CARD_GAP);
                                            self.draw_canvas_card(ui, ctx);
                                        },
                                    );

                                    ui.add_space(CARD_GAP);

                                    ui.allocate_ui_with_layout(
                                        vec2(right_width, MAIN_HEIGHT),
                                        Layout::top_down(Align::Min),
                                        |ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                                            self.draw_preview_card(ui);
                                            ui.add_space(CARD_GAP);
                                            self.draw_sizes_card(ui);
                                        },
                                    );
                                });

                                ui.add_space(14.0);
                            });
                        });
                    });
            });
    }

    fn draw_header(&mut self, ui: &mut Ui) {
        let (status_color, status_text) = if self.source.is_none() {
            (MUTED, "AWAITING SOURCE")
        } else if self.selected_count() == 0 {
            (AMBER, "SELECT A SIZE")
        } else {
            (MINT, "READY TO EXPORT")
        };

        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), HEADER_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {


                let text_width = (ui.available_width() * 0.62).min(700.0);
                ui.allocate_ui_with_layout(
                    vec2(text_width, HEADER_HEIGHT),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 1.2;
                        ui.label(
                            RichText::new("WINDOWS ICON STUDIO")
                                .font(theme::display(9.5))
                                .color(ACCENT_BRIGHT)
                                .line_height(Some(11.0)),
                        );
                        ui.label(
                            RichText::new("Build your icon.")
                                .font(theme::display(23.0))
                                .color(TEXT)
                                .line_height(Some(28.0)),
                        );
                        ui.label(
                            RichText::new(
                                "One SVG or PNG in. A complete, multi-resolution Windows icon out.",
                            )
                            .font(theme::body(10.0))
                            .color(MUTED)
                            .line_height(Some(13.0)),
                        );
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    status_pill(ui, status_text, status_color);
                    ui.add_space(8.0);
                    theme::badge(
                        ui,
                        "ICO  \u{00B7}  32-BIT",
                        theme::rgba(255, 255, 255, 7),
                        MUTED,
                    );
                });
            },
        );
    }

    fn draw_source_card(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let info = self.source_summary();
        let preview = self.previews.get(&256).cloned();
        let dragging_files = ctx.input(|input| !input.raw.hovered_files.is_empty());
        let mut open_requested = false;
        let mut remove_requested = false;

        theme::glass_card(ui, SOURCE_INNER_HEIGHT, |ui| {
            theme::card_header(ui, "SOURCE", "Artwork", |ui| {
                if let Some(info) = &info {
                    theme::badge(
                        ui,
                        info.format.label(),
                        theme::accent_fill(22),
                        ACCENT_BRIGHT,
                    );
                } else {
                    theme::badge(
                        ui,
                        "SVG / PNG",
                        theme::rgba(255, 255, 255, 7),
                        DIM,
                    );
                }
            });

            ui.add_space(9.0);
            let dropzone = draw_dropzone(ui, preview.as_ref(), info.as_ref(), dragging_files);
            if dropzone.clicked() {
                open_requested = true;
            }

            ui.add_space(9.0);
            let name = info
                .as_ref()
                .map(|info| info.name.clone())
                .unwrap_or_else(|| "No source loaded".to_owned());
            let name_color = if info.is_some() { TEXT } else { MUTED };
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), 18.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    let label = Label::new(
                        RichText::new(name)
                            .font(theme::display(12.0))
                            .color(name_color)
                            .line_height(Some(16.0)),
                    )
                    .truncate();

                    if let Some(info) = &info {
                        ui.add_sized([ui.available_width(), 18.0], label)
                            .on_hover_text(&info.path);
                    } else {
                        ui.add(label);
                    }
                },
            );

            ui.add_space(3.0);
            if let Some(info) = &info {
                ui.horizontal(|ui| {
                    let detail_color = if info.low_resolution { AMBER } else { MUTED };
                    let detail = if info.low_resolution {
                        format!("Low-res {} \u{00B7} {}", info.format, info.native_size)
                    } else {
                        format!("{} source \u{00B7} {}", info.format, info.native_size)
                    };
                    ui.label(
                        RichText::new(detail)
                            .font(theme::body(10.0))
                            .color(detail_color)
                            .line_height(Some(13.0)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(&info.file_size)
                                .font(theme::body(10.0))
                                .color(DIM),
                        );
                    });
                });
            } else {
                ui.label(
                    RichText::new("Vector or transparent raster artwork works best.")
                        .font(theme::body(10.0))
                        .color(DIM)
                        .line_height(Some(13.0)),
                );
            }

            ui.add_space(9.0);
            ui.horizontal(|ui| {
                if info.is_some() {
                    if small_button(ui, "Replace", 72.0, 26.0, ButtonTone::Neutral).clicked() {
                        open_requested = true;
                    }
                    if small_button(ui, "Remove", 66.0, 26.0, ButtonTone::Danger).clicked() {
                        remove_requested = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("CTRL + O")
                                .font(theme::display(8.5))
                                .color(DIM),
                        );
                    });
                } else {
                    if small_button(ui, "Browse files", 94.0, 26.0, ButtonTone::Neutral).clicked() {
                        open_requested = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("or drop anywhere")
                                .font(theme::body(9.5))
                                .color(DIM),
                        );
                    });
                }
            });
        });

        if remove_requested {
            self.clear_source();
        }
        if open_requested {
            self.open_source_dialog(ctx);
        }
    }

    fn draw_canvas_card(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let mut padding = self.padding;
        let mut backdrop = self.backdrop;

        theme::glass_card(ui, CANVAS_INNER_HEIGHT, |ui| {
            theme::card_header(ui, "CANVAS", "Fit & spacing", |ui| {
                theme::badge(
                    ui,
                    &format!("{padding:.0}%"),
                    theme::accent_fill(20),
                    ACCENT_BRIGHT,
                );
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Safe padding")
                        .font(theme::display(10.5))
                        .color(TEXT_SOFT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Extra breathing room")
                            .font(theme::body(9.0))
                            .color(DIM),
                    );
                });
            });

            ui.add_space(3.0);
            let _ = padding_slider(ui, &mut padding);

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Preview backdrop")
                        .font(theme::display(10.0))
                        .color(TEXT_SOFT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let _ = backdrop_selector(ui, &mut backdrop);
                });
            });

            ui.add_space(6.0);
            theme::paint_transparency_note(ui, "Exports keep 32-bit transparency.");
        });

        if (padding - self.padding).abs() > f32::EPSILON {
            self.padding = padding;
            self.notice = None;
            self.armed_overwrite = None;
            self.schedule_preview_refresh(ctx);
        }
        self.backdrop = backdrop;
    }

    fn draw_preview_card(&mut self, ui: &mut Ui) {
        let info = self.source_summary();
        let selected = self.selected_sizes();
        let previews = self.previews.clone();
        let hero = previews.get(&256).cloned();
        let count = selected.len();
        let range = size_range_label(&selected);
        let title = if info.is_none() {
            "Your icon, at every size."
        } else if count == 0 {
            "Choose some frames."
        } else {
            "Ready for Windows."
        };
        let subtitle = info
            .as_ref()
            .map(|info| format!("{}  \u{00B7}  {}", info.name, info.native_size))
            .unwrap_or_else(|| "Drop an SVG or PNG anywhere to begin.".to_owned());
        let subtitle_color = if info.as_ref().is_some_and(|info| info.low_resolution) {
            AMBER
        } else {
            MUTED
        };

        theme::glass_card(ui, PREVIEW_INNER_HEIGHT, |ui| {
            theme::card_header(ui, "PREVIEW", "Live output", |ui| {
                theme::badge(
                    ui,
                    "LIVE",
                    theme::rgba(255, 255, 255, 12),
                    MINT,
                );
            });

            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                let (tile_rect, _) =
                    ui.allocate_exact_size(vec2(140.0, 140.0), Sense::hover());
                theme::preview_surface(ui.painter(), tile_rect, self.backdrop);

                if let Some(texture) = &hero {
                    theme::paint_texture(
                        ui.painter(),
                        texture.id(),
                        tile_rect.center(),
                        126.0,
                        255,
                    );
                } else {
                    theme::paint_file_placeholder(ui.painter(), tile_rect.center(), 0.78);
                }

                ui.add_space(18.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.set_width(ui.available_width());

                    ui.label(
                        RichText::new("WINDOWS SHELL")
                            .font(theme::display(8.5))
                            .color(ACCENT_BRIGHT)
                            .line_height(Some(11.0)),
                    );
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(title)
                            .font(theme::display(20.0))
                            .color(TEXT)
                            .line_height(Some(24.0)),
                    );
                    ui.add_space(2.0);
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), 14.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.add(
                                Label::new(
                                    RichText::new(subtitle)
                                        .font(theme::body(10.5))
                                        .color(subtitle_color)
                                        .line_height(Some(14.0)),
                                )
                                .truncate(),
                            );
                        },
                    );

                    ui.add_space(7.0);
                    divider(ui, ui.available_width());
                    ui.add_space(7.0);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 22.0;
                        stat_block(ui, "FRAMES", &format!("{count} / 10"), TEXT_SOFT);
                        stat_block(ui, "RANGE", &range, TEXT_SOFT);
                        stat_block(
                            ui,
                            "ALPHA",
                            "32-bit",
                            if info.is_some() { MINT } else { DIM },
                        );
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(
                                RichText::new("ACTUAL")
                                    .font(theme::display(7.5))
                                    .color(DIM)
                                    .line_height(Some(9.0)),
                            );
                            ui.label(
                                RichText::new("SIZE")
                                    .font(theme::display(7.5))
                                    .color(DIM)
                                    .line_height(Some(9.0)),
                            );
                            ui.add_space(3.0);
                            ui.label(
                                RichText::new("1:1")
                                    .font(theme::display(9.0))
                                    .color(MUTED),
                            );
                        });
                        for size in [16_u32, 32, 48] {
                            let (tile, response) =
                                ui.allocate_exact_size(vec2(50.0, 50.0), Sense::hover());
                            paint_actual_size_tile(
                                ui,
                                tile,
                                size,
                                previews.get(&size),
                                info.is_some(),
                            );
                            let _ = response.on_hover_text(format!(
                                "{size} px shown at actual size"
                            ));
                        }
                    });
                });
            });
        });
    }

    fn draw_sizes_card(&mut self, ui: &mut Ui) {
        let mut enabled = self.enabled;
        let old_enabled = self.enabled;
        let previews = self.previews.clone();

        theme::glass_card(ui, SIZES_INNER_HEIGHT, |ui| {
            let mut all = false;
            let mut none = false;
            let count = enabled.iter().filter(|selected| **selected).count();

            theme::card_header(ui, "WINDOWS FRAMES", "Embedded sizes", |ui| {
                theme::badge(
                    ui,
                    &format!("{count} / {}", ICON_SIZES.len()),
                    theme::accent_fill(18),
                    ACCENT_BRIGHT,
                );
                ui.add_space(3.0);
                if link_button(ui, "NONE").clicked() {
                    none = true;
                }
                if link_button(ui, "ALL").clicked() {
                    all = true;
                }
            });

            if all {
                enabled.fill(true);
            }
            if none {
                enabled.fill(false);
            }

            ui.add_space(7.0);
            let width = ui.available_width();
            let gap = 6.0;
            let columns = 5;
            let cell_width = ((width - gap * (columns - 1) as f32) / columns as f32)
                .floor()
                .max(60.0);

            egui::Grid::new("iconbv-size-grid")
                .num_columns(columns)
                .spacing(vec2(gap, gap))
                .show(ui, |ui| {
                    for (index, size) in ICON_SIZES.iter().enumerate() {
                        let response = draw_size_tile(
                            ui,
                            *size,
                            enabled[index],
                            previews.get(size),
                            cell_width,
                        );
                        if response.clicked() {
                            enabled[index] = !enabled[index];
                        }
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        if enabled != old_enabled {
            self.enabled = enabled;
            self.notice = None;
            self.armed_overwrite = None;
        }
    }

    fn draw_export_card(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let sizes = self.selected_sizes();
        let path_exists = self
            .output_path
            .as_ref()
            .is_some_and(|path| path.exists());
        let overwrite_ready = path_exists
            && self.output_path.as_ref() == self.armed_overwrite.as_ref();
        let enabled = self.source.is_some() && !sizes.is_empty();
        let path_text = self
            .output_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Choose a destination file".to_owned());
        let (status_color, status_text) = self.export_status(&sizes);
        let status_tooltip = status_text.clone();
        let mut browse_requested = false;
        let mut export_requested = false;

        theme::glass_card(ui, 44.0, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) =
                    ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
                paint_export_icon(ui.painter(), icon_rect, enabled);

                ui.add_space(12.0);
                let reserved = 190.0 + 82.0 + 24.0;
                let info_width = (ui.available_width() - reserved).max(140.0);
                ui.allocate_ui_with_layout(
                    vec2(info_width, 44.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new("DESTINATION")
                                .font(theme::display(8.0))
                                .color(DIM)
                                .line_height(Some(10.0)),
                        );
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), 17.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.add(
                                    Label::new(
                                        RichText::new(path_text.clone())
                                            .font(theme::display(11.5))
                                            .color(if self.output_path.is_some() {
                                                TEXT_SOFT
                                            } else {
                                                MUTED
                                            })
                                            .line_height(Some(16.0)),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(
                                    self.output_path
                                        .as_ref()
                                        .map(|path| path.display().to_string())
                                        .unwrap_or_else(|| path_text.clone()),
                                );
                            },
                        );
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(
                                Vec2::splat(6.0),
                                Sense::hover(),
                            );
                            theme::draw_dot(
                                ui.painter(),
                                dot_rect.center(),
                                2.4,
                                status_color,
                            );
                            let label = Label::new(
                                RichText::new(&status_text)
                                    .font(theme::body(9.5))
                                    .color(status_color)
                                    .line_height(Some(12.0)),
                            )
                            .truncate();
                            ui.add(label)
                                .on_hover_text(status_tooltip.clone());
                        });
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(
                        ui,
                        if overwrite_ready {
                            "Replace ICO"
                        } else {
                            "Export ICO"
                        },
                        enabled,
                        overwrite_ready,
                    )
                    .clicked()
                    {
                        export_requested = true;
                    }
                    ui.add_space(10.0);
                    if small_button(
                        ui,
                        "Browse",
                        82.0,
                        30.0,
                        ButtonTone::Neutral,
                    )
                    .clicked()
                    {
                        browse_requested = true;
                    }
                });
            });
        });

        if browse_requested {
            self.choose_output_path();
        }
        if export_requested {
            self.export_now();
        }

        let _ = ctx;
    }

    fn export_status(&self, sizes: &[u32]) -> (Color32, String) {
        if let Some(notice) = &self.notice {
            return (notice.kind.color(), notice.text.clone());
        }
        if self.source.is_none() {
            return (DIM, "Drop an SVG or PNG to get started.".to_owned());
        }
        if sizes.is_empty() {
            return (
                AMBER,
                "Select at least one frame to enable export.".to_owned(),
            );
        }
        (
            MUTED,
            format!(
                "{} frames \u{00B7} {} \u{00B7} transparent alpha",
                sizes.len(),
                size_range_label(sizes)
            ),
        )
    }
}

impl eframe::App for IconApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;
            ctx.request_repaint();
        }

        self.tick_preview_refresh(ctx);
        self.handle_shortcuts(ctx);
        self.handle_dropped_files(ctx);

        theme::paint_window_background(ctx);
        self.draw_titlebar(ctx);
        self.draw_content(ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}





#[derive(Clone, Copy, Eq, PartialEq)]
enum ButtonTone {
    Neutral,
    Danger,
}

fn small_button(
    ui: &mut Ui,
    text: &str,
    width: f32,
    height: f32,
    tone: ButtonTone,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    let danger = tone == ButtonTone::Danger;

    let fill = if danger {
        theme::mix(
            theme::rgba(255, 255, 255, 6),
            theme::rgba(255, 255, 255, 22),
            hover,
        )
    } else {
        theme::mix(
            theme::rgba(255, 255, 255, 6),
            theme::rgba(255, 255, 255, 12),
            hover,
        )
    };
    let stroke_color = if danger {
        theme::mix(theme::rgba(255, 255, 255, 16), theme::rgba(255, 255, 255, 110), hover)
    } else {
        theme::mix(theme::rgba(255, 255, 255, 15), theme::rgba(255, 255, 255, 95), hover)
    };
    let text_color = if danger {
        theme::mix(MUTED, ROSE, hover)
    } else {
        theme::mix(MUTED, TEXT, hover)
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, fill);
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0_f32, stroke_color),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        theme::display(10.0),
        text_color,
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn link_button(ui: &mut Ui, text: &str) -> Response {
    let font = theme::display(8.5);
    let width = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), MUTED)
        .size()
        .x
        + 12.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 18.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.1);

    if hover > 0.0 {
        ui.painter()
            .rect_filled(rect, 5.0, theme::rgba(255, 255, 255, (8.0 * hover) as u8));
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        theme::mix(DIM, TEXT, hover),
    );

    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn status_pill(ui: &mut Ui, text: &str, color: Color32) {
    let font = theme::display(8.5);
    let width = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color)
        .size()
        .x
        + 28.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());

    ui.painter().rect_filled(
        rect,
        13.0,
        theme::rgba(color.r(), color.g(), color.b(), 18),
    );
    ui.painter().rect_stroke(
        rect,
        13.0,
        Stroke::new(1.0_f32, theme::rgba(color.r(), color.g(), color.b(), 55)),
        StrokeKind::Inside,
    );
    theme::draw_dot(ui.painter(), pos2(rect.left() + 11.0, rect.center().y), 3.0, color);
    ui.painter().text(
        pos2(rect.left() + 19.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        theme::mix(color, TEXT, 0.25),
    );
}

fn padding_slider(ui: &mut Ui, value: &mut f32) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), vec2(rect.width() - 12.0, 4.0));
    let mut fraction = (*value / PADDING_MAX).clamp(0.0, 1.0);

    if response.is_pointer_button_down_on() || response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let next = ((pointer.x - track.left()) / track.width()).clamp(0.0, 1.0);
            let next_value = (next * PADDING_MAX).round();
            *value = next_value;
            fraction = next_value / PADDING_MAX;
        }
    }

    let painter = ui.painter();
    painter.rect_filled(track, 2.0, theme::rgba(255, 255, 255, 14));
    let fill = Rect::from_min_size(
        track.min,
        vec2(track.width() * fraction, track.height()),
    );
    if fill.width() > 0.5 {
        painter.rect_filled(fill, 2.0, ACCENT);
    }

    for tick in 0..=3 {
        let x = track.left() + track.width() * tick as f32 / 3.0;
        painter.line_segment(
            [pos2(x, track.center().y + 7.0), pos2(x, track.center().y + 10.0)],
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 22)),
        );
    }

    let knob = pos2(track.left() + track.width() * fraction, track.center().y);
    painter.circle_filled(knob, 7.5, theme::rgba(255, 255, 255, 26));
    painter.circle_filled(knob, 5.0, Color32::from_rgb(250, 250, 255));
    painter.circle_stroke(knob, 5.0, Stroke::new(1.0_f32, theme::rgba(15, 20, 30, 140)));

    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn backdrop_selector(ui: &mut Ui, backdrop: &mut theme::PreviewBackdrop) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(174.0, 27.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, theme::rgba(255, 255, 255, 5));
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 11)),
        StrokeKind::Inside,
    );

    let options = [
        theme::PreviewBackdrop::Grid,
        theme::PreviewBackdrop::Dark,
        theme::PreviewBackdrop::Light,
    ];
    let inner = rect.shrink(3.0);
    let cell_width = inner.width() / 3.0;
    let mut changed = false;

    for (index, option) in options.iter().enumerate() {
        let cell = Rect::from_min_size(
            pos2(inner.left() + cell_width * index as f32, inner.top()),
            vec2(cell_width - 2.0, inner.height()),
        );
        let response = ui.interact(
            cell,
            ui.id().with(("iconbv-backdrop", index)),
            Sense::click(),
        );
        let selected = *backdrop == *option;
        let hover = ui
            .ctx()
            .animate_bool_with_time(response.id, response.hovered(), 0.1);

        if selected {
            painter.rect_filled(cell, 6.0, theme::rgba(255, 255, 255, 32));
            painter.rect_stroke(
                cell,
                6.0,
                Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 80)),
                StrokeKind::Inside,
            );
        } else if hover > 0.0 {
            painter.rect_filled(
                cell,
                6.0,
                theme::rgba(255, 255, 255, (7.0 * hover) as u8),
            );
        }

        painter.text(
            cell.center(),
            Align2::CENTER_CENTER,
            option.label(),
            theme::display(8.5),
            if selected {
                ACCENT_BRIGHT
            } else {
                theme::mix(DIM, TEXT_SOFT, hover)
            },
        );

        if response.clicked() && !selected {
            *backdrop = *option;
            changed = true;
        }
    }

    changed
}

fn draw_dropzone(
    ui: &mut Ui,
    preview: Option<&TextureHandle>,
    info: Option<&SourceSummary>,
    dragging_files: bool,
) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 108.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered() || dragging_files, 0.14);

    let painter = ui.painter();
    let fill = theme::mix(
        theme::rgba(9, 13, 20, 180),
        theme::rgba(255, 255, 255, 14),
        hover * 0.7,
    );
    painter.rect_filled(rect, 14.0, fill);

    let border_color = theme::mix(
        theme::rgba(255, 255, 255, 18),
        theme::rgba(255, 255, 255, 100),
        hover,
    );
    if info.is_none() {
        theme::dashed_round_rect(
            painter,
            rect.shrink(0.5),
            14.0,
            Stroke::new(1.2_f32, border_color),
            5.0,
            5.0,
        );
    } else {
        painter.rect_stroke(
            rect,
            14.0,
            Stroke::new(1.0_f32, border_color),
            StrokeKind::Inside,
        );
    }

    if let (Some(texture), Some(_)) = (preview, info) {
        let tile = Rect::from_center_size(rect.center(), vec2(92.0, 92.0));
        theme::preview_surface(painter, tile, theme::PreviewBackdrop::Grid);
        theme::paint_texture(
            painter,
            texture.id(),
            tile.center(),
            84.0,
            255,
        );
    } else {
        let placeholder_center = rect.center() - vec2(0.0, 17.0);
        theme::paint_file_placeholder(painter, placeholder_center, 0.7);

        let title = if dragging_files {
            "Drop to load"
        } else {
            "Drop artwork here"
        };
        painter.text(
            pos2(rect.center().x, rect.center().y + 25.0),
            Align2::CENTER_CENTER,
            title,
            theme::display(12.0),
            if dragging_files { ACCENT_BRIGHT } else { TEXT_SOFT },
        );
        painter.text(
            pos2(rect.center().x, rect.center().y + 40.0),
            Align2::CENTER_CENTER,
            "SVG or PNG  \u{00B7}  click to browse",
            theme::body(9.0),
            DIM,
        );
    }

    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Click to choose an SVG or PNG source")
}

fn draw_size_tile(
    ui: &mut Ui,
    size: u32,
    selected: bool,
    texture: Option<&TextureHandle>,
    width: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 64.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id, response.hovered());
    let painter = ui.painter();

    let base = if selected {
        theme::rgba(255, 255, 255, 17)
    } else {
        theme::rgba(255, 255, 255, 4)
    };
    let fill = theme::mix(base, theme::rgba(255, 255, 255, 11), hover * 0.6);
    painter.rect_filled(rect, 12.0, fill);
    painter.rect_stroke(
        rect,
        12.0,
        Stroke::new(
            1.0_f32,
            if selected {
                theme::rgba(255, 255, 255, 110)
            } else {
                theme::mix(
                    theme::rgba(255, 255, 255, 12),
                    theme::rgba(255, 255, 255, 70),
                    hover,
                )
            },
        ),
        StrokeKind::Inside,
    );

    if selected {
    }

    let preview_center = pos2(rect.center().x, rect.top() + 25.0);
    if let Some(texture) = texture {
        theme::paint_texture(painter, texture.id(), preview_center, 24.0, 255);
    } else {
        painter.rect_filled(
            Rect::from_center_size(preview_center, Vec2::splat(24.0)),
            6.0,
            theme::rgba(255, 255, 255, 5),
        );
        painter.rect_stroke(
            Rect::from_center_size(preview_center, Vec2::splat(24.0)),
            6.0,
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 12)),
            StrokeKind::Inside,
        );
    }

    let checkbox = Rect::from_center_size(
        pos2(rect.right() - 11.0, rect.top() + 11.0),
        Vec2::splat(12.0),
    );
    if selected {
        painter.rect_filled(checkbox, 4.0, ACCENT);
        painter.line_segment(
            [
                checkbox.center() + vec2(-3.2, 0.0),
                checkbox.center() + vec2(-0.8, 2.4),
            ],
            Stroke::new(1.5_f32, Color32::from_rgb(15, 17, 26)),
        );
        painter.line_segment(
            [
                checkbox.center() + vec2(-0.8, 2.4),
                checkbox.center() + vec2(3.4, -2.4),
            ],
            Stroke::new(1.5_f32, Color32::from_rgb(15, 17, 26)),
        );
    } else {
        painter.rect_stroke(
            checkbox,
            4.0,
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 30)),
            StrokeKind::Inside,
        );
    }

    painter.text(
        pos2(rect.center().x, rect.top() + 53.0),
        Align2::CENTER_CENTER,
        format!("{size} px"),
        theme::display(9.5),
        if selected { TEXT } else { TEXT_SOFT },
    );

    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{} the {size} x {size} pixel frame",
            if selected { "Remove" } else { "Include" }
        ))
}

fn paint_actual_size_tile(
    ui: &Ui,
    rect: Rect,
    size: u32,
    texture: Option<&TextureHandle>,
    has_source: bool,
) {
    let painter = ui.painter();
    painter.rect_filled(rect, 10.0, theme::rgba(8, 12, 18, 220));
    painter.rect_stroke(
        rect,
        10.0,
        Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 14)),
        StrokeKind::Inside,
    );

    if let Some(texture) = texture {
        theme::paint_texture(
            painter,
            texture.id(),
            rect.center(),
            size as f32,
            255,
        );
    } else if !has_source {
        painter.rect_stroke(
            rect.shrink(8.0),
            5.0,
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 10)),
            StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "\u{00B7}",
            theme::display(9.0),
            DIM,
        );
    }
}

fn stat_block(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.label(
            RichText::new(label)
                .font(theme::display(7.5))
                .color(DIM)
                .line_height(Some(9.0)),
        );
        ui.label(
            RichText::new(value)
                .font(theme::display(10.5))
                .color(color)
                .line_height(Some(13.0)),
        );
    });
}

fn divider(ui: &mut Ui, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme::rgba(255, 255, 255, 13));
}

fn size_range_label(sizes: &[u32]) -> String {
    match sizes {
        [] => "\u{2014}".to_owned(),
        [only] => format!("{only} px"),
        [first, .., last] => format!("{first}\u{2013}{last} px"),
    }
}

fn window_control(ui: &mut Ui, control: WindowControl) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(33.0, 28.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.1);
    let close = control == WindowControl::Close;
    let hover_color = if close {
        theme::rgba(255, 255, 255, 45)
    } else {
        theme::rgba(255, 255, 255, 13)
    };
    if hover > 0.0 {
        ui.painter()
            .rect_filled(rect, 7.0, theme::mix(Color32::TRANSPARENT, hover_color, hover));
    }

    let color = if close {
        theme::mix(MUTED, ROSE, hover)
    } else {
        theme::mix(MUTED, TEXT, hover)
    };
    let painter = ui.painter();
    let center = rect.center();

    match control {
        WindowControl::Minimize => {
            painter.line_segment(
                [center + vec2(-5.0, 1.0), center + vec2(5.0, 1.0)],
                Stroke::new(1.2_f32, color),
            );
        }
        WindowControl::Close => {
            painter.line_segment(
                [center + vec2(-4.5, -4.5), center + vec2(4.5, 4.5)],
                Stroke::new(1.2_f32, color),
            );
            painter.line_segment(
                [center + vec2(4.5, -4.5), center + vec2(-4.5, 4.5)],
                Stroke::new(1.2_f32, color),
            );
        }
    }

    response.on_hover_cursor(CursorIcon::PointingHand)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum WindowControl {
    Minimize,
    Close,
}

fn primary_button(ui: &mut Ui, text: &str, enabled: bool, overwrite: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(190.0, 44.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered() && enabled, 0.12);
    let pressed = response.is_pointer_button_down_on();

    if enabled {
        let glow = if overwrite {
            AMBER
        } else {
            ACCENT
        };
        for layer in 0..3 {
            let alpha = ((5.0 - layer as f32) * (0.55 + hover * 0.45)) as u8;
            ui.painter().rect_filled(
                rect.expand(2.0 + layer as f32 * 2.5),
                13.0,
                theme::rgba(glow.r(), glow.g(), glow.b(), alpha),
            );
        }

        let (left, right) = if overwrite {
            (AMBER, ROSE)
        } else {
            (Color32::from_rgb(244, 245, 247), Color32::from_rgb(174, 178, 186))
        };
        let (left, right) = if pressed {
            (theme::mix(left, Color32::BLACK, 0.12), theme::mix(right, Color32::BLACK, 0.12))
        } else if hover > 0.0 {
            (
                theme::mix(left, Color32::WHITE, hover * 0.16),
                theme::mix(right, Color32::WHITE, hover * 0.16),
            )
        } else {
            (left, right)
        };

        theme::gradient_rounded_rect(ui.painter(), rect, 12, left, right);
        ui.painter().rect_stroke(
            rect,
            12.0,
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 80)),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            theme::display(13.0),
            Color32::from_rgb(12, 14, 20),
        );
    } else {
        ui.painter()
            .rect_filled(rect, 12.0, theme::rgba(255, 255, 255, 5));
        ui.painter().rect_stroke(
            rect,
            12.0,
            Stroke::new(1.0_f32, theme::rgba(255, 255, 255, 12)),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            theme::display(13.0),
            DIM,
        );
    }

    if enabled {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    }
}

fn paint_export_icon(painter: &egui::Painter, rect: Rect, enabled: bool) {
    painter.rect_filled(
        rect,
        11.0,
        theme::rgba(255, 255, 255, if enabled { 20 } else { 9 }),
    );
    painter.rect_stroke(
        rect,
        11.0,
        Stroke::new(
            1.0_f32,
            theme::rgba(255, 255, 255, if enabled { 55 } else { 26 }),
        ),
        StrokeKind::Inside,
    );

    let color = if enabled { ACCENT_BRIGHT } else { DIM };
    let center = rect.center();
    painter.line_segment(
        [center + vec2(0.0, -8.0), center + vec2(0.0, 4.0)],
        Stroke::new(1.4_f32, color),
    );
    painter.line_segment(
        [center + vec2(-4.0, 0.0), center + vec2(0.0, 4.0)],
        Stroke::new(1.4_f32, color),
    );
    painter.line_segment(
        [center + vec2(4.0, 0.0), center + vec2(0.0, 4.0)],
        Stroke::new(1.4_f32, color),
    );
    painter.line_segment(
        [center + vec2(-7.0, 9.0), center + vec2(7.0, 9.0)],
        Stroke::new(1.4_f32, color),
    );
}
