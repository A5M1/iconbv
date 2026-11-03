

use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame,
    InnerResponse, Layout, Margin, Painter, Pos2, Rect, RichText, Shadow, Shape, Stroke,
    StrokeKind, TextStyle, Ui, Vec2, pos2, vec2,
};





pub const TEXT: Color32 = Color32::from_rgb(246, 247, 249);
pub const TEXT_SOFT: Color32 = Color32::from_rgb(206, 209, 215);
pub const MUTED: Color32 = Color32::from_rgb(146, 150, 158);
pub const DIM: Color32 = Color32::from_rgb(101, 105, 113);
pub const ACCENT: Color32 = Color32::from_rgb(203, 207, 214);
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(238, 240, 244);
pub const CYAN: Color32 = Color32::from_rgb(186, 191, 199);
pub const MINT: Color32 = Color32::from_rgb(223, 226, 231);
pub const AMBER: Color32 = Color32::from_rgb(170, 175, 183);
pub const ROSE: Color32 = Color32::from_rgb(214, 217, 223);

pub const CARD_RADIUS: u8 = 16;
pub const CARD_PADDING: f32 = 16.0;

pub fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8| a as f32 + (b as f32 - a as f32) * t;
    rgba(
        lerp(a.r(), b.r()).round() as u8,
        lerp(a.g(), b.g()).round() as u8,
        lerp(a.b(), b.b()).round() as u8,
        lerp(a.a(), b.a()).round() as u8,
    )
}

pub fn accent_fill(alpha: u8) -> Color32 {
    rgba(255, 255, 255, alpha)
}





pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("iconbv-display".into()))
}

pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_style(ctx);
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    #[cfg(target_os = "windows")]
    {
        let system_fonts = [
            ("iconbv-ui", "C:\\Windows\\Fonts\\segoeui.ttf"),
            ("iconbv-display", "C:\\Windows\\Fonts\\seguisb.ttf"),
        ];

        for (name, path) in system_fonts {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
            }
        }
    }

    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    if fonts.font_data.contains_key("iconbv-ui") {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "iconbv-ui".to_owned());
    }

    let mut display_family = Vec::new();
    if fonts.font_data.contains_key("iconbv-display") {
        display_family.push("iconbv-display".to_owned());
    }
    if fonts.font_data.contains_key("iconbv-ui") {
        display_family.push("iconbv-ui".to_owned());
    }
    display_family.extend(fallbacks);
    fonts
        .families
        .insert(FontFamily::Name("iconbv-display".into()), display_family);

    ctx.set_fonts(fonts);
}

fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.15;
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(12.0, 7.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Name("iconbv-display".into())),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.5, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(12.5, FontFamily::Name("iconbv-display".into())),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(10.5, FontFamily::Proportional),
    );

    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(MUTED);
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = rgba(19, 20, 23, 250);
    visuals.window_stroke = Stroke::new(1.0_f32, rgba(255, 255, 255, 22));
    visuals.window_corner_radius = CornerRadius::same(16);
    visuals.menu_corner_radius = CornerRadius::same(10);
    visuals.window_shadow = Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: rgba(0, 0, 0, 150),
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 22,
        spread: 0,
        color: rgba(0, 0, 0, 140),
    };
    visuals.selection.bg_fill = accent_fill(75);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_BRIGHT);
    visuals.hyperlink_color = CYAN;
    visuals.faint_bg_color = rgba(255, 255, 255, 5);
    visuals.extreme_bg_color = rgba(6, 9, 14, 245);
    visuals.widgets.noninteractive.bg_fill = rgba(255, 255, 255, 4);
    visuals.widgets.noninteractive.weak_bg_fill = rgba(255, 255, 255, 4);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, rgba(255, 255, 255, 14));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_SOFT);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(9);
    visuals.widgets.inactive.bg_fill = rgba(255, 255, 255, 7);
    visuals.widgets.inactive.weak_bg_fill = rgba(255, 255, 255, 7);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, rgba(255, 255, 255, 13));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_SOFT);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(9);
    visuals.widgets.hovered.bg_fill = rgba(255, 255, 255, 13);
    visuals.widgets.hovered.weak_bg_fill = rgba(255, 255, 255, 10);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, rgba(255, 255, 255, 105));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(9);
    visuals.widgets.active.bg_fill = rgba(255, 255, 255, 18);
    visuals.widgets.active.weak_bg_fill = rgba(255, 255, 255, 14);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, rgba(255, 255, 255, 150));
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.active.corner_radius = CornerRadius::same(9);
    visuals.widgets.open = visuals.widgets.hovered;
    style.visuals = visuals;

    ctx.set_style(style);
}





