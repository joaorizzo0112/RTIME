use windows_sys::Win32::Foundation::RECT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditorRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl EditorRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub fn translated(self, shift_y: i32) -> Self {
        Self {
            y: self.y + shift_y,
            ..self
        }
    }
    pub fn to_rect(self) -> RECT {
        RECT {
            left: self.x,
            top: self.y,
            right: self.x + self.w,
            bottom: self.y + self.h,
        }
    }
    #[cfg(test)]
    fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

pub const SECTION_TITLES: [&str; 5] = [
    "Estilo e cores",
    "Conteúdo",
    "Aparência",
    "Texto e progresso",
    "Atualização e desempenho",
];
pub const FIELD_TITLES: [&str; 11] = [
    "Widget",
    "Estilo visual",
    "Cor de destaque",
    "Gráfico",
    "Números",
    "Borda",
    "Opacidade",
    "Barra de progresso",
    "Texto secundário",
    "Intervalo de atualização",
    "Perfil rápido (global)",
];

#[derive(Clone, Debug)]
pub struct EditorLayout {
    #[cfg_attr(not(test), allow(dead_code))]
    pub stacked: bool,
    pub heading: EditorRect,
    pub intro: EditorRect,
    pub selector_label: EditorRect,
    pub sections: [EditorRect; 5],
    pub combos: [EditorRect; 11],
    pub previous: EditorRect,
    pub next: EditorRect,
    pub reset: EditorRect,
    pub close: EditorRect,
    pub footer_hint: EditorRect,
    pub preview_panel: EditorRect,
    pub preview_heading: EditorRect,
    pub preview_help: EditorRect,
    pub main_preview: EditorRect,
    pub network_heading: EditorRect,
    pub network_preview: EditorRect,
    pub activity_heading: EditorRect,
    pub activity_preview: EditorRect,
    pub copy_all: EditorRect,
    pub group_toggle: EditorRect,
    pub arrange: EditorRect,
    pub preview_note: EditorRect,
    pub content_height: i32,
}
impl EditorLayout {
    pub fn new(view_width: i32) -> Self {
        let w = view_width.max(380);
        let pad = 24;
        let gap = 21;
        let stacked = w < 920;
        let left_width = if stacked {
            w - 2 * pad
        } else {
            ((w - 2 * pad - gap) * 47 / 100).clamp(390, 630)
        };
        let left_x = pad;
        let right_x = if stacked {
            pad
        } else {
            left_x + left_width + gap
        };
        let right_width = if stacked {
            w - 2 * pad
        } else {
            w - pad - right_x
        };
        let field_gap = 16;
        let half = (left_width - 36 - field_gap) / 2;
        let first_x = left_x + 18;
        let second_x = first_x + half + field_gap;
        let selector_start = left_x + 18;
        let arrow_width = 92.min((left_width - 80) / 3);
        let selector_combo_w = (left_width - 36 - 2 * arrow_width - 16).max(120);
        let selector_y = 133;
        let widget_combo = EditorRect::new(selector_start, selector_y, selector_combo_w, 38);
        let previous = EditorRect::new(
            widget_combo.x + widget_combo.w + 8,
            selector_y,
            arrow_width,
            38,
        );
        let next = EditorRect::new(previous.x + previous.w + 8, selector_y, arrow_width, 38);
        let sections =
            std::array::from_fn(|i| EditorRect::new(left_x, 184 + i as i32 * 115, left_width, 108));
        let mut combos = [EditorRect::new(0, 0, 0, 0); 11];
        combos[0] = widget_combo;
        for (i, section) in sections.iter().enumerate() {
            combos[1 + 2 * i] = EditorRect::new(first_x, section.y + 66, half, 34);
            combos[2 + 2 * i] = EditorRect::new(second_x, section.y + 66, half, 34);
        }
        let last_section_bottom = sections[4].y + sections[4].h;
        let footer_hint =
            EditorRect::new(left_x + 8, last_section_bottom + 14, left_width - 12, 22);
        let footer_button_width = (left_width - 12) / 2;
        let footer_buttons_y = footer_hint.y + 29;
        let reset = EditorRect::new(left_x, footer_buttons_y, footer_button_width, 40);
        let close = EditorRect::new(
            left_x + footer_button_width + 12,
            footer_buttons_y,
            left_width - footer_button_width - 12,
            40,
        );
        let preview_y = if stacked { close.y + close.h + 24 } else { 184 };
        let preview_panel = EditorRect::new(right_x, preview_y, right_width, 666);
        let inset = 24;
        let preview_heading =
            EditorRect::new(right_x + inset, preview_y + 18, right_width - 2 * inset, 25);
        let preview_help =
            EditorRect::new(right_x + inset, preview_y + 47, right_width - 2 * inset, 34);
        let main_preview = EditorRect::new(
            right_x + inset,
            preview_y + 91,
            right_width - 2 * inset,
            162,
        );
        let network_heading = EditorRect::new(
            right_x + inset,
            preview_y + 263,
            right_width - 2 * inset,
            19,
        );
        let network_preview = EditorRect::new(
            right_x + inset,
            preview_y + 285,
            right_width - 2 * inset,
            94,
        );
        let activity_heading = EditorRect::new(
            right_x + inset,
            preview_y + 387,
            right_width - 2 * inset,
            19,
        );
        let activity_preview = EditorRect::new(
            right_x + inset,
            preview_y + 409,
            right_width - 2 * inset,
            82,
        );
        let button_w = right_width - 2 * inset;
        let copy_all = EditorRect::new(right_x + inset, preview_y + 505, button_w, 38);
        let group_toggle = EditorRect::new(right_x + inset, preview_y + 548, button_w, 38);
        let arrange = EditorRect::new(right_x + inset, preview_y + 591, button_w, 38);
        let preview_note = EditorRect::new(right_x + inset, preview_y + 634, button_w, 23);
        let content_height = (preview_note.y + preview_note.h + 28).max(close.y + close.h + 28);
        Self {
            stacked,
            heading: EditorRect::new(left_x, 26, left_width, 37),
            intro: EditorRect::new(left_x, 67, left_width, 28),
            selector_label: EditorRect::new(left_x + 2, 103, left_width - 4, 23),
            sections,
            combos,
            previous,
            next,
            reset,
            close,
            footer_hint,
            preview_panel,
            preview_heading,
            preview_help,
            main_preview,
            network_heading,
            network_preview,
            activity_heading,
            activity_preview,
            copy_all,
            group_toggle,
            arrange,
            preview_note,
            content_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_layout_separates_two_panes() {
        for width in [920, 1024, 1160, 1420, 1920] {
            let layout = EditorLayout::new(width);
            assert!(!layout.stacked);
            assert!(!layout.sections[0].overlaps(layout.preview_panel));
            assert!(!layout.combos[1].overlaps(layout.combos[2]));
            assert!(layout.preview_panel.x + layout.preview_panel.w <= width);
            assert!(layout.content_height > layout.preview_note.y);
        }
    }
    #[test]
    fn narrow_layout_places_preview_below_editor() {
        for width in [420, 620, 700, 919] {
            let layout = EditorLayout::new(width);
            assert!(layout.stacked);
            assert!(layout.preview_panel.y >= layout.close.y + layout.close.h);
            assert!(layout.preview_panel.x + layout.preview_panel.w <= width);
            assert!(!layout.combos[1].overlaps(layout.combos[2]));
        }
    }
    #[test]
    fn section_titles_and_labels_are_visually_separated() {
        for width in [420, 620, 919, 920, 1190, 1600] {
            let layout = EditorLayout::new(width);
            for (i, section) in layout.sections.iter().enumerate() {
                let title_bottom = section.y + 11 + 28;
                for combo in [layout.combos[1 + 2 * i], layout.combos[2 + 2 * i]] {
                    let label_top = combo.y - 25;
                    assert!(label_top > title_bottom, "título e rótulo sobrepostos: {i}");
                    assert!(combo.y + combo.h + 5 <= section.y + section.h);
                }
            }
            assert!(layout.footer_hint.y >= layout.sections[4].y + layout.sections[4].h);
            assert!(layout.content_height >= layout.close.y + layout.close.h);
        }
    }
    #[test]
    fn scroll_keeps_action_buttons_at_distinct_positions() {
        for width in [620, 919, 920, 1190, 1600] {
            let layout = EditorLayout::new(width);
            for shift in [0, 15, 60, 135, 450, 800] {
                let actions = [layout.copy_all, layout.group_toggle, layout.arrange]
                    .map(|r| r.translated(-shift));
                for pair in actions.windows(2) {
                    assert!(
                        pair[0].y + pair[0].h <= pair[1].y,
                        "botões da prévia sobrepostos: {width}px, rolagem {shift}px"
                    );
                }
            }
        }
    }

    #[test]
    fn all_editor_fields_remain_inside_their_section() {
        for width in [420, 720, 919, 920, 1190, 1600] {
            let layout = EditorLayout::new(width);
            for (i, section) in layout.sections.iter().enumerate() {
                for field in [layout.combos[2 * i + 1], layout.combos[2 * i + 2]] {
                    assert!(field.x >= section.x && field.x + field.w <= section.x + section.w);
                    assert!(field.y >= section.y && field.y + field.h <= section.y + section.h);
                }
            }
        }
    }
}
