use super::*;

const INDICATOR: Color32 = Color32::from_rgb(96, 205, 255);

/// The same closed control and flyout treatment for both player selectors.
pub fn fluent_select_button(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    value: &str,
    accessible_name: &str,
    leading_icon: Option<&str>,
) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| {
        WidgetInfo::labeled(WidgetType::ComboBox, ui.is_enabled(), accessible_name)
    });
    let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response));
    let fill = if response.is_pointer_button_down_on() || open {
        Color32::from_white_alpha(10)
    } else if response.hovered() {
        Color32::from_white_alpha(22)
    } else {
        Color32::from_white_alpha(14)
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0_f32, Color32::from_white_alpha(16)),
        StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [
            pos2(rect.left() + 4.0, rect.bottom() - 0.5),
            pos2(rect.right() - 4.0, rect.bottom() - 0.5),
        ],
        Stroke::new(1.0_f32, Color32::from_white_alpha(38)),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, INDICATOR),
            StrokeKind::Inside,
        );
    }
    let left = if let Some(symbol) = leading_icon {
        icon(
            ui.painter(),
            pos2(rect.left() + 18.0, rect.center().y),
            symbol,
            20.0,
            FG,
        );
        rect.left() + 36.0
    } else {
        rect.left() + 12.0
    };
    ellipsis(
        ui.painter(),
        Rect::from_min_max(
            pos2(left, rect.top()),
            pos2(rect.right() - 32.0, rect.bottom()),
        ),
        value,
        14.0,
        FG,
    );
    icon(
        ui.painter(),
        pos2(rect.right() - 16.0, rect.center().y),
        if open {
            icons::CARET_UP
        } else {
            icons::CARET_DOWN
        },
        10.0,
        FG,
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if !open {
        response.clone().on_hover_text(value);
    }
    response
}

pub fn fluent_popup(response: &Response) -> Popup<'_> {
    Popup::menu(response)
        .width(response.rect.width())
        .gap(4.0)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .frame(
            Frame::new()
                .fill(Color32::from_rgb(46, 42, 51))
                .corner_radius(8)
                .inner_margin(6)
                .stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(80)))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 5],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(80),
                }),
        )
        .style(|style: &mut Style| {
            style.spacing.item_spacing = vec2(0.0, 3.0);
        })
}

pub fn fluent_option(ui: &mut Ui, selected: bool, label: &str) -> Response {
    popup_row(ui, Some(selected), label)
}

pub fn fluent_menu_action(ui: &mut Ui, label: &str) -> Response {
    popup_row(ui, None, label)
}

fn popup_row(ui: &mut Ui, selected: Option<bool>, label: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    response.widget_info(|| match selected {
        Some(selected) => WidgetInfo::selected(
            WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        ),
        None => WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label),
    });
    if selected == Some(true) || response.hovered() || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            4.0,
            Color32::from_white_alpha(if response.is_pointer_button_down_on() {
                12
            } else if response.hovered() {
                24
            } else {
                18
            }),
        );
    }
    if selected == Some(true) {
        ui.painter().rect_filled(
            Rect::from_center_size(pos2(rect.left() + 2.0, rect.center().y), vec2(3.0, 16.0)),
            2.0,
            INDICATOR,
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            4.0,
            Stroke::new(1.0_f32, INDICATOR),
            StrokeKind::Inside,
        );
    }
    ellipsis(
        ui.painter(),
        Rect::from_min_max(
            pos2(rect.left() + 12.0, rect.top()),
            pos2(rect.right() - 10.0, rect.bottom()),
        ),
        label,
        14.0,
        if ui.is_enabled() {
            FG
        } else {
            MUTED.gamma_multiply(0.5)
        },
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}