pub fn paint_window_background(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    let maximized = ctx
        .input(|input| input.viewport().maximized)
        .unwrap_or(false);
    let radius = if maximized { 0 } else { 16 };

    let painter = ctx.layer_painter(egui::LayerId::background());
    gradient_rounded_rect(
        &painter,
        rect,
        radius,
        Color32::from_rgb(14, 15, 17),
        Color32::from_rgb(7, 8, 9),
    );

    let glow_layer = painter.with_clip_rect(rect.shrink(1.0));
    radial_glow(
        &glow_layer,
        pos2(rect.left() + rect.width() * 0.24, rect.top() + 40.0),
        rect.width() * 0.48,
        rgba(255, 255, 255, 16),
    );
    radial_glow(
        &glow_layer,
        pos2(rect.right() - 60.0, rect.top() + rect.height() * 0.34),
        rect.width() * 0.34,
        rgba(255, 255, 255, 9),
    );
    radial_glow(
        &glow_layer,
        pos2(rect.left() + rect.width() * 0.54, rect.bottom() + 130.0),
        rect.width() * 0.40,
        rgba(255, 255, 255, 10),
    );


    let top = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + 116.0));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(top.left_top(), rgba(255, 255, 255, 12));
    mesh.colored_vertex(top.right_top(), rgba(255, 255, 255, 3));
    mesh.colored_vertex(top.left_bottom(), Color32::TRANSPARENT);
    mesh.colored_vertex(top.right_bottom(), Color32::TRANSPARENT);
    mesh.add_triangle(0, 2, 1);
    mesh.add_triangle(1, 2, 3);
    painter.add(Shape::mesh(mesh));

    painter.rect_stroke(
        rect.shrink(0.5),
        radius,
        Stroke::new(1.0_f32, rgba(255, 255, 255, 22)),
        StrokeKind::Inside,
    );
}





pub fn glass_card<R>(
    ui: &mut Ui,
    min_content_height: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let frame = Frame::new()
        .fill(rgba(19, 20, 23, 238))
        .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 17)))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(CARD_PADDING as i8))
        .shadow(Shadow {
            offset: [0, 10],
            blur: 28,
            spread: 0,
            color: rgba(0, 0, 0, 85),
        });

    frame.show(ui, |ui| {
        let width = ui.available_width();
        ui.set_width(width);
        ui.set_min_height(min_content_height);
        ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
        add_contents(ui)
    })
}

pub fn card_header(
    ui: &mut Ui,
    kicker: &str,
    title: &str,
    add_accessory: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(
                RichText::new(kicker)
                    .font(display(9.5))
                    .color(DIM)
                    .line_height(Some(12.0)),
            );
            ui.label(
                RichText::new(title)
                    .font(display(16.5))
                    .color(TEXT)
                    .line_height(Some(21.0)),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            add_accessory(ui);
        });
    });
}

pub fn badge(ui: &mut Ui, text: &str, tint: Color32, text_color: Color32) -> egui::Response {
    Frame::new()
        .fill(tint)
        .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 8)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .font(display(9.0))
                    .color(text_color)
                    .line_height(Some(11.0)),
            );
        })
        .response
}





pub fn gradient_rounded_rect(
    painter: &Painter,
    rect: Rect,
    radius: u8,
    left: Color32,
    right: Color32,
) {
    if !rect.is_positive() {
        return;
    }

    let radius = f32::from(radius).min(rect.width() * 0.5).min(rect.height() * 0.5);
    let path = rounded_perimeter(rect, radius, 8, 0.0);
    if path.len() < 3 {
        return;
    }

    let width = rect.width().max(1.0);
    let color_at = |point: Pos2| {
        let t = ((point.x - rect.left()) / width).clamp(0.0, 1.0);
        mix(left, right, t)
    };

    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), mix(left, right, 0.5));
    for point in &path {
        mesh.colored_vertex(*point, color_at(*point));
    }
    let count = path.len() as u32;
    for index in 0..count {
        mesh.add_triangle(0, index + 1, (index + 1) % count + 1);
    }
    painter.add(Shape::mesh(mesh));
}

