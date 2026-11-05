//! Retângulos compartilhados pelo desenho GDI e pelos controles Win32.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl UiRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub const fn right(self) -> i32 {
        self.x + self.w
    }
    pub const fn bottom(self) -> i32 {
        self.y + self.h
    }
}

const PAD: i32 = 24;
const GAP: i32 = 18;
const TOP: i32 = 145;
const COMBO_POPUP_HEIGHT: i32 = 230;

const SHOW_GRAPHS: u32 = 1211;
const SHOW_BARS: u32 = 1212;
const SHOW_SUBTITLES: u32 = 1213;
const OPEN_EDITOR: u32 = 1400;
const BEHIND: u32 = 1005;
const VISIBILITY: u32 = 1002;
const LOCK: u32 = 1003;
const RECOVER: u32 = 1010;
const ARRANGE_GRID: u32 = 1007;
const ARRANGE_COLUMN: u32 = 1009;
const GROUP_MOVE: u32 = 1011;
const SNAP: u32 = 1006;
const APPLY_LAYOUT: u32 = 1514;
const LAYOUT_MODE: u32 = 1510;
const LAYOUT_MONITOR: u32 = 1512;
const LAYOUT_GAP: u32 = 1513;
const ANCHOR_START: u32 = 1560;
const MONITOR_DISK: u32 = 1550;
const MONITOR_ADAPTER: u32 = 1551;
const MONITOR_INTERVAL: u32 = 1552;
const MONITOR_REFRESH: u32 = 1553;
const STYLE: u32 = 1200;
const THEME: u32 = 1201;
const ACCENT: u32 = 1202;
const SIZE: u32 = 1203;
const OPACITY: u32 = 1204;
const UI_THEME: u32 = 1214;

#[derive(Clone, Copy, Debug)]
pub struct UiLayout {
    pub page: usize,
    pub stacked: bool,
    pub scroll_width: i32,
    pub scroll_height: i32,
    pub left: UiRect,
    pub right: UiRect,
    pub widget_columns: usize,
    pub widget_card_width: i32,
    pub view_w: i32,
    pub compact: bool,
}

impl UiLayout {
    pub fn new(width: i32, height: i32, page: usize) -> Self {
        let w = width.max(330);
        let h = height.max(1);
        let compact = w < 540;
        let stacked = w < 810 && matches!(page, 0 | 2 | 4);
        let inner = w - PAD * 2;
        let left_width = if stacked {
            inner
        } else {
            (inner - GAP) * 55 / 100
        };
        let right_width = if stacked {
            inner
        } else {
            inner - GAP - left_width
        };

        let (left_min, right_min) = match page {
            0 => (if compact { 610 } else { 455 }, 485),
            2 => (465, 460),
            4 => (435, 465),
            _ => (460, 460),
        };
        let available_height = h - TOP - 8;
        let left_height = if stacked {
            left_min
        } else {
            available_height.max(left_min)
        };
        let right_height = if stacked {
            right_min
        } else {
            available_height.max(right_min)
        };
        let left = UiRect::new(PAD, TOP, left_width, left_height);
        let right = UiRect::new(
            if stacked { PAD } else { left.right() + GAP },
            if stacked { left.bottom() + GAP } else { TOP },
            right_width,
            right_height,
        );

        let widget_columns = if w >= 1180 {
            3
        } else if w >= 580 {
            2
        } else {
            1
        };

        let widget_card_width = (inner - GAP * (widget_columns as i32 - 1)) / widget_columns as i32;
        let widget_rows = (6 + widget_columns - 1) / widget_columns;
        let widget_bottom = TOP + widget_rows as i32 * (195 + GAP) + 20;
        let behavior_bottom = if w < 690 {
            TOP + 4 * 156 + 35
        } else {
            TOP + 4 * 112 + 40
        };

        let content_bottom = match page {
            1 => widget_bottom,
            3 => behavior_bottom,
            _ => left.bottom().max(right.bottom()) + 8,
        };

        let scroll_height = content_bottom.max(left.bottom()).max(right.bottom()).max(h);
        Self {
            page,
            stacked,
            scroll_width: w,
            scroll_height,
            left,
            right,
            widget_columns,
            widget_card_width,
            view_w: w,
            compact,
        }
    }

