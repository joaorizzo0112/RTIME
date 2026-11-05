//! Algoritmos de posicionamento independentes das APIs do Windows.
//! Coordenadas em pixels físicos da tela virtual, inclusive negativos.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width.max(1),
            bottom: y + height.max(1),
        }
    }
    pub fn width(self) -> i32 {
        self.right - self.left
    }
    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
    pub fn moved(self, x: i32, y: i32) -> Self {
        Self::new(x, y, self.width(), self.height())
    }
    pub fn intersection(self, other: Self) -> i64 {
        let w = (self.right.min(other.right) - self.left.max(other.left)).max(0) as i64;
        let h = (self.bottom.min(other.bottom) - self.top.max(other.top)).max(0) as i64;
        w * h
    }
    fn center2(self) -> (i64, i64) {
        (
            (self.left as i64) + (self.right as i64),
            (self.top as i64) + (self.bottom as i64),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Monitor {
    pub work: Rect,
    pub screen: Rect,
    pub dpi: u32,
    pub primary: bool,
}

pub fn scaled(value: i32, dpi: u32) -> i32 {
    (((value as i64) * (dpi.clamp(72, 384) as i64) + 48) / 96).max(1) as i32
}

pub fn nearest_monitor(monitors: &[Monitor], position: Rect) -> Option<usize> {
    if monitors.is_empty() {
        return None;
    }
    // Prioriza a maior interseção real; em caso de empate, o centro mais próximo.
    let (x, y) = position.center2();
    monitors
        .iter()
        .enumerate()
        .max_by_key(|(_, monitor)| {
            let area = position.intersection(monitor.screen);
            let (mx, my) = monitor.screen.center2();
            let dx = x - mx;
            let dy = y - my;
            (area, -(dx * dx + dy * dy), monitor.primary as i32)
        })
        .map(|(index, _)| index)
}

pub fn keep_visible(position: Rect, work: Rect, margin: i32) -> Rect {
    let margin = margin.max(0);
    let left = work.left.saturating_add(margin);
    let top = work.top.saturating_add(margin);
    let right = work.right.saturating_sub(margin);
    let bottom = work.bottom.saturating_sub(margin);
    // Se o widget for maior que a área útil, mantemos ao menos o canto inicial visível.
    let max_x = (right - position.width()).max(left);
    let max_y = (bottom - position.height()).max(top);
    position.moved(
        position.left.clamp(left, max_x),
        position.top.clamp(top, max_y),
    )
}

pub fn recover(position: Rect, monitors: &[Monitor]) -> Rect {
    let Some(index) = nearest_monitor(monitors, position) else {
        return position;
    };
    keep_visible(
        position,
        monitors[index].work,
        scaled(12, monitors[index].dpi),
    )
}

fn overlaps(a0: i32, a1: i32, b0: i32, b1: i32) -> bool {
    a0 < b1 && b0 < a1
}

fn nearest_snap(value: i32, candidates: impl Iterator<Item = i32>, tolerance: i32) -> i32 {
    let mut best = value;
    let mut distance = tolerance + 1;
    for candidate in candidates {
        let delta = ((candidate as i64) - (value as i64)).abs();
        if delta < distance as i64 && delta <= tolerance as i64 {
            best = candidate;
            distance = delta as i32;
        }
    }
    best
}

pub fn snap_moving(
    moving: Rect,
    monitor: Monitor,
    other_windows: impl Iterator<Item = Rect>,
) -> Rect {
    let tolerance = scaled(12, monitor.dpi);
    let gap = scaled(14, monitor.dpi);
    let mut x_candidates = vec![monitor.work.left, monitor.work.right - moving.width()];
    let mut y_candidates = vec![monitor.work.top, monitor.work.bottom - moving.height()];

    for other in other_windows {
        if overlaps(
            moving.top - tolerance,
            moving.bottom + tolerance,
            other.top,
            other.bottom,
        ) {
            x_candidates.extend([other.right + gap, other.left - moving.width() - gap]);
        }
        if overlaps(
            moving.left - tolerance,
            moving.right + tolerance,
            other.left,
            other.right,
        ) {
            y_candidates.extend([other.bottom + gap, other.top - moving.height() - gap]);
        }
    }

    let x = nearest_snap(moving.left, x_candidates.into_iter(), tolerance);
    let y = nearest_snap(moving.top, y_candidates.into_iter(), tolerance);
    moving.moved(x, y)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrangement {
    Grid,
    Row,
    Column,
}

/// Organiza widgets sem apagar atribuições de monitor: cada janela permanece na sua tela.
/// Os resultados são dimensionados em pixels físicos por monitor e evitam a área da barra de tarefas.
#[cfg(test)]
pub fn arrange(
    current: &[Rect],
    visible: &[bool],
    sizes: &[(i32, i32)],
    monitors: &[Monitor],
    mode: Arrangement,
) -> Vec<Rect> {
    let mut result = current.to_vec();
    if monitors.is_empty() {
        return result;
    }
    for (monitor_index, monitor) in monitors.iter().enumerate() {
        let selected: Vec<usize> = current
            .iter()
            .enumerate()
            .filter(|(i, rect)| {
                visible.get(*i).copied().unwrap_or(false)
                    && nearest_monitor(monitors, **rect) == Some(monitor_index)
            })
            .map(|(i, _)| i)
            .collect();
        if selected.is_empty() {
            continue;
        }
        let margin = scaled(16, monitor.dpi);
        let gap = scaled(14, monitor.dpi);
        let mut x = monitor.work.left + margin;
        let mut y = monitor.work.top + margin;
        let mut row_max = 0;
        let mut col_max = 0;
        let mut col_count = 0usize;
        let max_grid_cols = if mode == Arrangement::Grid {
            2
        } else {
            usize::MAX
        };
        for i in selected {
            let (w, h) = sizes.get(i).copied().unwrap_or((180, 150));
            let w = w.max(1);
            let h = h.max(1);
            match mode {
                Arrangement::Grid | Arrangement::Row => {
                    let new_row = (col_count >= max_grid_cols)
                        || (col_count > 0 && x + w > monitor.work.right - margin);
                    if new_row {
                        x = monitor.work.left + margin;
                        y += row_max + gap;
                        row_max = 0;
                        col_count = 0;
                    }
                    result[i] = keep_visible(Rect::new(x, y, w, h), monitor.work, margin);
                    x += w + gap;
                    row_max = row_max.max(h);
                    col_count += 1;
                }
                Arrangement::Column => {
                    if y + h > monitor.work.bottom - margin && y > monitor.work.top + margin {
                        y = monitor.work.top + margin;
                        x += col_max + gap;
                        col_max = 0;
                    }
                    result[i] = keep_visible(Rect::new(x, y, w, h), monitor.work, margin);
                    y += h + gap;
                    col_max = col_max.max(w);
                }
            }
        }
    }
    result
}

/// Âncora 3×3 para a disposição do conjunto na área útil do monitor.
/// Índices na ordem de leitura: superior esquerdo até inferior direito.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    pub fn from_index(value: u32) -> Self {
        match value {
            1 => Self::TopCenter,
            2 => Self::TopRight,
            3 => Self::CenterLeft,
            4 => Self::Center,
            5 => Self::CenterRight,
            6 => Self::BottomLeft,
            7 => Self::BottomCenter,
            8 => Self::BottomRight,
            _ => Self::TopLeft,
        }
    }
    fn horizontal(self) -> i32 {
        match self {
            Self::TopLeft | Self::CenterLeft | Self::BottomLeft => 0,
            Self::TopCenter | Self::Center | Self::BottomCenter => 1,
            _ => 2,
        }
    }
    fn vertical(self) -> i32 {
        match self {
            Self::TopLeft | Self::TopCenter | Self::TopRight => 0,
            Self::CenterLeft | Self::Center | Self::CenterRight => 1,
            _ => 2,
        }
    }
}

pub fn layout_gap(selection: u32, dpi: u32) -> i32 {
    scaled(
        match selection {
            0 => 8,
            2 => 22,
            3 => 32,
            _ => 14,
        },
        dpi,
    )
}

#[cfg(test)]
pub fn translate_group(rectangles: &[Rect], dx: i32, dy: i32) -> Vec<Rect> {
    rectangles
        .iter()
        .map(|rect| rect.moved(rect.left.saturating_add(dx), rect.top.saturating_add(dy)))
        .collect()
}

fn pack_cells(
    sizes: &[(i32, i32)],
    mode: Arrangement,
    max_w: i32,
    max_h: i32,
    gap: i32,
) -> Vec<Rect> {
    if sizes.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(sizes.len());
    match mode {
        Arrangement::Grid => {
            let mut cols = (sizes.len() as f64).sqrt().ceil() as usize;
            while cols > 1 {
                let mut widths = vec![0; cols];
                for (i, (w, _)) in sizes.iter().enumerate() {
                    widths[i % cols] = widths[i % cols].max(*w);
                }
                if widths.iter().sum::<i32>() + gap * (cols as i32 - 1) <= max_w {
                    break;
                }
                cols -= 1;
            }
            let rows = sizes.len().div_ceil(cols);
            let mut widths = vec![0; cols];
            let mut heights = vec![0; rows];
            for (i, &(w, h)) in sizes.iter().enumerate() {
                widths[i % cols] = widths[i % cols].max(w);
                heights[i / cols] = heights[i / cols].max(h);
            }
            for (i, &(w, h)) in sizes.iter().enumerate() {
                let col = i % cols;
                let row = i / cols;
                let x = widths[..col].iter().sum::<i32>() + gap * col as i32;
                let y = heights[..row].iter().sum::<i32>() + gap * row as i32;
                result.push(Rect::new(x, y, w, h));
            }
        }
        Arrangement::Row => {
            let (mut x, mut y, mut row_height) = (0, 0, 0);
            for &(w, h) in sizes {
                if x > 0 && x + w > max_w {
                    x = 0;
                    y += row_height + gap;
                    row_height = 0;
                }
                result.push(Rect::new(x, y, w, h));
                row_height = row_height.max(h);
                x += w + gap;
            }
        }
        Arrangement::Column => {
            let (mut x, mut y, mut column_width) = (0, 0, 0);
            for &(w, h) in sizes {
                if y > 0 && y + h > max_h {
                    y = 0;
                    x += column_width + gap;
                    column_width = 0;
                }
                result.push(Rect::new(x, y, w, h));
                column_width = column_width.max(w);
                y += h + gap;
            }
        }
    }
    result
}

/// Organiza de acordo com o modo, âncora, monitor de destino e espaçamento.
pub fn arrange_anchored(
    current: &[Rect],
    visible: &[bool],
    sizes: &[(i32, i32)],
    monitors: &[Monitor],
    mode: Arrangement,
    anchor: Anchor,
    target_monitor: Option<usize>,
    spacing: u32,
) -> Vec<Rect> {
    let mut result = current.to_vec();
    if monitors.is_empty() {
        return result;
    }
    for (monitor_index, monitor) in monitors.iter().enumerate() {
        let selected: Vec<usize> = current
            .iter()
            .enumerate()
            .filter(|(i, rect)| {
                visible.get(*i).copied().unwrap_or(false)
                    && (if let Some(target) = target_monitor {
                        target == monitor_index
                    } else {
                        nearest_monitor(monitors, **rect) == Some(monitor_index)
                    })
            })
            .map(|(i, _)| i)
            .collect();
        if selected.is_empty() {
            continue;
        }
        let margin = scaled(16, monitor.dpi);
        let gap = layout_gap(spacing, monitor.dpi);
        let available_w = (monitor.work.width() - margin * 2).max(1);
        let available_h = (monitor.work.height() - margin * 2).max(1);
        let group_sizes: Vec<_> = selected
            .iter()
            .map(|&i| sizes.get(i).copied().unwrap_or((180, 150)))
            .collect();
        let layout = pack_cells(&group_sizes, mode, available_w, available_h, gap);
        let block_w = layout.iter().map(|r| r.right).max().unwrap_or(0);
        let block_h = layout.iter().map(|r| r.bottom).max().unwrap_or(0);
        let free_x = (available_w - block_w).max(0);
        let free_y = (available_h - block_h).max(0);
        let origin_x = monitor.work.left + margin + free_x * anchor.horizontal() / 2;
        let origin_y = monitor.work.top + margin + free_y * anchor.vertical() / 2;
        for (&index, slot) in selected.iter().zip(layout.into_iter()) {
            let point = Rect::new(
                origin_x + slot.left,
                origin_y + slot.top,
                slot.width(),
                slot.height(),
            );
            result[index] = keep_visible(point, monitor.work, margin);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn screens() -> Vec<Monitor> {
        vec![
            Monitor {
                work: Rect::new(0, 0, 1920, 1040),
                screen: Rect::new(0, 0, 1920, 1080),
                dpi: 96,
                primary: true,
            },
            Monitor {
                work: Rect::new(-1600, 0, 1600, 860),
                screen: Rect::new(-1600, 0, 1600, 900),
                dpi: 144,
                primary: false,
            },
        ]
    }
    #[test]
    fn monitor_with_negative_coordinates() {
        let m = screens();
        assert_eq!(nearest_monitor(&m, Rect::new(-700, 80, 280, 200)), Some(1));
        assert_eq!(nearest_monitor(&m, Rect::new(600, 80, 280, 200)), Some(0));
    }
    #[test]
    fn recovery_clamps_to_work_area() {
        let m = screens();
        let pos = recover(Rect::new(1850, 980, 300, 150), &m);
        assert_eq!(pos.right, 1908);
        assert_eq!(pos.bottom, 1028);
    }
    #[test]
    fn snap_to_work_boundary() {
        let m = screens();
        let pos = snap_moving(Rect::new(8, 92, 200, 110), m[0], std::iter::empty());
        assert_eq!(pos.left, 0);
        assert_eq!(pos.top, 92);
    }
    #[test]
    fn snap_to_neighbor_edge() {
        let m = screens();
        let pos = snap_moving(
            Rect::new(311, 120, 200, 120),
            m[0],
            std::iter::once(Rect::new(100, 100, 200, 150)),
        );
        assert_eq!(pos.left, 314);
    }
    #[test]
    fn layout_keeps_cards_on_original_monitors() {
        let m = screens();
        let current = vec![
            Rect::new(30, 30, 180, 150),
            Rect::new(-1400, 70, 180, 150),
            Rect::new(500, 400, 180, 150),
        ];
        let sizes = vec![(180, 150); 3];
        let arranged = arrange(&current, &[true; 3], &sizes, &m, Arrangement::Grid);
        assert_eq!(nearest_monitor(&m, arranged[0]), Some(0));
        assert_eq!(nearest_monitor(&m, arranged[1]), Some(1));
        assert_eq!(nearest_monitor(&m, arranged[2]), Some(0));
        assert_eq!(arranged[0].intersection(arranged[2]), 0);
    }
    #[test]
    fn disabled_cards_keep_their_position() {
        let m = screens();
        let positions = vec![Rect::new(80, 80, 180, 150), Rect::new(400, 400, 180, 150)];
        let output = arrange(
            &positions,
            &[true, false],
            &[(180, 150); 2],
            &m,
            Arrangement::Row,
        );
        assert_eq!(output[1], positions[1]);
    }
    #[test]
    fn bottom_right_anchor_positions_group_at_work_area_corner() {
        let monitors = screens();
        let cards = vec![Rect::new(10, 10, 120, 70), Rect::new(50, 50, 120, 70)];
        let result = arrange_anchored(
            &cards,
            &[true, true],
            &[(120, 70); 2],
            &monitors,
            Arrangement::Row,
            Anchor::BottomRight,
            Some(0),
            0,
        );
        assert_eq!(result.iter().map(|r| r.right).max().unwrap(), 1920 - 16);
        assert_eq!(result.iter().map(|r| r.bottom).max().unwrap(), 1040 - 16);
    }
    #[test]
    fn target_monitor_moves_all_enabled_widgets() {
        let monitors = screens();
        let cards = vec![Rect::new(-1200, 30, 180, 150), Rect::new(20, 20, 180, 150)];
        let result = arrange_anchored(
            &cards,
            &[true, true],
            &[(180, 150); 2],
            &monitors,
            Arrangement::Grid,
            Anchor::Center,
            Some(1),
            1,
        );
        assert!(result
            .iter()
            .all(|r| nearest_monitor(&monitors, *r) == Some(1)));
    }
    #[test]
    fn collective_drag_preserves_relative_offsets() {
        let cards = [Rect::new(50, 100, 200, 100), Rect::new(270, 122, 180, 150)];
        let moved = translate_group(&cards, -38, 26);
        assert_eq!(moved[1].left - moved[0].left, 220);
        assert_eq!(moved[1].top - moved[0].top, 22);
    }
    #[test]
    fn dpi_scaling() {
        assert_eq!(scaled(180, 96), 180);
        assert_eq!(scaled(180, 144), 270);
        assert_eq!(scaled(180, 192), 360);
    }
}
