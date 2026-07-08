//! Injector isolation: series-R test + private rings + cluster exclusion.
//!
//! AOAL ch14 14.1.5 three-question checklist + Medium-Risk Mergers.
//!
//! (a) Walk pad nets through resistors summing series R, flag < 1kOhm as injector.
//! (b) Injector devices get private guard rings (never shared).
//! (c) `find_sharing_clusters` excludes injectors → singleton clusters.

use std::collections::{HashMap, HashSet};

use crate::types::DeviceId;

/// Result of series-R walk from a pad net.
#[derive(Debug, Clone)]
pub struct InjectorCandidate {
    pub device_id: DeviceId,
    /// Total series resistance from pad to device (ohms).
    pub series_r_ohm: f64,
    /// Whether this device is flagged as a substrate injector.
    pub is_injector: bool,
}

/// Walk a simple resistor chain from a pad net, summing series R.
/// `resistors` maps (net_a, net_b) -> resistance in ohms.
/// `pad_nets` is the set of net IDs directly connected to pads.
/// `device_nets` maps device_id -> set of net IDs the device touches.
///
/// A device with total series-R < `threshold_ohm` from any pad net
/// is flagged as a potential substrate injector.
pub fn detect_injectors(
    pad_nets: &HashSet<u32>,
    resistors: &[(u32, u32, f64)], // (net_a, net_b, R_ohm)
    device_nets: &HashMap<DeviceId, Vec<u32>>,
    threshold_ohm: f64,
) -> Vec<InjectorCandidate> {
    // Build adjacency: net -> [(neighbor_net, R)]
    let mut adj: HashMap<u32, Vec<(u32, f64)>> = HashMap::new();
    for &(a, b, r) in resistors {
        adj.entry(a).or_default().push((b, r));
        adj.entry(b).or_default().push((a, r));
    }

    // For each device, BFS/DFS from its nets back to pad nets, finding min series R
    let mut results = Vec::new();
    for (&dev_id, nets) in device_nets {
        let mut min_r = f64::MAX;
        for &net in nets {
            if pad_nets.contains(&net) {
                min_r = 0.0;
                break;
            }
            // Simple DFS to find shortest R path to any pad net
            let mut visited = HashSet::new();
            let mut stack: Vec<(u32, f64)> = vec![(net, 0.0)];
            while let Some((n, r_so_far)) = stack.pop() {
                if !visited.insert(n) {
                    continue;
                }
                if pad_nets.contains(&n) {
                    min_r = min_r.min(r_so_far);
                    continue;
                }
                if let Some(neighbors) = adj.get(&n) {
                    for &(nb, r) in neighbors {
                        if !visited.contains(&nb) {
                            stack.push((nb, r_so_far + r));
                        }
                    }
                }
            }
        }
        results.push(InjectorCandidate {
            device_id: dev_id,
            series_r_ohm: min_r,
            is_injector: min_r < threshold_ohm,
        });
    }
    results
}

/// Filter sharing clusters to exclude injector devices.
/// Injectors always get singleton clusters (private guard rings).
pub fn exclude_injectors_from_clusters(
    clusters: &mut Vec<Vec<DeviceId>>,
    injectors: &HashSet<DeviceId>,
) {
    let mut new_clusters = Vec::new();
    for cluster in clusters.drain(..) {
        let (inj, clean): (Vec<_>, Vec<_>) =
            cluster.into_iter().partition(|d| injectors.contains(d));
        if !clean.is_empty() {
            new_clusters.push(clean);
        }
        // Each injector becomes its own singleton cluster
        for d in inj {
            new_clusters.push(vec![d]);
        }
    }
    *clusters = new_clusters;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_series_r_detected() {
        let pad_nets: HashSet<u32> = [10].into();
        let resistors = vec![(10, 20, 500.0)]; // 500 ohm pad-to-device net
        let mut device_nets = HashMap::new();
        device_nets.insert(DeviceId(0), vec![20]);
        device_nets.insert(DeviceId(1), vec![30]); // not connected to pad

        let results = detect_injectors(&pad_nets, &resistors, &device_nets, 1000.0);
        let d0 = results.iter().find(|r| r.device_id == DeviceId(0)).unwrap();
        assert!(d0.is_injector, "500 ohm < 1k threshold");
        let d1 = results.iter().find(|r| r.device_id == DeviceId(1)).unwrap();
        assert!(!d1.is_injector, "no path to pad");
    }

    #[test]
    fn cluster_exclusion() {
        let injectors: HashSet<DeviceId> = [DeviceId(1)].into();
        let mut clusters = vec![vec![DeviceId(0), DeviceId(1), DeviceId(2)]];
        exclude_injectors_from_clusters(&mut clusters, &injectors);
        assert_eq!(clusters.len(), 2);
        assert!(clusters.iter().any(|c| c == &[DeviceId(0), DeviceId(2)]));
        assert!(clusters.iter().any(|c| c == &[DeviceId(1)]));
    }
}
