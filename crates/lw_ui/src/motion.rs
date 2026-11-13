//! Reference motion tokens. Curves and samples stay inline on animation paths.
use pxui::theme::Curve;

pub const INSTANT: f32 = 0.08;
pub const QUICK: f32 = 0.14;
pub const BASE: f32 = 0.24;
pub const SLOW: f32 = 0.42;
pub const AMBIENT: f32 = 1.6;
pub const STAGGER: f32 = 0.018;

pub const fn out(duration: f32) -> Curve {
    Curve::Bezier {
        duration,
        points: [0.16, 1.0, 0.3, 1.0],
    }
}
pub const fn ease_in(duration: f32) -> Curve {
    Curve::Bezier {
        duration,
        points: [0.7, 0.0, 0.84, 0.0],
    }
}
pub const fn in_out(duration: f32) -> Curve {
    Curve::Bezier {
        duration,
        points: [0.65, 0.0, 0.35, 1.0],
    }
}
pub const fn spring(duration: f32) -> Curve {
    Curve::Table {
        duration,
        count: 12,
        positions: [
            0.0, 0.0105, 0.021, 0.044, 0.129, 0.167, 0.205, 0.245, 0.284, 0.377, 0.472, 1.0, 0.0,
            0.0, 0.0, 0.0,
        ],
        values: [
            0.0, 0.009, 0.035, 0.141, 0.723, 0.938, 1.017, 1.043, 1.035, 1.01, 0.997, 1.0, 0.0,
            0.0, 0.0, 0.0,
        ],
    }
}
pub const fn steps(count: u32, duration: f32) -> Curve {
    Curve::Steps { count, duration }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_matches_swift_and_browser_vectors_without_allocating() {
        let spring = spring(1.0);
        assert!(spring.valid());
        let samples = [
            (0.0, 0.0),
            (0.0105, 0.009),
            (0.021, 0.035),
            (0.044, 0.141),
            (0.129, 0.723),
            (0.167, 0.938),
            (0.205, 1.017),
            (0.245, 1.043),
            (0.284, 1.035),
            (0.377, 1.01),
            (0.472, 0.997),
            (1.0, 1.0),
        ];
        mem_sys::local_reset();
        for (time, value) in samples.into_iter().chain([(0.75, 0.99858)]) {
            assert!((spring.at(time as f32) as f64 - value).abs() < 1.0e-6);
        }
        for tick in 0..=10_000 {
            let time = tick as f64 / 10_000.0;
            let pair = samples.windows(2).find(|w| time <= w[1].0).unwrap();
            let [(a, av), (b, bv)] = [pair[0], pair[1]];
            let reference = av + (bv - av) * (time - a) / (b - a);
            assert!((spring.at(time as f32) as f64 - reference).abs() < 1.0e-6);
        }
        for (time, value) in [
            (0.0, 0.0),
            (0.1, 0.494391),
            (0.25, 0.825622),
            (0.5, 0.971779),
            (0.75, 0.997677),
            (1.0, 1.0),
        ] {
            assert!((out(1.0).at(time) - value).abs() < 1.0e-5);
        }
        let start = 815_000_000.123_456_f64;
        for row in 0..7 {
            let delay = row as f64 * 0.016;
            for (offset, expected) in [(0.07, 0.5), (0.14, 1.0)] {
                let age = (start + delay + offset) - start - delay;
                assert_eq!(steps(2, QUICK).at(age as f32), expected);
            }
        }
        for step in 0..=12 {
            let age = (start + step as f64 * 1.6 / 12.0) - start;
            assert_eq!(steps(12, AMBIENT).at(age as f32), step as f32 / 12.0);
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
        assert_eq!(spring.at(-1.0), 0.0);
        assert_eq!(spring.at(2.0), 1.0);
        assert_eq!(steps(2, QUICK).at(-1.0), 0.0);
        assert_eq!(steps(2, QUICK).at(2.0), 1.0);
        assert!(spring.at(0.245) > 1.0 && !spring.finished(0.245));
        assert!(!spring.finished(0.999) && spring.finished(1.0));
    }
}