    pub fn anchor_grid(self) -> UiRect {
        let panel = self.right;
        let width = (panel.w - 38).min(285);
        UiRect::new(panel.x + (panel.w - width) / 2, panel.y + 65, width, 171)
    }

    pub fn anchor_tile(self, index: usize) -> UiRect {
        let grid = self.anchor_grid();
        let gap = 8;
        let edge = 10;
        let tile_w = (grid.w - 2 * edge - 2 * gap) / 3;
        let tile_h = (grid.h - 2 * edge - 2 * gap) / 3;
        let content_w = 3 * tile_w + 2 * gap;
        let content_h = 3 * tile_h + 2 * gap;
        let start_x = grid.x + (grid.w - content_w) / 2;
        let start_y = grid.y + (grid.h - content_h) / 2;
        let col = (index % 3) as i32;
        let row = (index / 3) as i32;
        UiRect::new(
            start_x + col * (tile_w + gap),
            start_y + row * (tile_h + gap),
            tile_w,
            tile_h,
        )
    }

    pub fn widget_rect(self, index: usize) -> UiRect {
        let col = index % self.widget_columns;
        let row = index / self.widget_columns;
        UiRect::new(
            PAD + col as i32 * (self.widget_card_width + GAP),
            TOP + row as i32 * (195 + GAP),
            self.widget_card_width,
            195,
        )
    }

    pub fn behavior_panel(self) -> UiRect {
        let height = if self.view_w < 690 {
            4 * 156 + 18
        } else {
            4 * 112 + 24
        };
        UiRect::new(PAD, TOP, self.view_w - 2 * PAD, height)
    }

    fn combo(x: i32, y: i32, w: i32) -> UiRect {
        UiRect::new(x, y, w.max(1), COMBO_POPUP_HEIGHT)
    }

    pub fn appearance_preview(self) -> UiRect {
        let r = self.right;
        UiRect::new(r.x + 17, r.y + 68, (r.w - 34).max(1), 215)
    }

    pub fn appearance_editor_button(self) -> UiRect {
        let r = self.right;
        UiRect::new(r.x + 18, r.y + 307, (r.w - 36).max(1), 46)
    }

    pub fn appearance_theme_label(self) -> UiRect {
        let r = self.right;
        UiRect::new(r.x + 18, r.y + 375, (r.w - 36).max(1), 25)
    }

    pub fn appearance_theme_choice(self) -> UiRect {
        let r = self.right;
        Self::combo(r.x + 18, r.y + 410, r.w - 36)
    }

    fn appearance_rect(self, id: u32, base: UiRect) -> UiRect {
        let p = self.left;
        let x = p.x + 18;
        let content = p.w - 36;
        let split = (content - 15) / 2;
        let right_x = x + split + 15;
        if id == OPEN_EDITOR {
            return self.appearance_editor_button();
        }

        if id == UI_THEME {
            return self.appearance_theme_choice();
        }
        if self.compact {
            return match id {
                STYLE => Self::combo(x, p.y + 62, content),
                THEME => Self::combo(x, p.y + 145, content),
                SIZE => Self::combo(x, p.y + 225, content),
                ACCENT => Self::combo(x, p.y + 305, content),
                OPACITY => Self::combo(x, p.y + 385, content),
                SHOW_GRAPHS => UiRect::new(x, p.y + 449, content, 31),
                SHOW_BARS => UiRect::new(x, p.y + 485, content, 31),
                SHOW_SUBTITLES => UiRect::new(x, p.y + 521, content, 31),
                _ => match base.y {
                    176 => UiRect::new(x, p.y + 32, content, 26),
                    268 => {
                        UiRect::new(x, p.y + (if base.x < 270 { 115 } else { 195 }), content, 26)
                    }
                    354 => {
                        UiRect::new(x, p.y + (if base.x < 270 { 275 } else { 355 }), content, 26)
                    }
                    _ => base,
                },
            };
        }
        match id {
            STYLE => Self::combo(x, p.y + 68, content),
            THEME => Self::combo(x, p.y + 170, split),
            SIZE => Self::combo(right_x, p.y + 170, content - split - 15),
            ACCENT => Self::combo(x, p.y + 269, split),
            OPACITY => Self::combo(right_x, p.y + 269, content - split - 15),
            SHOW_GRAPHS => UiRect::new(x, p.y + 338, split, 31),
            SHOW_BARS => UiRect::new(right_x, p.y + 338, content - split - 15, 31),
            SHOW_SUBTITLES => UiRect::new(x, p.y + 375, content, 31),
            _ => match base.y {
                176 => UiRect::new(x, p.y + 36, content, 27),
                268 => UiRect::new(
                    if base.x < 270 { x } else { right_x },
                    p.y + 139,
                    if base.x < 270 {
                        split
                    } else {
                        content - split - 15
                    },
                    27,
                ),
                354 => UiRect::new(
                    if base.x < 270 { x } else { right_x },
                    p.y + 238,
                    if base.x < 270 {
                        split
                    } else {
                        content - split - 15
                    },
                    27,
                ),
                _ => base,
            },
        }
    }

