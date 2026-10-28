use eframe::egui::{self, *};
pub use egui_phosphor::regular as icons;

pub const BG: Color32 = Color32::from_rgb(24, 30, 41);
pub const SIDE: Color32 = Color32::from_rgb(31, 38, 50);
pub const FG: Color32 = Color32::from_rgb(246, 248, 252);
pub const MUTED: Color32 = Color32::from_rgb(180, 191, 210);
pub const ACCENT: Color32 = Color32::from_rgb(157, 200, 250);
pub const LINE: Color32 = Color32::from_rgba_premultiplied(24, 24, 24, 24);
pub const SELECT: Color32 = Color32::from_rgba_premultiplied(32, 48, 70, 110);

pub fn setup(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let windows_fonts = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:/Windows".into())
        .join("Fonts");
    let regular = std::fs::read(windows_fonts.join("segoeui.ttf"))
        .unwrap_or_else(|_| include_bytes!("../assets/Inter-Regular.ttf").to_vec());
    let semibold = std::fs::read(windows_fonts.join("seguisb.ttf"))
        .unwrap_or_else(|_| include_bytes!("../assets/Inter-Semibold.ttf").to_vec());
    fonts
        .font_data
        .insert("UI".into(), FontData::from_owned(regular).into());
    fonts
        .font_data
        .insert("Display".into(), FontData::from_owned(semibold).into());
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "UI".into());
    fonts
        .families
        .insert(FontFamily::Name("Display".into()), vec!["Display".into()]);
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    if let Ok(bytes) = std::fs::read(windows_fonts.join("SegoeIcons.ttf")) {
        fonts
            .font_data
            .insert("Fluent".into(), FontData::from_owned(bytes).into());
        fonts
            .families
            .insert(FontFamily::Name("Icons".into()), vec!["Fluent".into()]);
        ctx.data_mut(|d| d.insert_temp(Id::new("fluent-icons"), true));
    }
    ctx.set_fonts(fonts);
    let mut style = Style {
        visuals: Visuals::dark(),
        ..Default::default()
    };
    style.visuals.override_text_color = Some(FG);
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = SIDE;
    style.visuals.extreme_bg_color = BG;
    style.visuals.selection.bg_fill = SELECT;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.inactive.bg_fill = Color32::from_white_alpha(12);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
    style.visuals.widgets.hovered.bg_fill = Color32::from_white_alpha(28);
    style.visuals.widgets.active.bg_fill = SELECT;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(10.0, 6.0);
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(13.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(11.0));
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0, FontFamily::Name("Display".into())),
    );
    ctx.set_style(style);
}

