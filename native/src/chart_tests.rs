// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn frame(chart: &mut Chart, ctx: &egui::Context, events: Vec<egui::Event>) {
    if ctx.cumulative_pass_nr() == 0 {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "noto".into(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/NotoSansCJKsc-Regular.otf"
            ))),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("noto".into());
        }
        ctx.set_fonts(fonts);
    }
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 700.))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(raw, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            chart.show(ui, [0., 1000.], [-1., 1.], "Index", "Value", |range, _| {
                vec![[range[0], -0.5], [range[1], 0.5]]
            });
        });
    });
    output.textures_delta.clear();
}

#[test]
fn wheel_zoom_and_pointer_pan_change_view() {
    let ctx = egui::Context::default();
    let mut chart = Chart::default();
    frame(&mut chart, &ctx, vec![]);
    let original = chart.view.unwrap();
    let pos = Pos2::new(450., 300.);
    frame(
        &mut chart,
        &ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0., 120.),
                phase: egui::TouchPhase::Move,
                modifiers: Default::default(),
            },
        ],
    );
    let zoomed = chart.view.unwrap();
    assert!(zoomed[0][1] - zoomed[0][0] < original[0][1] - original[0][0]);
    frame(
        &mut chart,
        &ctx,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }],
    );
    frame(
        &mut chart,
        &ctx,
        vec![egui::Event::PointerMoved(pos + Vec2::new(60., 0.))],
    );
    assert!(chart.view.unwrap()[0][0] < zoomed[0][0]);
    chart.reset();
    assert!(chart.view.is_none());
}

#[test]
fn svg_is_a_bounded_vector_export_with_escaped_title() {
    let chart = Chart {
        view: Some([[0., 10.], [-1., 1.]]),
        last_points: vec![[0., 0.], [5., 1.], [f64::NAN, 0.], [10., 0.]],
        ..Default::default()
    };
    let svg = chart.svg("a < b & c");
    assert!(svg.contains("a &lt; b &amp; c"));
    assert!(svg.contains("clip-path"));
    assert!(!svg.contains("NaN"));
    assert!(svg.ends_with("</svg>"));
}