    fn layout_rect(self, id: u32, base: UiRect) -> UiRect {
        let l = self.left;
        let r = self.right;
        let lx = l.x + 18;
        let lw = l.w - 36;
        if (ARRANGE_GRID..=ARRANGE_COLUMN).contains(&id) {
            let index = (id - ARRANGE_GRID) as i32;
            let gap = 9;
            let cell = (lw - gap * 2) / 3;
            return UiRect::new(lx + index * (cell + gap), l.y + 61, cell, 77);
        }
        if (ANCHOR_START..ANCHOR_START + 9).contains(&id) {
            return self.anchor_tile((id - ANCHOR_START) as usize);
        }
        match id {
            LAYOUT_GAP => Self::combo(lx, l.y + 204, lw),
            LAYOUT_MONITOR => Self::combo(lx, l.y + 292, lw),
            LAYOUT_MODE => Self::combo(lx, l.y + 380, lw),
            GROUP_MOVE => UiRect::new(r.x + 18, r.y + 254, r.w - 36, 32),
            SNAP => UiRect::new(r.x + 18, r.y + 296, r.w - 36, 32),
            APPLY_LAYOUT => UiRect::new(r.x + 18, r.y + 352, r.w - 36, 46),
            _ => match base.y {
                296 => UiRect::new(lx, l.y + 174, lw, 27),
                379 => UiRect::new(lx, l.y + 263, lw, 27),
                464 => UiRect::new(lx, l.y + 351, lw, 27),
                578 => UiRect::new(r.x + 18, r.y + 412, r.w - 36, 36),
                _ => base,
            },
        }
    }

    fn monitor_rect(self, id: u32, base: UiRect) -> UiRect {
        let l = self.left;
        let r = self.right;
        match id {
            MONITOR_DISK => Self::combo(l.x + 18, l.y + 148, l.w - 36),
            MONITOR_ADAPTER => Self::combo(l.x + 18, l.y + 275, l.w - 36),
            MONITOR_INTERVAL => Self::combo(r.x + 18, r.y + 148, r.w - 36),
            MONITOR_REFRESH => UiRect::new(r.x + 18, r.y + 223, r.w - 36, 46),
            _ => match base.y {
                227 => {
                    let panel = if base.x >= 470 { r } else { l };
                    UiRect::new(panel.x + 18, panel.y + 108, panel.w - 36, 29)
                }
                357 => UiRect::new(l.x + 18, l.y + 236, l.w - 36, 29),
                _ => base,
            },
        }
    }

    fn behavior_rect(self, id: u32, base: UiRect) -> UiRect {
        let p = self.behavior_panel();
        let step = if self.view_w < 690 { 156 } else { 112 };
        if id == BEHIND {
            return UiRect::new(p.right() - 108, p.y + 19, 82, 34);
        }
        let row = match id {
            LOCK => 1,
            VISIBILITY => 2,
            RECOVER => 3,
            _ => return base,
        };
        let (x, y, w) = if self.view_w < 690 {
            (p.x + 20, p.y + row * step + 90, p.w - 40)
        } else {
            (p.right() - 266, p.y + row * step + 20, 242)
        };
        UiRect::new(x, y, w, 44)
    }