pub fn radial_glow(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    if radius <= 1.0 || color.a() == 0 {
        return;
    }

    let edge = Color32::from_rgba_premultiplied(0, 0, 0, 0);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color);
    let segments = 64;
    for index in 0..segments {
        let angle = std::f32::consts::TAU * index as f32 / segments as f32;
        mesh.colored_vertex(
            center + vec2(angle.cos(), angle.sin()) * radius,
            edge,
        );
    }
    for index in 0..segments as u32 {
        mesh.add_triangle(0, 1 + index, 1 + (index + 1) % segments as u32);
    }
    painter.add(Shape::mesh(mesh));
}

fn rounded_perimeter(rect: Rect, radius: f32, arc_steps: usize, edge_step: f32) -> Vec<Pos2> {
    let radius = radius.max(0.0).min(rect.width() * 0.5).min(rect.height() * 0.5);
    let mut points = Vec::new();
    let corners = [
        (pos2(rect.left() + radius, rect.top() + radius), 180.0),
        (pos2(rect.right() - radius, rect.top() + radius), 270.0),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (pos2(rect.left() + radius, rect.bottom() - radius), 90.0),
    ];

    for (center, start_degrees) in corners {
        for step in 0..=arc_steps {
            let angle =
                (start_degrees + 90.0 * step as f32 / arc_steps as f32).to_radians();
            points.push(center + vec2(angle.cos(), angle.sin()) * radius);
        }
    }

    if edge_step > 0.0 {
        let mut with_edges = Vec::new();
        for (index, point) in points.iter().enumerate() {
            with_edges.push(*point);
            if index + 1 < points.len() {
                let next = points[index + 1];
                let distance = point.distance(next);
                if distance > edge_step * 1.5 {
                    let steps = (distance / edge_step).ceil() as usize;
                    for step in 1..steps {
                        let t = step as f32 / steps as f32;
                        with_edges.push(point.lerp(next, t));
                    }
                }
            }
        }
        points = with_edges;
    }

    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        if first.distance(*last) < 0.1 && points.len() > 1 {
            points.pop();
        }
    }
    points
}

pub fn dashed_round_rect(
    painter: &Painter,
    rect: Rect,
    radius: f32,
    stroke: Stroke,
    dash: f32,
    gap: f32,
) {
    let points = rounded_perimeter(rect, radius, 5, 5.0);
    if points.len() < 2 {
        return;
    }

    let period = (dash + gap).max(1.0);
    let mut phase = 0.0;
    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];
        let length = start.distance(end);
        if length <= f32::EPSILON {
            continue;
        }

        let mut consumed = 0.0;
        while consumed < length {
            let in_dash = phase < dash;
            let remaining = if in_dash {
                (dash - phase).max(0.05)
            } else {
                (period - phase).max(0.05)
            };
            let step = remaining.min(length - consumed);
            if in_dash {
                let begin = start.lerp(end, consumed / length);
                let finish = start.lerp(end, (consumed + step) / length);
                painter.line_segment([begin, finish], stroke);
            }
            consumed += step;
            phase = (phase + step) % period;
        }
    }
}

