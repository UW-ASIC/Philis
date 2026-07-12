use crate::types::{ConstraintContract, ConstraintStatus};

/// Summary of constraint coverage validation.
#[derive(Debug, Clone)]
pub struct ContractValidation {
    pub total: usize,
    pub emitted: usize,
    pub consumed: usize,
    pub satisfied: usize,
    pub violated: usize,
    pub waived: usize,
    pub missing_consumers: Vec<String>,
    pub hard_violations: Vec<String>,
}

/// Validate a set of contracts and return coverage summary.
#[must_use]
pub fn validate_constraint_record(contracts: &[ConstraintContract]) -> ContractValidation {
    let mut v = ContractValidation {
        total: contracts.len(),
        emitted: 0,
        consumed: 0,
        satisfied: 0,
        violated: 0,
        waived: 0,
        missing_consumers: Vec::new(),
        hard_violations: Vec::new(),
    };
    for c in contracts {
        match c.status {
            ConstraintStatus::Emitted => {
                v.emitted += 1;
                v.missing_consumers.push(c.constraint_id.clone());
            }
            ConstraintStatus::Consumed => v.consumed += 1,
            ConstraintStatus::Satisfied => v.satisfied += 1,
            ConstraintStatus::Violated => {
                v.violated += 1;
                if c.strength == crate::types::ConstraintStrength::Hard && c.waiver_reason.is_none()
                {
                    v.hard_violations.push(c.constraint_id.clone());
                }
            }
            ConstraintStatus::Waived => v.waived += 1,
        }
    }
    v
}