    pub fn control_rect(self, id: u32, base: UiRect) -> UiRect {
        match self.page {
            0 => self.appearance_rect(id, base),
            1 => {
                if (1100..1106).contains(&id) {
                    let card = self.widget_rect((id - 1100) as usize);
                    return UiRect::new(card.x + 16, card.y + 12, card.w - 32, 34);
                }
                if (1300..1306).contains(&id) {
                    let card = self.widget_rect((id - 1300) as usize);
                    let wide = card.w >= 300;
                    let x = if wide {
                        card.right() - 158
                    } else {
                        card.x + 16
                    };
                    let width = if wide { 140 } else { card.w - 32 };
                    return Self::combo(x, card.y + 124, width);
                }
                base
            }
            2 => self.layout_rect(id, base),
            3 => self.behavior_rect(id, base),
            4 => self.monitor_rect(id, base),
            _ => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_pages_and_panel_geometry() {
        for w in [330, 375, 520, 580, 760, 809, 810, 890, 915, 1250, 1920] {
            for page in 0..5 {
                let l = UiLayout::new(w, 560, page);
                assert!(l.scroll_width >= w);
                assert!(l.left.x >= 0 && l.left.right() <= l.scroll_width);
                assert!(l.right.x >= 0 && l.right.right() <= l.scroll_width);
                if l.stacked {
                    assert!(l.right.y >= l.left.bottom());
                } else {
                    assert!(l.right.x >= l.left.right());
                }
                assert!(l.scroll_height >= l.left.bottom());
                assert!(l.scroll_height >= l.right.bottom());
            }
        }
    }

    #[test]
    fn appearance_controls_never_leave_panels() {
        for w in [330, 390, 520, 620, 809, 810, 890, 915, 1200] {
            let l = UiLayout::new(w, 560, 0);
            let ids = [
                STYLE,
                THEME,
                SIZE,
                ACCENT,
                OPACITY,
                SHOW_GRAPHS,
                SHOW_BARS,
                SHOW_SUBTITLES,
            ];
            for id in ids {
                let r = l.control_rect(id, UiRect::new(50, 200, 200, 31));
                assert!(
                    r.x >= l.left.x + 8 && r.right() <= l.left.right() - 8,
                    "id={id}, w={w}, rect={r:?}, panel={:?}",
                    l.left
                );

                assert!(r.y >= l.left.y && r.y < l.left.bottom());
            }
            let edit = l.control_rect(OPEN_EDITOR, UiRect::new(641, 493, 228, 46));
            assert!(edit.x >= l.right.x && edit.right() <= l.right.right());
            let theme = l.control_rect(UI_THEME, UiRect::new(641, 655, 228, 230));
            assert!(theme.x >= l.right.x + 8 && theme.right() <= l.right.right() - 8);
            assert!(theme.y > edit.bottom());
            assert!(theme.y + 37 + 16 <= l.right.bottom());
        }
    }

    #[test]
    fn appearance_preview_button_and_theme_do_not_overlap() {
        // Até no modo empilhado/compacto, o rótulo tem espaço próprio.
        for w in [330, 375, 390, 510, 539, 540, 760, 809, 810, 915, 1200, 1920] {
            for h in [380, 560, 720, 1000] {
                let layout = UiLayout::new(w, h, 0);
                let panel = layout.right;
                let preview = layout.appearance_preview();
                let button = layout.appearance_editor_button();
                let label = layout.appearance_theme_label();
                let combo = layout.appearance_theme_choice();

                assert!(preview.bottom() + 20 <= button.y, "w={w}, h={h}");
                assert!(button.bottom() + 16 <= label.y, "w={w}, h={h}");
                assert!(label.bottom() + 8 <= combo.y, "w={w}, h={h}");
                assert!(combo.y + 37 + 16 <= panel.bottom(), "w={w}, h={h}");
                for r in [preview, button, label, combo] {
                    assert!(
                        r.x >= panel.x + 12 && r.right() <= panel.right() - 12,
                        "w={w}, h={h}, rect={r:?}"
                    );
                }
                assert_eq!(
                    button,
                    layout.control_rect(OPEN_EDITOR, UiRect::new(886, 493, 228, 46))
                );
                assert_eq!(
                    combo,
                    layout.control_rect(UI_THEME, UiRect::new(886, 655, 228, 230))
                );
            }
        }
    }

    #[test]
    fn labels_and_fields_have_separate_y_ranges() {
        let l = UiLayout::new(915, 580, 4);
        let disk_label = l.control_rect(0, UiRect::new(51, 227, 350, 23));
        let disk_combo = l.control_rect(MONITOR_DISK, UiRect::new(50, 263, 362, 230));
        let adapter_label = l.control_rect(0, UiRect::new(51, 357, 350, 23));
        let adapter_combo = l.control_rect(MONITOR_ADAPTER, UiRect::new(50, 393, 362, 230));
        assert!(disk_label.bottom() + 6 < disk_combo.y);
        assert!(adapter_label.bottom() + 6 < adapter_combo.y);
    }

    #[test]
    fn layout_actions_and_checkboxes_fit_the_right_panel() {
        for w in [330, 520, 809, 810, 915] {
            let l = UiLayout::new(w, 610, 2);
            for id in [GROUP_MOVE, SNAP, APPLY_LAYOUT] {
                let r = l.control_rect(id, UiRect::new(500, 405, 300, 31));
                assert!(r.x >= l.right.x + 8 && r.right() <= l.right.right() - 8);
                assert!(r.y >= l.right.y && r.bottom() <= l.right.bottom());
            }
        }
    }

    #[test]
    fn anchor_grid_tiles_are_centered_and_never_overlap() {
        for w in [330, 375, 520, 760, 809, 810, 915, 1180, 1920] {
            let layout = UiLayout::new(w, 560, 2);
            let grid = layout.anchor_grid();
            assert!(grid.x >= layout.right.x + 8);
            assert!(grid.right() <= layout.right.right() - 8);
            assert!(grid.y >= layout.right.y + 48);
            assert!(grid.bottom() < layout.right.y + 244);
            let mut items = Vec::new();
            for i in 0..9 {
                let tile = layout.anchor_tile(i);
                let positioned =
                    layout.control_rect(ANCHOR_START + i as u32, UiRect::new(0, 0, 52, 42));
                assert_eq!(tile, positioned, "w={w}, index={i}");
                assert!(tile.x >= grid.x + 8 && tile.right() <= grid.right() - 8);
                assert!(tile.y >= grid.y + 8 && tile.bottom() <= grid.bottom() - 8);
                assert!(tile.w >= 40 && tile.h >= 40);
                for prior in &items {
                    let prior: &UiRect = prior;
                    assert!(
                        tile.x >= prior.right()
                            || tile.right() <= prior.x
                            || tile.y >= prior.bottom()
                            || tile.bottom() <= prior.y,
                        "w={w}, tile {i} overlaps {prior:?}"
                    );
                }
                items.push(tile);
            }
            let a = items[0];
            let b = items[2];
            let c = items[6];
            let d = items[8];
            let left_pad = a.x - grid.x;
            let right_pad = grid.right() - b.right();
            let top_pad = a.y - grid.y;
            let bottom_pad = grid.bottom() - d.bottom();
            assert!(
                (left_pad - right_pad).abs() <= 1,
                "w={w}, horizontal padding"
            );
            assert!((top_pad - bottom_pad).abs() <= 1, "w={w}, vertical padding");
            assert_eq!(a.x, c.x);
            assert_eq!(b.x, d.x);
        }
    }

    #[test]
    fn widgets_reflow_without_clipping_last_row() {
        for w in [330, 500, 780, 1180, 1900] {
            let l = UiLayout::new(w, 450, 1);
            for i in 0..6 {
                let r = l.widget_rect(i);
                assert!(r.x >= 0 && r.right() <= l.scroll_width);
                assert!(r.bottom() < l.scroll_height);
            }
        }
    }
}
