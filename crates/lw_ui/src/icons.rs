use pxui::{
    Paint,
    geom::{Rect, Rgba},
};

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
        paint.curve(at(a), at(middle), at(b), width, color);
    }
    for points in [
        [(7.0, 7.0), (11.5, 7.0), (12.5, 17.0), (17.0, 17.0)],
        [(7.0, 17.0), (8.7, 17.0), (9.8, 15.6), (10.7, 13.8)],
        [(13.3, 10.2), (14.2, 8.4), (15.3, 7.0), (17.0, 7.0)],
    ] {
        paint.cubic(points.map(at), width, color);
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
        let mut ui = pxui::Ui::new().unwrap();
        for round in 0..6 {
            mem_sys::local_reset();
            for (size, scale) in [(16.0, 1.0), (20.0, 1.0), (16.0, 2.0), (20.0, 2.0)] {
                ui.frame(
                    &pxui::FrameIn {
                        width: 40.0,
                        height: 40.0,
                        scale,
                        ..Default::default()
                    },
                    &mut |ui| {
                        let node = ui.node(
                            "icon",
                            pxui::ui::Node {
                                w: pxui::ui::Size::Px(size),
                                h: pxui::ui::Size::Px(size),
                                ..pxui::ui::Node::DEFAULT
                            },
                        );
                        ui.draw(&node, |paint, rect| {
                            shuffle(
                                paint,
                                rect,
                                Rgba {
                                    r: 1,
                                    g: 2,
                                    b: 3,
                                    a: 255,
                                },
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