pub fn text(
    painter: &Painter,
    pos: Pos2,
    align: Align2,
    value: impl ToString,
    size: f32,
    color: Color32,
) {
    painter.text(pos, align, value, FontId::proportional(size), color);
}
pub fn ellipsis(painter: &Painter, rect: Rect, value: &str, size: f32, color: Color32) {
    label(painter, rect, value, FontId::proportional(size), color);
}
pub fn heading(painter: &Painter, rect: Rect, value: &str, size: f32) {
    label(
        painter,
        rect,
        value,
        FontId::new(size, FontFamily::Name("Display".into())),
        FG,
    );
}
fn label(painter: &Painter, rect: Rect, value: &str, font: FontId, color: Color32) {
    let mut job = egui::text::LayoutJob::simple(value.into(), font, color, rect.width().max(1.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = painter.layout_job(job);
    painter.galley(
        pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}
pub fn icon(painter: &Painter, pos: Pos2, value: &str, size: f32, color: Color32) {
    let fluent = painter
        .ctx()
        .data(|d| d.get_temp::<bool>(Id::new("fluent-icons")).unwrap_or(false));
    // Segoe Fluent Icons ships with Windows 11; it is loaded from the OS, not redistributed.
    let mapped = match value {
        icons::PLAY => "\u{E768}",
        icons::PAUSE => "\u{E769}",
        icons::SKIP_BACK => "\u{E892}",
        icons::SKIP_FORWARD => "\u{E893}",
        icons::SHUFFLE => "\u{E8B1}",
        icons::REPEAT => "\u{E8EE}",
        icons::REPEAT_ONCE => "\u{E8ED}",
        icons::SPEAKER_HIGH => "\u{E767}",
        icons::SPEAKER_SLASH => "\u{E74F}",
        icons::SPEAKER_LOW => "\u{E993}",
        icons::FOLDER_SIMPLE => "\u{E8B7}",
        icons::FOLDER_SIMPLE_PLUS => "\u{E8F4}",
        icons::CARET_DOWN => "\u{E70D}",
        icons::X => "\u{E8BB}",
        icons::MINUS => "\u{E921}",
        icons::SQUARE => "\u{E922}",
        icons::MUSIC_NOTES => "\u{EC4F}",
        icons::LIST => "\u{E8FD}",
        icons::DOTS_THREE => "\u{E712}",
        icons::MAGNIFYING_GLASS => "\u{E721}",
        icons::ARROWS_CLOCKWISE => "\u{E72C}",
        icons::ARROW_LEFT => "\u{E72B}",
        icons::CHECK => "\u{E73E}",
        icons::PLUS => "\u{E710}",
        _ => "",
    };
    if fluent && !mapped.is_empty() {
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            mapped,
            FontId::new(size, FontFamily::Name("Icons".into())),
            color,
        );
    } else {
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            value,
            FontId::proportional(size),
            color,
        );
    }
}
pub fn icon_button(
    ui: &mut Ui,
    rect: Rect,
    value: &str,
    name: &str,
    active: bool,
    primary: bool,
) -> Response {
    let response = ui.interact(rect, ui.id().with(name), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), name));
    if response.hovered() || response.has_focus() || active {
        ui.painter().rect_filled(
            rect.shrink(1.0),
            5.0,
            if active {
                SELECT
            } else {
                Color32::from_white_alpha(20)
            },
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            5.0,
            Stroke::new(1.0_f32, ACCENT),
            StrokeKind::Inside,
        );
    }
    icon(
        ui.painter(),
        rect.center(),
        value,
        if primary { 28.0 } else { 18.0 },
        if !ui.is_enabled() {
            MUTED.gamma_multiply(0.4)
        } else if active {
            ACCENT
        } else {
            FG
        },
    );
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(name)
}
pub fn slider(ui: &mut Ui, rect: Rect, value: &mut f64, max: f64, name: &str) -> Response {
    let mut response = ui.interact(rect, ui.id().with(name), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::slider(ui.is_enabled(), *value, name));
    if ui.is_enabled() {
        if let Some(pointer) = response.interact_pointer_pos() {
            *value = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * max;
            response.mark_changed();
        }
        if response.has_focus() {
            let step = if max > 1.0 { 5.0 } else { 0.05 };
            if ui.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp)) {
                *value = (*value + step).min(max);
                response.mark_changed();
            }
            if ui.input(|i| i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown)) {
                *value = (*value - step).max(0.0);
                response.mark_changed();
            }
        }
    }
    let y = rect.center().y;
    let end = rect.left() + ((*value / max.max(0.001)).clamp(0.0, 1.0) as f32 * rect.width());
    ui.painter().line_segment(
        [pos2(rect.left(), y), pos2(rect.right(), y)],
        Stroke::new(3.0_f32, Color32::from_white_alpha(60)),
    );
    if ui.is_enabled() {
        ui.painter().line_segment(
            [pos2(rect.left(), y), pos2(end, y)],
            Stroke::new(3.0_f32, ACCENT),
        );
        ui.painter().circle_filled(
            pos2(end, y),
            if response.hovered() || response.dragged() {
                5.5
            } else {
                4.5
            },
            ACCENT,
        );
    }
    if response.has_focus() {
        ui.painter()
            .rect_stroke(rect, 3.0, Stroke::new(1.0_f32, ACCENT), StrokeKind::Inside);
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(name)
}

pub fn equalizer_slider(ui: &mut Ui, rect: Rect, gain: &mut f32, name: &str) -> Response {
    let mut response = ui.interact(rect, ui.id().with(name), Sense::click_and_drag());
    let rail = rect.shrink2(vec2(0.0, 6.0));
    if ui.is_enabled() {
        if let Some(pointer) = response.interact_pointer_pos() {
            *gain =
                (12.0 - ((pointer.y - rail.top()) / rail.height()).clamp(0.0, 1.0) * 24.0).round();
            response.mark_changed();
        }
        if response.has_focus() {
            let delta = ui.input(|i| {
                i.num_presses(Key::ArrowUp) as f32 - i.num_presses(Key::ArrowDown) as f32
            });
            if delta != 0.0 {
                *gain = (*gain + delta).clamp(-12.0, 12.0);
                response.mark_changed();
            }
        }
        if response.double_clicked() {
            *gain = 0.0;
            response.mark_changed();
        }
    }
    response.widget_info(|| WidgetInfo::slider(ui.is_enabled(), *gain as f64, name));
    let x = rail.center().x;
    let y = rail.top() + (12.0 - *gain) / 24.0 * rail.height();
    let color = if ui.is_enabled() {
        ACCENT
    } else {
        MUTED.gamma_multiply(0.4)
    };
    ui.painter().line_segment(
        [pos2(x, rail.top()), pos2(x, rail.bottom())],
        Stroke::new(3.0_f32, Color32::from_white_alpha(50)),
    );
    ui.painter().line_segment(
        [
            pos2(x - 5.0, rail.center().y),
            pos2(x + 5.0, rail.center().y),
        ],
        Stroke::new(1.0_f32, MUTED.gamma_multiply(0.5)),
    );
    ui.painter().line_segment(
        [pos2(x, rail.center().y), pos2(x, y)],
        Stroke::new(3.0_f32, color),
    );
    ui.painter().circle_filled(
        pos2(x, y),
        if response.dragged() || response.hovered() {
            6.0
        } else {
            5.0
        },
        color,
    );
    if response.has_focus() {
        ui.painter()
            .rect_stroke(rect, 3.0, Stroke::new(1.0_f32, ACCENT), StrokeKind::Inside);
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(name)
}
