//! Circular progress indicator widget.
//!
//! Renders a small ring that fills clockwise from 12 o'clock.
//! - `Some(fraction)` → determinate fill
//! - `None` → indeterminate spinning arc

use std::f32::consts::{PI, TAU};

use eframe::egui::{self, Pos2, Stroke, Vec2};

/// A circular progress ring widget.
pub struct ProgressRing {
    fraction: Option<f32>,
    radius: f32,
    stroke_width: f32,
}

impl ProgressRing {
    /// Creates a new progress ring.
    ///
    /// `fraction`: `None` = indeterminate (spinning), `Some(0.0..=1.0)` = determinate.
    pub fn new(fraction: Option<f32>) -> Self {
        Self {
            fraction,
            radius: 8.0,
            stroke_width: 2.,
        }
    }

    /// Set the radius of the ring.
    #[allow(dead_code)]
    pub fn radius(
        mut self,
        radius: f32,
    ) -> Self {
        self.radius = radius;
        self
    }

    /// Set the stroke width of the ring.
    #[allow(dead_code)]
    pub fn stroke_width(
        mut self,
        stroke_width: f32,
    ) -> Self {
        self.stroke_width = stroke_width;
        self
    }
}

impl egui::Widget for ProgressRing {
    fn ui(
        self,
        ui: &mut egui::Ui,
    ) -> egui::Response {
        let desired_size = Vec2::splat(self.radius * 2.0 + self.stroke_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let center = rect.center();
            let painter = ui.painter();

            let track_color = ui.visuals().faint_bg_color;
            let active_color = ui.visuals().selection.bg_fill;

            // Draw track (full circle, dim)
            painter.circle_stroke(center, self.radius, Stroke::new(self.stroke_width, track_color));

            match self.fraction {
                Some(fraction) => {
                    // Determinate: arc from 12 o'clock (top) clockwise
                    let fraction = fraction.clamp(0.0, 1.0);
                    if fraction > 0.001 {
                        let start_angle = -PI / 2.0; // 12 o'clock
                        let sweep = fraction * TAU;
                        draw_arc(
                            painter,
                            center,
                            self.radius,
                            start_angle,
                            start_angle + sweep,
                            Stroke::new(self.stroke_width, active_color),
                        );
                    }
                },
                None => {
                    // Indeterminate: spinning 90-degree arc
                    let time = ui.input(|i| i.time) as f32;
                    let spin_speed = 2.0; // radians per second
                    let base_angle = time * spin_speed;
                    let arc_len = PI / 2.0; // 90 degrees

                    draw_arc(
                        painter,
                        center,
                        self.radius,
                        base_angle,
                        base_angle + arc_len,
                        Stroke::new(self.stroke_width, active_color),
                    );

                    // Request continuous repaint for animation
                    ui.ctx().request_repaint();
                },
            }
        }

        response
    }
}

/// Draws an arc on the painter using line segments.
fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: Stroke,
) {
    let segments = 32;
    let delta = (end_angle - start_angle) / segments as f32;
    let points: Vec<Pos2> = (0..=segments)
        .map(|i| {
            let angle = start_angle + delta * i as f32;
            Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();

    // Draw as connected line segments
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }
}
