use pxui::{
    Fill, Paint, Stroke,
    geom::{Rect, Rgba},
};

pub(crate) fn play(paint: &mut Paint<'_>, rect: Rect, color: Rgba) {
    let scale = rect.w.min(rect.h) / 24.0;
    let at = |(x, y): (f32, f32)| (rect.x + x * scale, rect.y + y * scale);
    paint.triangle(at((7.0, 4.6)), at((7.0, 19.4)), at((19.4, 12.0)), Fill::new(color));
}
pub(crate) fn heart(paint: &mut Paint<'_>, rect: Rect, filled: bool, color: Rgba) {
    let scale = rect.w.min(rect.h) / 24.0;
    let at = |(x, y): (f32, f32)| (rect.x + x * scale, rect.y + y * scale);
    if filled {
        for x in [8.2, 15.8] {
            paint.circle(at((x, 8.8)), 4.2 * scale, Some(Fill::new(color)), None);
        }
        paint.triangle(at((4.2, 10.4)), at((19.8, 10.4)), at((12.0, 20.0)), Fill::new(color));
        return;
    }
    for side in [-1.0_f32, 1.0] {
        let mirror = |(x, y): (f32, f32)| at((12.0 + side * (x - 12.0), y));
        for points in [
            [(12.0, 20.5), (6.0, 15.5), (3.0, 12.0), (3.0, 8.5)],
            [(3.0, 8.5), (3.0, 5.5), (5.2, 4.0), (7.5, 4.0)],
            [(7.5, 4.0), (9.5, 4.0), (11.0, 5.0), (12.0, 7.0)],
        ] {
            paint.cubic(points.map(mirror), Stroke::new(color).width(1.6 * scale));
        }
    }
}
pub(crate) fn offline(paint: &mut Paint<'_>, rect: Rect, kept: bool, color: Rgba) {
    let scale = rect.w.min(rect.h) / 24.0;
    let at = |(x, y): (f32, f32)| (rect.x + x * scale, rect.y + y * scale);
    let mut line = |a: (f32, f32), b: (f32, f32)| {
        paint.curve(at(a), at(((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5)), at(b), Stroke::new(color).width(1.6 * scale))
    };
    if kept {
        line((5.0, 12.5), (9.5, 17.0));
        line((9.5, 17.0), (19.0, 7.5));
    } else {
        line((12.0, 4.0), (12.0, 15.0));
        line((7.0, 10.0), (12.0, 15.0));
        line((12.0, 15.0), (17.0, 10.0));
        line((5.0, 20.0), (19.0, 20.0));
    }
}

pub(crate) fn repeat(paint: &mut Paint<'_>, rect: Rect, color: Rgba) {
    // LW-rep.imageset: M5 11V9.5A2.5 2.5 0 0 1 7.5 7H19M16 4l3 3-3 3M19 13v1.5a2.5 2.5 0 0 1-2.5 2.5H5M8 20l-3-3 3-3
    let scale = rect.w.min(rect.h) / 24.0;
    let at = |(x, y): (f32, f32)| (rect.x + x * scale, rect.y + y * scale);
    let width = 1.6 * scale;
    for (a, control, b) in [
        ((5.0, 11.0), (5.0, 11.0), (5.0, 9.5)),
        ((5.0, 9.5), (5.0, 7.0), (7.5, 7.0)),
        ((7.5, 7.0), (13.25, 7.0), (19.0, 7.0)),
        ((16.0, 4.0), (17.5, 5.5), (19.0, 7.0)),
        ((19.0, 7.0), (17.5, 8.5), (16.0, 10.0)),
        ((19.0, 13.0), (19.0, 13.0), (19.0, 14.5)),
        ((19.0, 14.5), (19.0, 17.0), (16.5, 17.0)),
        ((16.5, 17.0), (10.75, 17.0), (5.0, 17.0)),
        ((8.0, 20.0), (6.5, 18.5), (5.0, 17.0)),
        ((5.0, 17.0), (6.5, 15.5), (8.0, 14.0)),
    ] {
        paint.curve(at(a), at(control), at(b), Stroke::new(color).width(width));
    }
}

pub(crate) fn shuffle(paint: &mut Paint<'_>, rect: Rect, color: Rgba) {
    // Same 24-unit paths and 1.6-unit round stroke as LW-shuf.imageset/shuf.svg.
    let scale = rect.w.min(rect.h) / 24.0;
    let at = |(x, y): (f32, f32)| (rect.x + x * scale, rect.y + y * scale);
    let width = 1.6 * scale;
    for (a, b) in [
        ((4.0, 7.0), (7.0, 7.0)),
        ((17.0, 17.0), (20.0, 17.0)),
        ((4.0, 17.0), (7.0, 17.0)),
        ((17.0, 7.0), (20.0, 7.0)),
        ((17.5, 4.5), (20.0, 7.0)),
        ((20.0, 7.0), (17.5, 9.5)),
        ((17.5, 14.5), (20.0, 17.0)),
        ((20.0, 17.0), (17.5, 19.5)),
    ] {
        let middle = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        paint.curve(at(a), at(middle), at(b), Stroke::new(color).width(width));
    }
    for points in [
        [(7.0, 7.0), (11.5, 7.0), (12.5, 17.0), (17.0, 17.0)],
        [(7.0, 17.0), (8.7, 17.0), (9.8, 15.6), (10.7, 13.8)],
        [(13.3, 10.2), (14.2, 8.4), (15.3, 7.0), (17.0, 7.0)],
    ] {
        paint.cubic(points.map(at), Stroke::new(color).width(width));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shuffle_matches_reference_curves_with_subpixel_error_and_no_warm_allocations() {
        mem_sys::set_peer(pxui::library_counter);
        let svg = include_str!("../../../localwave/Assets.xcassets/LW-shuf.imageset/shuf.svg");
        assert!(
            svg.contains("stroke-width=\"1.6\" stroke-linecap=\"round\" stroke-linejoin=\"round\"")
        );
        assert!(svg.contains("M4 7h3c4.5 0 5.5 10 10 10h3M4 17h3c1.7 0 2.8-1.4 3.7-3.2M13.3 10.2C14.2 8.4 15.3 7 17 7h3M17.5 4.5 20 7l-2.5 2.5M17.5 14.5 20 17l-2.5 2.5"));
        let original = [
            [(7.0, 7.0), (11.5, 7.0), (12.5, 17.0), (17.0, 17.0)],
            [(7.0, 17.0), (8.7, 17.0), (9.8, 15.6), (10.7, 13.8)],
            [(13.3, 10.2), (14.2, 8.4), (15.3, 7.0), (17.0, 7.0)],
        ];
        let mut ui = pxui::Ui::new(&pxui::UiDesc::DEFAULT).unwrap();
        for round in 0..6 {
            mem_sys::local_reset();
            for (size, scale) in [(16.0, 1.0), (20.0, 1.0), (16.0, 2.0), (20.0, 2.0)] {
                crate::testing::frame(
                    &mut ui,
                    &pxui::FrameIn::new(40.0, 40.0).scale(scale),
                    &mut |ui| {
                        let node = ui.node("icon", pxui::ui::Node::column().w_px(size).h_px(size));
                        ui.canvas(&node, |paint, rect| {
                            shuffle(
                                paint,
                                rect,
                                Rgba::rgb(1, 2, 3),
                            )
                        });
                    },
                )
                .unwrap();
                let mut count = 0;
                for (index, curve) in ui
                    .layers()
                    .flat_map(|layer| layer.instances.iter())
                    .enumerate()
                {
                    count += 1;
                    assert_eq!(curve.kind, pxui::draw::kind::CURVE);
                    if index < 8 {
                        continue;
                    }
                    let points = original[(index - 8) / 4];
                    let segment = (index - 8) % 4;
                    let (qx, qy) = (
                        f32::from_bits(curve.extra[0]),
                        f32::from_bits(curve.extra[1]),
                    );
                    for sample in 0..=32 {
                        let u = sample as f32 / 32.0;
                        let v = 1.0 - u;
                        let t = (segment as f32 + u) / 4.0;
                        let s = 1.0 - t;
                        let expected = (
                            points[0].0 * s * s * s
                                + 3.0 * points[1].0 * s * s * t
                                + 3.0 * points[2].0 * s * t * t
                                + points[3].0 * t * t * t,
                            points[0].1 * s * s * s
                                + 3.0 * points[1].1 * s * s * t
                                + 3.0 * points[2].1 * s * t * t
                                + points[3].1 * t * t * t,
                        );
                        let actual = (
                            curve.rect[0] * v * v + 2.0 * qx * v * u + curve.rect[2] * u * u,
                            curve.rect[1] * v * v + 2.0 * qy * v * u + curve.rect[3] * u * u,
                        );
                        let error = (actual.0 - expected.0 * size / 24.0)
                            .hypot(actual.1 - expected.1 * size / 24.0)
                            * scale;
                        assert!(
                            error < 0.04,
                            "reference cubic deviation at {scale}x: {error} pixels"
                        );
                    }
                }
                assert_eq!(
                    count, 20,
                    "bounded analytic instances, no tessellation buffer"
                );
            }
            if round >= 3 {
                assert_eq!(mem_sys::local_alloc_count(), 0, "warm reference vectors");
            }
        }
    }
}
