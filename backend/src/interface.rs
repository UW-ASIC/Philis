//! Optional constrained-interface specification.
//!
//! Analog blocks dropped into digital harnesses (e.g. TinyTapeout) do not own
//! their die: the harness dictates the die size and the boundary nets/pin
//! positions the block must connect to. An [`InterfaceSpec`] carries that
//! contract into the flow; when absent the flow behaves exactly as before
//! (adaptive die, unattached IO).

use serde::Deserialize;

use pnr_cells::snap_to_grid;

/// Fixed die in nm. Placement must fit inside; the flow never grows it.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DieSpec {
    pub w: i32,
    pub h: i32,
}

/// Die edge for boundary-pin placement.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    North,
    South,
    East,
    West,
}

/// One harness-dictated boundary pin.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoundaryPin {
    pub net: String,
    /// Pin layer name; default = top routing metal of the PDK.
    pub layer: Option<String>,
    /// Edge placement: side + fraction along that edge...
    pub side: Option<Side>,
    /// 0.0..=1.0, required with `side`.
    pub frac: Option<f64>,
    /// ...or absolute coords in nm (mutually exclusive with side/frac).
    pub at: Option<(i32, i32)>,
    /// Pin square side in nm; default = routing pitch, clamped to the layer's
    /// DRC minimum width.
    pub width: Option<i32>,
}

/// Harness contract: fixed die and/or boundary pins.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterfaceSpec {
    pub die: Option<DieSpec>,
    #[serde(default)]
    pub pins: Vec<BoundaryPin>,
}

impl InterfaceSpec {
    /// Structural validation independent of any netlist/PDK: side XOR at,
    /// frac range, positive dimensions. Errors name the offending pin's net.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(d) = self.die {
            if d.w <= 0 || d.h <= 0 {
                return Err(format!("interface die {}x{} must be positive", d.w, d.h));
            }
        }
        for p in &self.pins {
            match (p.side, p.at) {
                (Some(_), Some(_)) => {
                    return Err(format!(
                        "interface pin `{}`: side/frac and at are mutually exclusive",
                        p.net
                    ))
                }
                (None, None) => {
                    return Err(format!(
                        "interface pin `{}`: needs either side+frac or at",
                        p.net
                    ))
                }
                (Some(_), None) => match p.frac {
                    Some(f) if (0.0..=1.0).contains(&f) => {}
                    Some(f) => {
                        return Err(format!(
                            "interface pin `{}`: frac {f} outside 0.0..=1.0",
                            p.net
                        ))
                    }
                    None => return Err(format!("interface pin `{}`: side requires frac", p.net)),
                },
                (None, Some(_)) => {
                    if p.frac.is_some() {
                        return Err(format!(
                            "interface pin `{}`: frac requires side, not at",
                            p.net
                        ));
                    }
                }
            }
            if p.width.is_some_and(|w| w <= 0) {
                return Err(format!("interface pin `{}`: width must be positive", p.net));
            }
        }
        Ok(())
    }
}