pub fn checkerboard(painter: &Painter, rect: Rect, cell: f32) {
    const DARK: Color32 = Color32::from_rgb(21, 22, 25);
    const LIGHT: Color32 = Color32::from_rgb(26, 27, 30);

    let clipped = painter.with_clip_rect(rect);
    clipped.rect_filled(rect, 0.0, DARK);
    let columns = (rect.width() / cell).ceil() as usize;
    let rows = (rect.height() / cell).ceil() as usize;
    for row in 0..rows {
        for column in 0..columns {
            if (row + column) % 2 == 0 {
                continue;
            }
            let min = rect.min + vec2(column as f32 * cell, row as f32 * cell);
            let max = pos2(
                (min.x + cell).min(rect.right()),
                (min.y + cell).min(rect.bottom()),
            );
            let cell_rect = Rect::from_min_max(min, max);
            if cell_rect.is_positive() {
                clipped.rect_filled(cell_rect, 0.0, LIGHT);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewBackdrop {
    Grid,
    Dark,
    Light,
}

impl PreviewBackdrop {
    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

pub fn preview_surface(painter: &Painter, rect: Rect, backdrop: PreviewBackdrop) {
    let radius = 14.0;
    let base = match backdrop {
        PreviewBackdrop::Grid => Color32::from_rgb(17, 18, 21),
        PreviewBackdrop::Dark => Color32::from_rgb(11, 12, 14),
        PreviewBackdrop::Light => Color32::from_rgb(235, 236, 238),
    };
    painter.rect_filled(rect, radius, base);

    if backdrop == PreviewBackdrop::Grid {
        let checker = rect.shrink(5.0);
        if checker.is_positive() {
            let clip = painter.with_clip_rect(checker);
            checkerboard(&clip, checker, 10.0);
        }
    }

    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0_f32, rgba(255, 255, 255, if backdrop == PreviewBackdrop::Light { 35 } else { 16 })),
        StrokeKind::Inside,
    );
}

pub fn paint_texture(
    painter: &Painter,
    texture_id: egui::TextureId,
    center: Pos2,
    size: f32,
    alpha: u8,
) {
    let rect = Rect::from_center_size(center, Vec2::splat(size));
    painter.image(
        texture_id,
        rect,
        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
        Color32::from_white_alpha(alpha),
    );
}

pub fn paint_logo_mark(painter: &Painter, rect: Rect) {


    painter.rect_filled(
        rect,
        10.0,
        Color32::from_rgb(245, 246, 248),
    );

    let content = rect.shrink(rect.width() * 0.22);
    let gap = rect.width() * 0.09;
    let tile = (content.width() - gap) * 0.5;
    let tile_radius = (tile * 0.25).max(1.0);
    let points = [
        content.min,
        pos2(content.min.x + tile + gap, content.min.y),
        pos2(content.min.x, content.min.y + tile + gap),
        pos2(content.min.x + tile + gap, content.min.y + tile + gap),
    ];
    for (index, point) in points.into_iter().enumerate() {
        let alpha = if index == 3 { 150 } else { 230 };
        painter.rect_filled(
            Rect::from_min_size(point, Vec2::splat(tile)),
            tile_radius,
            rgba(23, 24, 27, alpha),
        );
    }
}

pub fn paint_file_placeholder(painter: &Painter, center: Pos2, scale: f32) {
    radial_glow(
        painter,
        center,
        74.0 * scale,
        rgba(255, 255, 255, 20),
    );

    let back_rect = Rect::from_center_size(center + vec2(9.0, -5.0) * scale, vec2(50.0, 60.0) * scale);
    painter.rect_filled(back_rect, 10.0 * scale, rgba(255, 255, 255, 24));
    painter.rect_stroke(
        back_rect,
        10.0 * scale,
        Stroke::new(1.0_f32, rgba(255, 255, 255, 55)),
        StrokeKind::Inside,
    );

    let front_rect = Rect::from_center_size(center + vec2(-6.0, 4.0) * scale, vec2(54.0, 64.0) * scale);
    painter.rect_filled(front_rect, 11.0 * scale, Color32::from_rgb(27, 28, 31));
    painter.rect_stroke(
        front_rect,
        11.0 * scale,
        Stroke::new(1.0_f32, rgba(255, 255, 255, 32)),
        StrokeKind::Inside,
    );

    let art = front_rect.shrink(12.0 * scale);
    let sun = pos2(art.left() + art.width() * 0.66, art.top() + art.height() * 0.28);
    painter.circle_filled(sun, 4.5 * scale, rgba(255, 255, 255, 215));
    let mountain = vec![
        pos2(art.left() + 2.0 * scale, art.bottom() - 4.0 * scale),
        pos2(art.left() + art.width() * 0.38, art.top() + art.height() * 0.44),
        pos2(art.left() + art.width() * 0.60, art.bottom() - 12.0 * scale),
        pos2(art.left() + art.width() * 0.78, art.top() + art.height() * 0.60),
        pos2(art.right() - 1.0 * scale, art.bottom() - 4.0 * scale),
    ];
    painter.add(Shape::convex_polygon(
        mountain,
        rgba(201, 204, 209, 215),
        Stroke::NONE,
    ));
}

pub fn draw_dot(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    painter.circle_filled(center, radius, color);
}

pub fn paint_transparency_note(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), egui::Sense::hover());
        let painter = ui.painter();
        let center = rect.center();
        painter.rect_stroke(
            Rect::from_center_size(center, vec2(8.0, 8.0)),
            2.0,
            Stroke::new(1.0_f32, DIM),
            StrokeKind::Inside,
        );
        painter.line_segment(
            [center + vec2(-2.0, 1.0), center + vec2(-0.5, 2.5)],
            Stroke::new(1.2_f32, DIM),
        );
        painter.line_segment(
            [center + vec2(-0.5, 2.5), center + vec2(2.5, -2.0)],
            Stroke::new(1.2_f32, DIM),
        );
        ui.label(
            RichText::new(text)
                .font(body(10.5))
                .color(DIM)
                .line_height(Some(14.0)),
        );
    });
}
