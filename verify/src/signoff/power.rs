use std::collections::{HashSet, VecDeque};

use super::{CheckReport, SignoffCheck, SignoffViolation};

#[derive(Debug, Clone, PartialEq)]
pub struct PowerNode {
    pub id: String,
    pub x: i32,
    pub y: i32,
    /// Expected rail voltage at this node before interconnect loss.
    pub nominal_voltage_v: f64,
    /// Some(v) makes this an ideal voltage source/boundary condition.
    pub fixed_voltage_v: Option<f64>,
    /// Positive values draw current from the grid; negative values inject it.
    pub load_current_a: f64,
    pub check_ir_drop: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PowerEdgeKind {
    Metal { width_um: f64, thickness_um: f64 },
    Via { cuts: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerEdge {
    pub id: String,
    pub from: usize,
    pub to: usize,
    pub resistance_ohm: f64,
    pub length_um: f64,
    pub kind: PowerEdgeKind,
    pub temperature_c: f64,
    /// Per-edge foundry limit; falls back to the EM config default.
    pub max_current_density_a_per_um2: Option<f64>,
    /// Per-cut limit for vias; falls back to the EM config default.
    pub max_current_per_cut_a: Option<f64>,
    /// Optional Blech `J*L` immortality threshold in A/um.
    pub blech_product_limit_a_per_um: Option<f64>,
    pub em_exempt: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PowerGrid {
    pub nodes: Vec<PowerNode>,
    pub edges: Vec<PowerEdge>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSolveConfig {
    pub relative_tolerance: f64,
    pub max_iterations: usize,
}

impl Default for PowerSolveConfig {
    fn default() -> Self {
        Self {
            relative_tolerance: 1e-10,
            max_iterations: 20_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrDropConfig {
    pub max_drop_v: Option<f64>,
    /// Percent, e.g. `5.0` means five percent.
    pub max_drop_pct: Option<f64>,
    pub max_overvoltage_v: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElectromigrationConfig {
    pub default_max_current_density_a_per_um2: Option<f64>,
    pub default_max_current_per_cut_a: Option<f64>,
    pub reference_temperature_c: f64,
    pub activation_energy_ev: f64,
    pub current_exponent: f64,
    pub max_temperature_c: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSignoffConfig {
    pub grid: PowerGrid,
    pub solver: PowerSolveConfig,
    pub ir_drop: Option<IrDropConfig>,
    pub electromigration: Option<ElectromigrationConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeVoltage {
    pub node: usize,
    pub id: String,
    pub voltage_v: f64,
    pub nominal_voltage_v: f64,
    pub drop_v: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BranchCurrent {
    pub edge: usize,
    pub id: String,
    /// Positive is from `PowerEdge::from` to `PowerEdge::to`.
    pub current_a: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerSolution {
    pub node_voltages: Vec<NodeVoltage>,
    pub branch_currents: Vec<BranchCurrent>,
    pub iterations: usize,
    pub relative_residual: f64,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Solve the resistor grid by conjugate gradients after eliminating fixed-voltage nodes.
pub fn solve_power_grid(
    grid: &PowerGrid,
    config: &PowerSolveConfig,
) -> Result<PowerSolution, String> {
    if grid.nodes.is_empty() {
        return Err("power grid contains no nodes".into());
    }
    if !config.relative_tolerance.is_finite()
        || config.relative_tolerance <= 0.0
        || config.max_iterations == 0
    {
        return Err("power solver tolerance/iteration limit is invalid".into());
    }
    let mut ids = HashSet::new();
    for (i, n) in grid.nodes.iter().enumerate() {
        if n.id.is_empty() || !ids.insert(n.id.as_str()) {
            return Err(format!("node {i} has an empty or duplicate id"));
        }
        if !n.nominal_voltage_v.is_finite()
            || !n.load_current_a.is_finite()
            || n.fixed_voltage_v.is_some_and(|v| !v.is_finite())
        {
            return Err(format!(
                "node '{}' contains a non-finite electrical value",
                n.id
            ));
        }
    }
    if !grid.nodes.iter().any(|n| n.fixed_voltage_v.is_some()) {
        return Err("power grid has no fixed-voltage supply/reference node".into());
    }

    let mut adjacency = vec![Vec::new(); grid.nodes.len()];
    let mut edge_ids = HashSet::new();
    for (i, e) in grid.edges.iter().enumerate() {
        if e.id.is_empty()
            || !edge_ids.insert(e.id.as_str())
            || e.from >= grid.nodes.len()
            || e.to >= grid.nodes.len()
            || e.from == e.to
            || !e.resistance_ohm.is_finite()
            || e.resistance_ohm <= 0.0
        {
            return Err(format!("edge {i} ('{}') is malformed or duplicated", e.id));
        }
        adjacency[e.from].push(e.to);
        adjacency[e.to].push(e.from);
    }

    // Every solved component must be anchored by at least one fixed-voltage node.
    let mut anchored = vec![false; grid.nodes.len()];
    let mut queue = VecDeque::new();
    for (i, n) in grid.nodes.iter().enumerate() {
        if n.fixed_voltage_v.is_some() {
            anchored[i] = true;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        for &j in &adjacency[i] {
            if !anchored[j] {
                anchored[j] = true;
                queue.push_back(j);
            }
        }
    }
    if let Some((i, _)) = grid
        .nodes
        .iter()
        .enumerate()
        .find(|(i, n)| n.fixed_voltage_v.is_none() && !anchored[*i])
    {
        return Err(format!(
            "node '{}' belongs to an unanchored power-grid island",
            grid.nodes[i].id
        ));
    }

    let mut unknown_of_node = vec![usize::MAX; grid.nodes.len()];
    let mut nodes_of_unknown = Vec::new();
    for (i, n) in grid.nodes.iter().enumerate() {
        if n.fixed_voltage_v.is_none() {
            unknown_of_node[i] = nodes_of_unknown.len();
            nodes_of_unknown.push(i);
        }
    }
    let m = nodes_of_unknown.len();
    let mut diag = vec![0.0; m];
    let mut off: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    let mut rhs = vec![0.0; m];
    for (ui, &ni) in nodes_of_unknown.iter().enumerate() {
        rhs[ui] = -grid.nodes[ni].load_current_a;
    }
    for e in &grid.edges {
        let g = 1.0 / e.resistance_ohm;
        let uf = unknown_of_node[e.from];
        let ut = unknown_of_node[e.to];
        match (uf != usize::MAX, ut != usize::MAX) {
            (true, true) => {
                diag[uf] += g;
                diag[ut] += g;
                off[uf].push((ut, g));
                off[ut].push((uf, g));
            }
            (true, false) => {
                diag[uf] += g;
                rhs[uf] += g * grid.nodes[e.to].fixed_voltage_v.unwrap();
            }
            (false, true) => {
                diag[ut] += g;
                rhs[ut] += g * grid.nodes[e.from].fixed_voltage_v.unwrap();
            }
            (false, false) => {}
        }
    }
    if let Some((ui, _)) = diag
        .iter()
        .enumerate()
        .find(|(_, d)| **d <= 0.0 || !d.is_finite())
    {
        return Err(format!(
            "node '{}' has no finite conductive path",
            grid.nodes[nodes_of_unknown[ui]].id
        ));
    }

    let matvec = |x: &[f64], y: &mut [f64]| {
        for i in 0..m {
            let mut v = diag[i] * x[i];
            for &(j, g) in &off[i] {
                v -= g * x[j];
            }
            y[i] = v;
        }
    };

    let mut x: Vec<f64> = nodes_of_unknown
        .iter()
        .map(|&i| grid.nodes[i].nominal_voltage_v)
        .collect();
    let mut ax = vec![0.0; m];
    matvec(&x, &mut ax);
    let mut r: Vec<f64> = rhs.iter().zip(&ax).map(|(b, a)| b - a).collect();
    let mut p = r.clone();
    let mut rr = dot(&r, &r);
    // Do not impose an implicit one-ampere absolute scale on convergence.
    // Small IC loads are routine; normalizing by max(1 A) could accept the
    // nominal-voltage initial guess without solving a micro/nanoamp network.
    let rhs_norm = dot(&rhs, &rhs).sqrt().max(f64::MIN_POSITIVE);
    let target = config.relative_tolerance * rhs_norm;
    let mut iterations = 0;
    let mut ap = vec![0.0; m];
    while rr.sqrt() > target && iterations < config.max_iterations {
        matvec(&p, &mut ap);
        let pap = dot(&p, &ap);
        // `pap` scales with both current and conductance and can legitimately
        // be far below machine epsilon in a high-resistance, low-current grid.
        // Positive definiteness is a sign test here, not an absolute cutoff.
        if !pap.is_finite() || pap <= 0.0 {
            return Err("power-grid matrix is singular or not positive definite".into());
        }
        let alpha = rr / pap;
        for i in 0..m {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        let next_rr = dot(&r, &r);
        if !next_rr.is_finite() {
            return Err("power-grid solver diverged".into());
        }
        let beta = if rr > 0.0 { next_rr / rr } else { 0.0 };
        for i in 0..m {
            p[i] = r[i] + beta * p[i];
        }
        rr = next_rr;
        iterations += 1;
    }
    let relative_residual = rr.sqrt() / rhs_norm;
    if relative_residual > config.relative_tolerance {
        return Err(format!(
            "power-grid solver did not converge in {} iterations (relative residual {:.3e})",
            config.max_iterations, relative_residual,
        ));
    }

    let mut voltages = vec![0.0; grid.nodes.len()];
    for (i, n) in grid.nodes.iter().enumerate() {
        voltages[i] = n.fixed_voltage_v.unwrap_or_else(|| x[unknown_of_node[i]]);
    }
    let node_voltages = grid
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            // Positive and negative rails both report degradation as a positive
            // drop.  On a nominal 0V rail, absolute ground bounce is the drop.
            let drop_v = if n.nominal_voltage_v.abs() <= f64::EPSILON {
                voltages[i].abs()
            } else {
                n.nominal_voltage_v.signum() * (n.nominal_voltage_v - voltages[i])
            };
            NodeVoltage {
                node: i,
                id: n.id.clone(),
                voltage_v: voltages[i],
                nominal_voltage_v: n.nominal_voltage_v,
                drop_v,
            }
        })
        .collect();
    let branch_currents = grid
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| BranchCurrent {
            edge: i,
            id: e.id.clone(),
            current_a: (voltages[e.from] - voltages[e.to]) / e.resistance_ohm,
        })
        .collect();
    Ok(PowerSolution {
        node_voltages,
        branch_currents,
        iterations,
        relative_residual,
    })
}

#[derive(Debug, Clone)]
pub struct IrDropReport {
    pub check: CheckReport,
    pub nodes: Vec<NodeVoltage>,
    pub branches: Vec<BranchCurrent>,
}

impl IrDropReport {
    pub(crate) fn not_run(reason: impl Into<String>) -> Self {
        Self {
            check: CheckReport::not_run(SignoffCheck::IrDrop, reason),
            nodes: Vec::new(),
            branches: Vec::new(),
        }
    }
    pub(crate) fn error(reason: impl Into<String>) -> Self {
        Self {
            check: CheckReport::error(SignoffCheck::IrDrop, reason),
            nodes: Vec::new(),
            branches: Vec::new(),
        }
    }
}

pub fn analyze_ir_drop(
    grid: &PowerGrid,
    solution: &PowerSolution,
    config: &IrDropConfig,
) -> IrDropReport {
    if solution.node_voltages.len() != grid.nodes.len()
        || solution.branch_currents.len() != grid.edges.len()
        || solution
            .node_voltages
            .iter()
            .enumerate()
            .any(|(i, n)| n.node != i || !n.voltage_v.is_finite() || !n.drop_v.is_finite())
        || solution
            .branch_currents
            .iter()
            .enumerate()
            .any(|(i, b)| b.edge != i || !b.current_a.is_finite())
    {
        return IrDropReport::error("power solution does not correspond to this grid");
    }
    if config.max_drop_v.is_none()
        && config.max_drop_pct.is_none()
        && config.max_overvoltage_v.is_none()
    {
        return IrDropReport::error("IR-drop configuration contains no limits");
    }
    if !grid.nodes.iter().any(|node| node.check_ir_drop) {
        return IrDropReport::error("IR-drop scope contains no nodes enabled for checking");
    }
    for (name, limit) in [
        ("max_drop_v", config.max_drop_v),
        ("max_drop_pct", config.max_drop_pct),
        ("max_overvoltage_v", config.max_overvoltage_v),
    ] {
        if limit.is_some_and(|v| !v.is_finite() || v < 0.0) {
            return IrDropReport::error(format!("IR-drop limit {name} is invalid"));
        }
    }

    let mut violations = Vec::new();
    for result in &solution.node_voltages {
        let node = &grid.nodes[result.node];
        if !node.check_ir_drop {
            continue;
        }
        if let Some(limit) = config.max_drop_v {
            if result.drop_v > limit {
                violations.push(SignoffViolation {
                    check: SignoffCheck::IrDrop,
                    rule_id: "ir_drop.absolute".into(),
                    message: format!(
                        "node '{}' drop {:.6}V exceeds {:.6}V",
                        node.id, result.drop_v, limit
                    ),
                    location: Some((node.x, node.y)),
                    measured: Some(result.drop_v),
                    limit: Some(limit),
                    units: "V".into(),
                });
            }
        }
        if let Some(limit) = config.max_drop_pct {
            if node.nominal_voltage_v.abs() <= f64::EPSILON {
                return IrDropReport::error(format!(
                    "node '{}' has zero nominal voltage but a percentage limit is enabled",
                    node.id,
                ));
            }
            let pct = 100.0 * result.drop_v / node.nominal_voltage_v.abs();
            if pct > limit {
                violations.push(SignoffViolation {
                    check: SignoffCheck::IrDrop,
                    rule_id: "ir_drop.percent".into(),
                    message: format!("node '{}' drop {:.4}% exceeds {:.4}%", node.id, pct, limit),
                    location: Some((node.x, node.y)),
                    measured: Some(pct),
                    limit: Some(limit),
                    units: "%".into(),
                });
            }
        }
        if let Some(limit) = config.max_overvoltage_v {
            let over = -result.drop_v;
            if over > limit {
                violations.push(SignoffViolation {
                    check: SignoffCheck::IrDrop,
                    rule_id: "ir_drop.overvoltage".into(),
                    message: format!(
                        "node '{}' overvoltage {:.6}V exceeds {:.6}V",
                        node.id, over, limit
                    ),
                    location: Some((node.x, node.y)),
                    measured: Some(over),
                    limit: Some(limit),
                    units: "V".into(),
                });
            }
        }
    }
    IrDropReport {
        check: CheckReport::from_violations(
            SignoffCheck::IrDrop,
            violations,
            vec![format!(
                "solver iterations={}, relative residual={:.3e}",
                solution.iterations, solution.relative_residual,
            )],
        ),
        nodes: solution.node_voltages.clone(),
        branches: solution.branch_currents.clone(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmBranchResult {
    pub edge: usize,
    pub id: String,
    pub current_a: f64,
    pub current_density_a_per_um2: Option<f64>,
    pub allowed: f64,
    pub blech_exempt: bool,
}

#[derive(Debug, Clone)]
pub struct ElectromigrationReport {
    pub check: CheckReport,
    pub branches: Vec<EmBranchResult>,
}

impl ElectromigrationReport {
    pub(crate) fn not_run(reason: impl Into<String>) -> Self {
        Self {
            check: CheckReport::not_run(SignoffCheck::Electromigration, reason),
            branches: Vec::new(),
        }
    }
    pub(crate) fn error(reason: impl Into<String>) -> Self {
        Self {
            check: CheckReport::error(SignoffCheck::Electromigration, reason),
            branches: Vec::new(),
        }
    }
}

const BOLTZMANN_EV_PER_K: f64 = 8.617_333_262_145e-5;

pub fn analyze_electromigration(
    grid: &PowerGrid,
    solution: &PowerSolution,
    config: &ElectromigrationConfig,
) -> ElectromigrationReport {
    if solution.branch_currents.len() != grid.edges.len()
        || solution
            .branch_currents
            .iter()
            .enumerate()
            .any(|(i, b)| b.edge != i || !b.current_a.is_finite())
        || grid
            .edges
            .iter()
            .any(|e| e.from >= grid.nodes.len() || e.to >= grid.nodes.len())
    {
        return ElectromigrationReport::error("power solution does not correspond to this grid");
    }
    if !config.reference_temperature_c.is_finite()
        || config.reference_temperature_c <= -273.15
        || !config.activation_energy_ev.is_finite()
        || config.activation_energy_ev < 0.0
        || !config.current_exponent.is_finite()
        || config.current_exponent <= 0.0
        || config
            .max_temperature_c
            .is_some_and(|v| !v.is_finite() || v <= -273.15)
    {
        return ElectromigrationReport::error(
            "electromigration temperature/Arrhenius parameters are invalid",
        );
    }
    for (name, limit) in [
        (
            "default_max_current_density_a_per_um2",
            config.default_max_current_density_a_per_um2,
        ),
        (
            "default_max_current_per_cut_a",
            config.default_max_current_per_cut_a,
        ),
    ] {
        if limit.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            return ElectromigrationReport::error(format!(
                "electromigration limit {name} is invalid"
            ));
        }
    }

    let tref = config.reference_temperature_c + 273.15;
    let mut results = Vec::new();
    let mut violations = Vec::new();
    for (i, edge) in grid.edges.iter().enumerate() {
        if edge.em_exempt {
            continue;
        }
        if !edge.temperature_c.is_finite() || edge.temperature_c <= -273.15 {
            return ElectromigrationReport::error(format!(
                "edge '{}' has an invalid temperature",
                edge.id
            ));
        }
        if edge
            .blech_product_limit_a_per_um
            .is_some_and(|v| !v.is_finite() || v < 0.0)
        {
            return ElectromigrationReport::error(format!(
                "edge '{}' has an invalid Blech-product limit",
                edge.id
            ));
        }
        let current = solution.branch_currents[i].current_a.abs();
        let temp_k = edge.temperature_c + 273.15;
        let exponent = config.activation_energy_ev / (config.current_exponent * BOLTZMANN_EV_PER_K)
            * (1.0 / temp_k - 1.0 / tref);
        let derating = exponent.exp();
        if !derating.is_finite() {
            return ElectromigrationReport::error(format!(
                "edge '{}' temperature derating overflowed",
                edge.id
            ));
        }

        if let Some(max_temp) = config.max_temperature_c {
            if edge.temperature_c > max_temp {
                violations.push(SignoffViolation {
                    check: SignoffCheck::Electromigration,
                    rule_id: "em.temperature".into(),
                    message: format!(
                        "edge '{}' temperature {:.2}C exceeds {:.2}C",
                        edge.id, edge.temperature_c, max_temp
                    ),
                    location: Some((grid.nodes[edge.from].x, grid.nodes[edge.from].y)),
                    measured: Some(edge.temperature_c),
                    limit: Some(max_temp),
                    units: "degC".into(),
                });
            }
        }

        match edge.kind {
            PowerEdgeKind::Metal {
                width_um,
                thickness_um,
            } => {
                if !width_um.is_finite()
                    || width_um <= 0.0
                    || !thickness_um.is_finite()
                    || thickness_um <= 0.0
                {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has invalid metal dimensions",
                        edge.id
                    ));
                }
                if !edge.length_um.is_finite() || edge.length_um < 0.0 {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has an invalid metal length",
                        edge.id
                    ));
                }
                let Some(base_limit) = edge
                    .max_current_density_a_per_um2
                    .or(config.default_max_current_density_a_per_um2)
                else {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has no current-density limit",
                        edge.id
                    ));
                };
                if !base_limit.is_finite() || base_limit <= 0.0 {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has an invalid current-density limit",
                        edge.id
                    ));
                }
                let density = current / (width_um * thickness_um);
                let allowed = base_limit * derating;
                let blech_exempt = edge.blech_product_limit_a_per_um.is_some_and(|limit| {
                    limit.is_finite() && limit >= 0.0 && density * edge.length_um.max(0.0) <= limit
                });
                results.push(EmBranchResult {
                    edge: i,
                    id: edge.id.clone(),
                    current_a: current,
                    current_density_a_per_um2: Some(density),
                    allowed,
                    blech_exempt,
                });
                if !blech_exempt && density > allowed {
                    violations.push(SignoffViolation {
                        check: SignoffCheck::Electromigration,
                        rule_id: "em.metal_current_density".into(),
                        message: format!(
                            "edge '{}' current density {:.6} exceeds {:.6} A/um^2",
                            edge.id, density, allowed
                        ),
                        location: Some((grid.nodes[edge.from].x, grid.nodes[edge.from].y)),
                        measured: Some(density),
                        limit: Some(allowed),
                        units: "A/um^2".into(),
                    });
                }
            }
            PowerEdgeKind::Via { cuts } => {
                if cuts == 0 {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has zero via cuts",
                        edge.id
                    ));
                }
                let Some(per_cut) = edge
                    .max_current_per_cut_a
                    .or(config.default_max_current_per_cut_a)
                else {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has no per-cut current limit",
                        edge.id
                    ));
                };
                if !per_cut.is_finite() || per_cut <= 0.0 {
                    return ElectromigrationReport::error(format!(
                        "edge '{}' has an invalid per-cut current limit",
                        edge.id
                    ));
                }
                let allowed = per_cut * f64::from(cuts) * derating;
                results.push(EmBranchResult {
                    edge: i,
                    id: edge.id.clone(),
                    current_a: current,
                    current_density_a_per_um2: None,
                    allowed,
                    blech_exempt: false,
                });
                if current > allowed {
                    violations.push(SignoffViolation {
                        check: SignoffCheck::Electromigration,
                        rule_id: "em.via_current".into(),
                        message: format!(
                            "via '{}' current {:.6}A exceeds {:.6}A",
                            edge.id, current, allowed
                        ),
                        location: Some((grid.nodes[edge.from].x, grid.nodes[edge.from].y)),
                        measured: Some(current),
                        limit: Some(allowed),
                        units: "A".into(),
                    });
                }
            }
        }
    }
    results.sort_by_key(|r| r.edge);
    ElectromigrationReport {
        check: CheckReport::from_violations(SignoffCheck::Electromigration, violations, Vec::new()),
        branches: results,
    }
}