impl BoundaryPin {
    /// Resolve the pin-square center on a die of `(w, h)` nm. Edge pins sit
    /// flush inside the named edge; the center is snapped to the manufacturing
    /// grid and kept fully inside the die.
    #[must_use]
    pub fn center(&self, die: (i32, i32), width: i32, grid: i32) -> (i32, i32) {
        let (w, h) = die;
        let half = width / 2;
        let (cx, cy) = if let Some((x, y)) = self.at {
            (x, y)
        } else {
            let frac = self.frac.unwrap_or(0.5);
            let along = |extent: i32| (f64::from(extent) * frac).round() as i32;
            match self.side.expect("validated: side or at present") {
                Side::South => (along(w), half),
                Side::North => (along(w), h - half),
                Side::West => (half, along(h)),
                Side::East => (w - half, along(h)),
            }
        };
        (
            snap_to_grid(cx, grid).max(half).min(w - half),
            snap_to_grid(cy, grid).max(half).min(h - half),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(json: &str) -> BoundaryPin {
        serde_json::from_str(json).expect("pin JSON")
    }

    #[test]
    fn parses_full_spec() {
        let s: InterfaceSpec = serde_json::from_str(
            r#"{"die": {"w": 30000, "h": 70000},
                "pins": [
                  {"net": "vinp", "side": "south", "frac": 0.2},
                  {"net": "clk", "at": [100, 200], "layer": "met2", "width": 500}
                ]}"#,
        )
        .expect("spec JSON");
        s.validate().expect("valid");
        assert_eq!(s.die, Some(DieSpec { w: 30000, h: 70000 }));
        assert_eq!(s.pins.len(), 2);
        assert_eq!(s.pins[0].side, Some(Side::South));
        assert_eq!(s.pins[1].at, Some((100, 200)));
        assert_eq!(s.pins[1].width, Some(500));
    }

    #[test]
    fn empty_spec_is_valid_default() {
        let s: InterfaceSpec = serde_json::from_str("{}").expect("empty spec");
        assert_eq!(s, InterfaceSpec::default());
        s.validate().expect("valid");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(serde_json::from_str::<InterfaceSpec>(r#"{"dies": []}"#).is_err());
        assert!(
            serde_json::from_str::<InterfaceSpec>(r#"{"pins": [{"net": "a", "sides": "north"}]}"#)
                .is_err()
        );
    }

    #[test]
    fn validation_errors_name_the_pin_net() {
        let cases = [
            (
                r#"{"net": "both", "side": "north", "frac": 0.5, "at": [0, 0]}"#,
                "mutually exclusive",
            ),
            (r#"{"net": "neither"}"#, "needs either"),
            (r#"{"net": "range", "side": "north", "frac": 1.5}"#, "outside"),
            (r#"{"net": "nofrac", "side": "north"}"#, "requires frac"),
            (
                r#"{"net": "fracat", "at": [0, 0], "frac": 0.5}"#,
                "requires side",
            ),
            (r#"{"net": "width", "at": [0, 0], "width": 0}"#, "positive"),
        ];
        for (json, want) in cases {
            let p = pin(json);
            let net = p.net.clone();
            let spec = InterfaceSpec {
                die: None,
                pins: vec![p],
            };
            let err = spec.validate().expect_err(json);
            assert!(err.contains(&net), "{err} must name `{net}`");
            assert!(err.contains(want), "{err} must mention `{want}`");
        }
    }

    #[test]
    fn negative_die_is_rejected() {
        let s: InterfaceSpec =
            serde_json::from_str(r#"{"die": {"w": -1, "h": 100}}"#).expect("parses");
        assert!(s.validate().is_err());
    }

    #[test]
    fn edge_pin_centers_sit_flush_inside_each_edge() {
        let die = (30_000, 70_000);
        let (w, grid) = (430, 5);
        let c = |json: &str| pin(json).center(die, w, grid);
        // south 0.2: x = 6000, y = half-width above the bottom edge
        assert_eq!(
            c(r#"{"net": "a", "side": "south", "frac": 0.2}"#),
            (6_000, 215)
        );
        assert_eq!(
            c(r#"{"net": "a", "side": "north", "frac": 0.5}"#),
            (15_000, 70_000 - 215)
        );
        assert_eq!(
            c(r#"{"net": "a", "side": "west", "frac": 0.5}"#),
            (215, 35_000)
        );
        assert_eq!(
            c(r#"{"net": "a", "side": "east", "frac": 0.25}"#),
            (30_000 - 215, 17_500)
        );
    }

    #[test]
    fn corner_fracs_clamp_the_square_inside_the_die() {
        let (cx, cy) = pin(r#"{"net": "a", "side": "south", "frac": 0.0}"#).center((10_000, 10_000), 430, 5);
        assert_eq!((cx, cy), (215, 215));
        let (cx, _) = pin(r#"{"net": "a", "side": "north", "frac": 1.0}"#).center((10_000, 10_000), 430, 5);
        assert_eq!(cx, 10_000 - 215);
    }

    #[test]
    fn absolute_pins_snap_to_grid_and_stay_inside() {
        let p = pin(r#"{"net": "a", "at": [1003, 69999]}"#);
        assert_eq!(p.center((30_000, 70_000), 430, 5), (1_005, 70_000 - 215));
    }
}
