//! Small deterministic graphs shared by backend algorithm tests.

use std::collections::HashMap;

use crate::device::DeviceRecord;
use crate::netlist::{BipartiteHypergraph, CellNode};
use crate::DeviceType;

fn cell(
    name: &str,
    model: &str,
    device_type: DeviceType,
    size: (i32, i32),
    multiplicity: (u16, u16),
    pins: &[(&str, u32)],
) -> CellNode {
    let pins: Vec<(String, u32)> = pins
        .iter()
        .map(|(pin, net)| ((*pin).into(), *net))
        .collect();
    let terminals = pins.iter().cloned().collect::<HashMap<_, _>>();
    CellNode {
        name: name.into(),
        model: model.into(),
        pins,
        device: Some(DeviceRecord {
            name: name.into(),
            device_type,
            w: size.0,
            l: size.1,
            nf: multiplicity.0,
            multiplier: multiplicity.1,
            model_name: model.into(),
            terminals,
            params: HashMap::new(),
        }),
        grouped_devices: Vec::new(),
    }
}

/// Five-transistor OTA used by placement and routing tests.
#[must_use]
pub fn ota() -> BipartiteHypergraph {
    let mut graph = BipartiteHypergraph {
        name: "ota".into(),
        ports: ["vinp", "vinm", "vout1", "vout2", "VDD", "VSS"]
            .into_iter()
            .map(String::from)
            .collect(),
        nets: [
            "vinp", "vinm", "vout1", "vout2", "VDD", "VSS", "vtail", "vbias", "vbn",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        cells: vec![
            cell(
                "XM1",
                "nfet_01v8",
                DeviceType::Nmos,
                (10_000, 1_000),
                (2, 1),
                &[("D", 2), ("G", 0), ("S", 6), ("B", 5)],
            ),
            cell(
                "XM2",
                "nfet_01v8",
                DeviceType::Nmos,
                (10_000, 1_000),
                (2, 1),
                &[("D", 3), ("G", 1), ("S", 6), ("B", 5)],
            ),
            cell(
                "XM3",
                "pfet_01v8",
                DeviceType::Pmos,
                (20_000, 1_000),
                (1, 1),
                &[("D", 2), ("G", 7), ("S", 4), ("B", 4)],
            ),
            cell(
                "XM4",
                "pfet_01v8",
                DeviceType::Pmos,
                (20_000, 1_000),
                (1, 1),
                &[("D", 3), ("G", 7), ("S", 4), ("B", 4)],
            ),
            cell(
                "XM5",
                "nfet_01v8",
                DeviceType::Nmos,
                (40_000, 2_000),
                (1, 4),
                &[("D", 6), ("G", 8), ("S", 5), ("B", 5)],
            ),
        ],
        ..Default::default()
    };
    graph.build_csr();
    graph
}

/// Four parallel NMOS devices used by guard-ring spacing tests.
#[must_use]
pub fn quad() -> BipartiteHypergraph {
    let mut graph = BipartiteHypergraph {
        name: "quad".into(),
        ports: ["n", "g", "VSS"].into_iter().map(String::from).collect(),
        nets: ["n", "g", "VSS"].into_iter().map(String::from).collect(),
        cells: (1..=4)
            .map(|index| {
                cell(
                    &format!("XM{index}"),
                    "nfet_01v8",
                    DeviceType::Nmos,
                    (2_000, 500),
                    (1, 1),
                    &[("D", 0), ("G", 1), ("S", 2), ("B", 2)],
                )
            })
            .collect(),
        ..Default::default()
    };
    graph.build_csr();
    graph
}
