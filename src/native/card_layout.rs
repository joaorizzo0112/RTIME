//! Posições lógicas e hierarquia tipográfica dos cartões.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl BoxRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub const fn right(self) -> i32 {
        self.x + self.w
    }
    pub const fn bottom(self) -> i32 {
        self.y + self.h
    }
    #[cfg(test)]
    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricKind {
    Normal,
    Network,
    Activity,
    Processes,
}

#[derive(Clone, Copy, Debug)]
pub struct CardTextLayout {
    pub title: BoxRect,
    pub value: BoxRect,
    pub subtitle: BoxRect,
    pub secondary_label: BoxRect,
    pub secondary_value: BoxRect,
    pub graph: BoxRect,
    pub progress: BoxRect,
    pub cycle_label: BoxRect,
    pub process_heading: BoxRect,
    pub process_rows: [BoxRect; 3],
    pub process_row_count: usize,
    pub process_row_px: i32,
    pub title_px: i32,
    pub value_px: i32,
    pub subtitle_px: i32,
    pub secondary_value_px: i32,
    pub secondary_label_px: i32,
}

impl CardTextLayout {
    pub fn new(size: u32, kind: MetricKind, font_preference: u32) -> Self {
        let (w, h) = match size {
            0 => (164, 128),
            2 => (232, 182),
            3 => (440, 140),
            _ => (180, 150),
        };
        let compact = size == 0;
        let expanded = size == 2;
        let horizontal = size == 3;
        let margin = if expanded { 18 } else { 16 };
        let title = BoxRect::new(
            margin,
            if expanded { 12 } else { 9 },
            w - 2 * margin,
            if expanded { 26 } else { 24 },
        );
        let progress = BoxRect::new(
            margin,
            h - if expanded { 15 } else { 14 },
            w - 2 * margin,
            6,
        );
        let mut layout = match kind {
            MetricKind::Processes => {
                
                let (value, subtitle, heading, rows, row_count, row_px) = if horizontal {
                    (
                        BoxRect::new(margin, 42, 188, 45),
                        BoxRect::new(margin, 87, 188, 21),
                        BoxRect::new(228, 31, 190, 18),
                        [
                            BoxRect::new(228, 53, 192, 19),
                            BoxRect::new(228, 76, 192, 19),
                            BoxRect::new(228, 99, 192, 19),
                        ],
                        3,
                        13,
                    )
                } else if expanded {
                    (
                        BoxRect::new(margin, 42, w - 2 * margin, 43),
                        BoxRect::new(margin, 87, w - 2 * margin, 21),
                        BoxRect::new(margin, 110, w - 2 * margin, 16),
                        [
                            BoxRect::new(margin, 127, w - 2 * margin, 16),
                            BoxRect::new(margin, 144, w - 2 * margin, 16),
                            BoxRect::new(margin, 161, w - 2 * margin, 16),
                        ],
                        3,
                        12,
                    )
                } else if compact {
                    (
                        BoxRect::new(margin, 35, w - 2 * margin, 34),
                        BoxRect::new(margin, 71, w - 2 * margin, 18),
                        BoxRect::new(margin, 91, w - 2 * margin, 14),
                        [
                            BoxRect::new(margin, 106, w - 2 * margin, 16),
                            BoxRect::new(0, 0, 0, 0),
                            BoxRect::new(0, 0, 0, 0),
                        ],
                        1,
                        11,
                    )
                } else {
                    (
                        BoxRect::new(margin, 37, w - 2 * margin, 38),
                        BoxRect::new(margin, 77, w - 2 * margin, 18),
                        BoxRect::new(margin, 96, w - 2 * margin, 14),
                        [
                            BoxRect::new(margin, 111, w - 2 * margin, 14),
                            BoxRect::new(margin, 127, w - 2 * margin, 14),
                            BoxRect::new(0, 0, 0, 0),
                        ],
                        2,
                        11,
                    )
                };
                Self {
                    title,
                    value,
                    subtitle,
                    secondary_label: BoxRect::new(0, 0, 0, 0),
                    secondary_value: BoxRect::new(0, 0, 0, 0),
                    graph: BoxRect::new(0, 0, 0, 0),
                    progress: BoxRect::new(0, 0, 0, 0),
                    cycle_label: BoxRect::new(0, 0, 0, 0),
                    process_heading: heading,
                    process_rows: rows,
                    process_row_count: row_count,
                    process_row_px: row_px,
                    title_px: if expanded || horizontal {
                        16
                    } else if compact {
                        13
                    } else {
                        14
                    },
                    value_px: if expanded || horizontal {
                        39
                    } else if compact {
                        29
                    } else {
                        35
                    },
                    subtitle_px: if compact { 12 } else { 14 },
                    secondary_value_px: 0,
                    secondary_label_px: 0,
                }
            }
            MetricKind::Network if horizontal => Self {
                title,
                value: BoxRect::new(margin, 40, 183, 47),
                subtitle: BoxRect::new(margin, 88, 183, 21),
                secondary_label: BoxRect::new(213, 43, 210, 22),
                secondary_value: BoxRect::new(213, 65, 110, 35),
                graph: BoxRect::new(331, 75, 92, 38),
                progress,
                cycle_label: BoxRect::new(0, 0, 0, 0),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 16,
                value_px: 37,
                subtitle_px: 14,
                secondary_value_px: 26,
                secondary_label_px: 13,
            },
            MetricKind::Network if compact => Self {
                title,
                value: BoxRect::new(margin, 34, w - 2 * margin, 31),
                subtitle: BoxRect::new(margin, 65, w - 2 * margin, 14),
                secondary_label: BoxRect::new(margin, 79, 89, 14),
                secondary_value: BoxRect::new(margin, 93, 89, 20),
                graph: BoxRect::new(113, 87, 35, 24),
                progress,
                cycle_label: BoxRect::new(0, 0, 0, 0),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 13,
                value_px: 26,
                subtitle_px: 12,
                secondary_value_px: 17,
                secondary_label_px: 11,
            },
            MetricKind::Network if expanded => Self {
                title,
                value: BoxRect::new(margin, 42, w - 2 * margin, 49),
                subtitle: BoxRect::new(margin, 91, w - 2 * margin, 19),
                secondary_label: BoxRect::new(margin, 111, 122, 20),
                secondary_value: BoxRect::new(margin, 132, 122, 29),
                graph: BoxRect::new(150, 116, 63, 44),
                progress,
                cycle_label: BoxRect::new(0, 0, 0, 0),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 16,
                value_px: 38,
                subtitle_px: 15,
                secondary_value_px: 24,
                secondary_label_px: 13,
            },
            MetricKind::Network => Self {
                title,
                value: BoxRect::new(margin, 39, w - 2 * margin, 36),
                subtitle: BoxRect::new(margin, 75, w - 2 * margin, 17),
                secondary_label: BoxRect::new(margin, 93, 94, 18),
                secondary_value: BoxRect::new(margin, 111, 94, 23),
                graph: BoxRect::new(117, 103, 46, 29),
                progress,
                cycle_label: BoxRect::new(0, 0, 0, 0),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 14,
                value_px: 31,
                subtitle_px: 13,
                secondary_value_px: 20,
                secondary_label_px: 11,
            },
            MetricKind::Activity | MetricKind::Normal if horizontal => Self {
                title,
                value: BoxRect::new(margin, 41, 199, 49),
                subtitle: BoxRect::new(margin, 90, 199, 22),
                secondary_label: BoxRect::new(0, 0, 0, 0),
                secondary_value: BoxRect::new(0, 0, 0, 0),
                graph: BoxRect::new(230, 42, 192, 74),
                progress,
                cycle_label: BoxRect::new(230, 92, 192, 20),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 16,
                value_px: 38,
                subtitle_px: 15,
                secondary_value_px: 0,
                secondary_label_px: 0,
            },
            MetricKind::Activity | MetricKind::Normal if expanded => Self {
                title,
                value: BoxRect::new(margin, 45, w - 2 * margin, 52),
                subtitle: BoxRect::new(margin, 100, w - 2 * margin, 25),
                secondary_label: BoxRect::new(0, 0, 0, 0),
                secondary_value: BoxRect::new(0, 0, 0, 0),
                graph: BoxRect::new(margin, 134, w - 2 * margin, 23),
                progress,
                cycle_label: BoxRect::new(margin, 136, w - 2 * margin, 19),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 16,
                value_px: 40,
                subtitle_px: 15,
                secondary_value_px: 0,
                secondary_label_px: 0,
            },
            MetricKind::Activity | MetricKind::Normal if compact => Self {
                title,
                value: BoxRect::new(margin, 37, w - 2 * margin, 36),
                subtitle: BoxRect::new(margin, 74, w - 2 * margin, 20),
                secondary_label: BoxRect::new(0, 0, 0, 0),
                secondary_value: BoxRect::new(0, 0, 0, 0),
                graph: BoxRect::new(margin, 99, w - 2 * margin, 12),
                progress,
                cycle_label: BoxRect::new(0, 0, 0, 0),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 13,
                value_px: 28,
                subtitle_px: 13,
                secondary_value_px: 0,
                secondary_label_px: 0,
            },
            MetricKind::Activity | MetricKind::Normal => Self {
                title,
                value: BoxRect::new(margin, 40, w - 2 * margin, 45),
                subtitle: BoxRect::new(margin, 85, w - 2 * margin, 23),
                secondary_label: BoxRect::new(0, 0, 0, 0),
                secondary_value: BoxRect::new(0, 0, 0, 0),
                graph: BoxRect::new(margin, 112, w - 2 * margin, 18),
                progress,
                cycle_label: BoxRect::new(0, 0, 0, 0),
                process_heading: BoxRect::new(0, 0, 0, 0),
                process_rows: [BoxRect::new(0, 0, 0, 0); 3],
                process_row_count: 0,
                process_row_px: 0,
                title_px: 14,
                value_px: 34,
                subtitle_px: 14,
                secondary_value_px: 0,
                secondary_label_px: 0,
            },
        };
        
        layout.value_px = match font_preference {
            1 => layout.value_px + if compact { 2 } else { 4 },
            2 => (layout.value_px - if compact { 2 } else { 4 }).max(18),
            _ => layout.value_px,
        };

        if kind != MetricKind::Activity {
            layout.cycle_label = BoxRect::new(0, 0, 0, 0);
        }
        if kind == MetricKind::Activity {
            layout.graph = BoxRect::new(0, 0, 0, 0);
        }
        layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_rectangles_fit_all_sizes_and_preferences() {
        for size in 0..4 {
            for kind in [
                MetricKind::Normal,
                MetricKind::Network,
                MetricKind::Activity,
                MetricKind::Processes,
            ] {
                for pref in 0..3 {
                    let l = CardTextLayout::new(size, kind, pref);
                    let (w, h) = match size {
                        0 => (164, 128),
                        2 => (232, 182),
                        3 => (440, 140),
                        _ => (180, 150),
                    };
                    for r in [l.title, l.value, l.subtitle, l.graph, l.progress] {
                        assert!(
                            r.x >= 0
                                && r.y >= 0
                                && r.w >= 0
                                && r.h >= 0
                                && r.right() <= w
                                && r.bottom() <= h,
                            "{size} {kind:?} {r:?}"
                        );
                    }
                    assert!(!l.title.overlaps(l.value));
                    assert!(!l.value.overlaps(l.subtitle));
                    if kind == MetricKind::Network {
                        assert!(!l.secondary_label.overlaps(l.secondary_value));
                        assert!(!l.graph.overlaps(l.secondary_value));
                        assert!(!l.graph.overlaps(l.secondary_label));
                        assert!(!l.graph.overlaps(l.progress));
                    } else if kind == MetricKind::Normal {
                        assert!(!l.graph.overlaps(l.progress));
                    } else if kind == MetricKind::Processes {
                        assert!(l.process_row_count >= 1 && l.process_row_count <= 3);
                        assert!(!l.process_heading.overlaps(l.subtitle));
                        for r in l.process_rows.iter().take(l.process_row_count) {
                            assert!(!r.overlaps(l.subtitle));
                            assert!(!r.overlaps(l.process_heading));
                            assert!(r.right() <= w && r.bottom() <= h);
                        }
                        for a in 0..l.process_row_count {
                            for b in (a + 1)..l.process_row_count {
                                assert!(!l.process_rows[a].overlaps(l.process_rows[b]));
                            }
                        }
                    }
                    assert!(!l.subtitle.overlaps(l.progress));
                }
            }
        }
    }
    #[test]
    fn font_option_affects_primary_number_only() {
        for size in 0..4 {
            let default = CardTextLayout::new(size, MetricKind::Network, 0);
            assert!(CardTextLayout::new(size, MetricKind::Network, 1).value_px > default.value_px);
            assert!(CardTextLayout::new(size, MetricKind::Network, 2).value_px < default.value_px);
            assert_eq!(
                CardTextLayout::new(size, MetricKind::Network, 1).subtitle_px,
                default.subtitle_px
            );
        }
    }
}
