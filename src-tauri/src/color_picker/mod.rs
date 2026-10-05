mod picker;

pub use picker::{cancel, start};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }

    pub fn hsl(self) -> (f64, f64, f64) {
        let red = self.red as f64 / 255.0;
        let green = self.green as f64 / 255.0;
        let blue = self.blue as f64 / 255.0;
        let maximum = red.max(green).max(blue);
        let minimum = red.min(green).min(blue);
        let delta = maximum - minimum;
        let lightness = (maximum + minimum) / 2.0;
        if delta <= f64::EPSILON {
            return (0.0, 0.0, lightness * 100.0);
        }
        let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
        let hue = if maximum == red {
            60.0 * ((green - blue) / delta).rem_euclid(6.0)
        } else if maximum == green {
            60.0 * ((blue - red) / delta + 2.0)
        } else {
            60.0 * ((red - green) / delta + 4.0)
        };
        (hue, saturation * 100.0, lightness * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn hsl_handles_primary_colors_gray_black_and_white() {
        let cases = [
            (
                Color {
                    red: 255,
                    green: 0,
                    blue: 0,
                },
                (0.0, 100.0, 50.0),
            ),
            (
                Color {
                    red: 0,
                    green: 255,
                    blue: 0,
                },
                (120.0, 100.0, 50.0),
            ),
            (
                Color {
                    red: 0,
                    green: 0,
                    blue: 255,
                },
                (240.0, 100.0, 50.0),
            ),
            (
                Color {
                    red: 255,
                    green: 0,
                    blue: 255,
                },
                (300.0, 100.0, 50.0),
            ),
            (
                Color {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
                (0.0, 0.0, 0.0),
            ),
            (
                Color {
                    red: 255,
                    green: 255,
                    blue: 255,
                },
                (0.0, 0.0, 100.0),
            ),
            (
                Color {
                    red: 128,
                    green: 128,
                    blue: 128,
                },
                (0.0, 0.0, 128.0 / 255.0 * 100.0),
            ),
        ];
        for (color, expected) in cases {
            let actual = color.hsl();
            assert!((actual.0 - expected.0).abs() < 0.0001);
            assert!((actual.1 - expected.1).abs() < 0.0001);
            assert!((actual.2 - expected.2).abs() < 0.0001);
        }
        assert_eq!(
            Color {
                red: 243,
                green: 192,
                blue: 193
            }
            .hex(),
            "#F3C0C1"
        );
    }
}
