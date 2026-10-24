// SPDX-License-Identifier: GPL-3.0-or-later
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;

#[derive(Default)]
pub struct Chart {
    pub language: crate::i18n::Language,
    pub view: Option<[[f64; 2]; 2]>,
    pub log_x: bool,
    pub log_y: bool,
    pub fill_to_zero: bool,
    pub last_points: Vec<[f64; 2]>,
    pub last_hover: Option<[f64; 2]>,
}
fn transformed(v: f64, log: bool) -> f64 {
    if log {
        if v > 0. { v.log10() } else { f64::NAN }
    } else {
        v
    }
}
fn inverse(v: f64, log: bool) -> f64 {
    if log { 10f64.powf(v) } else { v }
}
fn axis_range(mut range: [f64; 2], log: bool) -> [f64; 2] {
    if log {
        range = if range[1] > 0. {
            [range[0].max(range[1] * 1e-6), range[1]]
        } else {
            [0.1, 1.]
        };
    }
    range = [transformed(range[0], log), transformed(range[1], log)];
    if !range[0].is_finite() || !range[1].is_finite() {
        return [0., 1.];
    }
    if range[0] >= range[1] {
        let pad = (range[0].abs() * 0.01).max(0.5);
        range = [range[0] - pad, range[1] + pad];
    }
    range
}
pub fn tick(v: f64) -> String {
    if v != 0. && (v.abs() >= 100_000. || v.abs() < 0.001) {
        format!("{v:.2e}")
    } else if v.abs() >= 100. {
        format!("{v:.1}")
    } else {
        format!("{v:.4}")
    }
}
impl Chart {
    pub fn reset(&mut self) {
        self.view = None;
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        x: [f64; 2],
        y: [f64; 2],
        x_label: &str,
        y_label: &str,
        mut points: impl FnMut([f64; 2], usize) -> Vec<[f64; 2]>,
    ) -> usize {
        let size = ui.available_size().max(Vec2::new(240., 160.));
        let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
        let rect = Rect::from_min_max(
            outer.min + Vec2::new(80., 14.),
            outer.max - Vec2::new(22., 54.),
        );
        let response = ui.interact(rect, ui.id().with("plot"), Sense::click_and_drag());
        let painter = ui.painter_at(outer);
        let xr = axis_range(x, self.log_x);
        let mut yr = axis_range(y, self.log_y);
        let pad = (yr[1] - yr[0]) * 0.05;
        yr = [yr[0] - pad, yr[1] + pad];
        if y_label == "Count" && !self.log_y {
            yr[0] = 0.;
        }
        let mut view = self.view.unwrap_or([xr, yr]);
        if response.double_clicked() {
            view = [xr, yr];
        }
        if response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            let dx = delta.x as f64 / rect.width() as f64 * (view[0][1] - view[0][0]);
            let dy = delta.y as f64 / rect.height() as f64 * (view[1][1] - view[1][0]);
            view[0] = [view[0][0] - dx, view[0][1] - dx];
            view[1] = [view[1][0] + dy, view[1][1] + dy];
        }
        if response.hovered() {
            let (scroll, shift, pos) = ui.input(|i| {
                (
                    i.smooth_scroll_delta.y,
                    i.modifiers.shift,
                    i.pointer.hover_pos(),
                )
            });
            if scroll != 0. {
                let factor = (-scroll as f64 * 0.003).exp().clamp(0.1, 10.);
                if let Some(pos) = pos {
                    let axis = usize::from(shift);
                    let t = if shift {
                        ((rect.max.y - pos.y) / rect.height()) as f64
                    } else {
                        ((pos.x - rect.min.x) / rect.width()) as f64
                    };
                    let center = view[axis][0] + t * (view[axis][1] - view[axis][0]);
                    let next = [
                        center + (view[axis][0] - center) * factor,
                        center + (view[axis][1] - center) * factor,
                    ];
                    if next.iter().all(|v| v.is_finite() && v.abs() < 1e300)
                        && next[1] - next[0] > 1e-12
                    {
                        view[axis] = next;
                    }
                }
            }
        }
        self.view = Some(view);
        let text = ui.visuals().text_color();
        let muted = Color32::from_gray(125);
        let grid = Stroke::new(1., ui.visuals().widgets.noninteractive.bg_stroke.color);
        painter.rect_filled(rect, 4., ui.visuals().extreme_bg_color);
        for n in 0..=5 {
            let t = n as f32 / 5.;
            let xp = rect.min.x + rect.width() * t;
            let yp = rect.max.y - rect.height() * t;
            painter.line_segment([Pos2::new(xp, rect.min.y), Pos2::new(xp, rect.max.y)], grid);
            painter.line_segment([Pos2::new(rect.min.x, yp), Pos2::new(rect.max.x, yp)], grid);
            painter.text(
                Pos2::new(xp, rect.max.y + 10.),
                Align2::CENTER_TOP,
                tick(inverse(
                    view[0][0] + (view[0][1] - view[0][0]) * t as f64,
                    self.log_x,
                )),
                FontId::monospace(11.),
                muted,
            );
            painter.text(
                Pos2::new(rect.min.x - 9., yp),
                Align2::RIGHT_CENTER,
                tick(inverse(
                    view[1][0] + (view[1][1] - view[1][0]) * t as f64,
                    self.log_y,
                )),
                FontId::monospace(11.),
                muted,
            );
        }
        painter.text(
            Pos2::new(rect.center().x, outer.max.y - 8.),
            Align2::CENTER_BOTTOM,
            x_label,
            FontId::proportional(13.),
            text,
        );
        painter.text(
            outer.min + Vec2::new(6., 0.),
            Align2::LEFT_TOP,
            y_label,
            FontId::proportional(12.),
            text,
        );
        let actual_x = [
            inverse(view[0][0], self.log_x),
            inverse(view[0][1], self.log_x),
        ];
        let data = points(actual_x, rect.width() as usize);
        let clipped = painter.with_clip_rect(rect);
        let mut segment = Vec::new();
        let color = Color32::from_rgb(59, 153, 222);
        let fill_slot = self.fill_to_zero.then(|| clipped.add(egui::Shape::Noop));
        let mut fill_mesh = egui::epaint::Mesh::default();
        let fill_color = Color32::from_rgba_unmultiplied(59, 153, 222, 90);
        // Zero has no log coordinate: fill to the visible lower edge in log mode.
        let baseline = if self.log_y {
            rect.max.y
        } else {
            (rect.max.y - ((0. - view[1][0]) / (view[1][1] - view[1][0])) as f32 * rect.height())
                .clamp(-1e6, 1e6)
        };
        let mut previous: Option<Pos2> = None;
        for point in &data {
            let tx = transformed(point[0], self.log_x);
            let ty = transformed(point[1], self.log_y);
            if !tx.is_finite() || !ty.is_finite() {
                if segment.len() > 1 {
                    clipped.add(egui::Shape::line(
                        std::mem::take(&mut segment),
                        Stroke::new(1.3, color),
                    ));
                }
                segment.clear();
                previous = None;
                continue;
            }
            let px =
                rect.min.x + ((tx - view[0][0]) / (view[0][1] - view[0][0])) as f32 * rect.width();
            let py =
                rect.max.y - ((ty - view[1][0]) / (view[1][1] - view[1][0])) as f32 * rect.height();
            let position = Pos2::new(px.clamp(-1e6, 1e6), py.clamp(-1e6, 1e6));
            if self.fill_to_zero
                && let Some(previous) = previous
            {
                let start = fill_mesh.vertices.len() as u32;
                for vertex in [
                    previous,
                    position,
                    Pos2::new(position.x, baseline),
                    Pos2::new(previous.x, baseline),
                ] {
                    fill_mesh.colored_vertex(vertex, fill_color);
                }
                fill_mesh.add_triangle(start, start + 1, start + 2);
                fill_mesh.add_triangle(start, start + 2, start + 3);
            }
            previous = Some(position);
            segment.push(position);
        }
        if segment.len() > 1 {
            clipped.add(egui::Shape::line(segment, Stroke::new(1.3, color)));
        } else if let Some(&p) = segment.first() {
            clipped.circle_filled(p, 3., color);
        }
        if let Some(slot) = fill_slot {
            clipped.set(slot, egui::Shape::mesh(fill_mesh));
        }
        self.last_hover = None;
        if response.hovered()
            && let Some(pos) = response.hover_pos()
        {
            let px = inverse(
                view[0][0]
                    + ((pos.x - rect.min.x) / rect.width()) as f64 * (view[0][1] - view[0][0]),
                self.log_x,
            );
            let py = inverse(
                view[1][0]
                    + ((rect.max.y - pos.y) / rect.height()) as f64 * (view[1][1] - view[1][0]),
                self.log_y,
            );
            self.last_hover = Some([px, py]);
            clipped.line_segment(
                [Pos2::new(pos.x, rect.min.y), Pos2::new(pos.x, rect.max.y)],
                Stroke::new(1., Color32::from_gray(110)),
            );
            response.on_hover_text(format!(
                "x = {}\ny = {}\n{}",
                tick(px),
                tick(py),
                self.language.text("plot_tooltip")
            ));
        }
        let count = data.len();
        self.last_points = data;
        count
    }
    pub fn svg(&self, title: &str) -> String {
        let escape = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        let view = self.view.unwrap_or([[0., 1.], [0., 1.]]);
        let mut path = String::new();
        let mut area = String::new();
        let mut fill_last_x = None;
        let baseline = if self.log_y {
            630.
        } else {
            630. - (0. - view[1][0]) / (view[1][1] - view[1][0]) * 570.
        };
        let mut first = true;
        for p in &self.last_points {
            let x = transformed(p[0], self.log_x);
            let y = transformed(p[1], self.log_y);
            if !x.is_finite() || !y.is_finite() {
                if let Some(x) = fill_last_x.take() {
                    area.push_str(&format!("L {x:.2},{baseline:.2} Z "));
                }
                first = true;
                continue;
            }
            let x = 90. + (x - view[0][0]) / (view[0][1] - view[0][0]) * 1080.;
            let y = 630. - (y - view[1][0]) / (view[1][1] - view[1][0]) * 570.;
            if self.fill_to_zero {
                if first {
                    area.push_str(&format!("M {x:.2},{baseline:.2} "));
                }
                area.push_str(&format!("L {x:.2},{y:.2} "));
                fill_last_x = Some(x);
            }
            path.push_str(&format!("{} {x:.2},{y:.2} ", if first { "M" } else { "L" }));
            first = false;
        }
        if let Some(x) = fill_last_x {
            area.push_str(&format!("L {x:.2},{baseline:.2} Z "));
        }
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"700\"><rect width=\"1200\" height=\"700\" fill=\"white\"/><text x=\"90\" y=\"30\" font-family=\"sans-serif\" font-size=\"18\">{}</text><defs><clipPath id=\"p\"><rect x=\"90\" y=\"60\" width=\"1080\" height=\"570\"/></clipPath></defs>",
            escape(title)
        );
        for i in 0..=5 {
            let t = i as f64 / 5.;
            let x = 90. + 1080. * t;
            let y = 630. - 570. * t;
            svg.push_str(&format!("<path d=\"M{x},60V630 M90,{y}H1170\" stroke=\"#ddd\" fill=\"none\"/><text x=\"{x}\" y=\"655\" text-anchor=\"middle\" font-size=\"12\">{}</text><text x=\"80\" y=\"{y}\" text-anchor=\"end\" font-size=\"12\">{}</text>", tick(inverse(view[0][0] + t * (view[0][1] - view[0][0]), self.log_x)), tick(inverse(view[1][0] + t * (view[1][1] - view[1][0]), self.log_y))));
        }
        if self.fill_to_zero {
            svg.push_str(&format!(
                "<path d=\"{area}\" fill=\"#3b99de\" fill-opacity=\"0.35\" clip-path=\"url(#p)\"/>"
            ));
        }
        svg.push_str(&format!("<path d=\"{path}\" stroke=\"#3b99de\" stroke-width=\"1.3\" fill=\"none\" clip-path=\"url(#p)\"/></svg>"));
        svg
    }
}
