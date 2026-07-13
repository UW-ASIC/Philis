//! ERC — electrical rule checking on extracted layout.
//!
//! Operates on a GeometryStore + Deck, using LVS extraction results for
//! connectivity. One rule per file in `rules/`, globbed at compile time by
//! build.rs; each implements [`crate::rule::Rule`] over [`ErcCtx`].

use crate::backend::Backend;
use crate::geometry::GeometryStore;
use crate::lvs::{extract_netlist, ExtractedNetlist};
use crate::params::Deck;

#[derive(Debug, Clone)]
pub struct ErcViolation {
    pub check: String,
    pub detail: String,
    pub x: i32,
    pub y: i32,
}

pub struct ErcReport {
    pub violations: Vec<ErcViolation>,
}

impl ErcReport {
    pub fn by_check(&self, check: &str) -> Vec<&ErcViolation> {
        self.violations.iter().filter(|v| v.check == check).collect()
    }
}

/// Everything every ERC rule reads. The netlist is extracted once per run
/// (exactly as before, when it was shared through an `Arc`) and lent to the
/// rules by reference.
#[derive(Clone, Copy)]
pub struct ErcCtx<'a> {
    pub store: &'a GeometryStore,
    pub deck: &'a Deck,
    pub ext: &'a ExtractedNetlist,
}

/// A boxed ERC rule, usable with any context lifetime.
pub type BoxedRule = Box<dyn for<'a> crate::rule::Rule<ErcCtx<'a>, Finding = ErcViolation>>;

pub mod rules {
    use super::BoxedRule;
    /// One per rule file: build the rule from the deck, or `None` when the
    /// rule is inapplicable for this deck.
    pub type Factory = fn(&crate::params::Deck) -> Option<BoxedRule>;
    include!(concat!(env!("OUT_DIR"), "/erc_rules.rs"));
}

pub use rules::multiple_drivers::MultipleDriverCheck;
pub use rules::tie_high_low::TieHighLowCheck;

pub fn run_erc(store: &GeometryStore, deck: &Deck) -> ErcReport {
    let ext = match extract_netlist(store, deck) {
        Ok(ext) => ext,
        Err(e) => {
            return ErcReport {
                violations: vec![ErcViolation {
                    check: "erc_extraction_error".into(),
                    detail: format!("ERC connectivity extraction failed: {e}"),
                    x: 0,
                    y: 0,
                }],
            }
        }
    };
    let ctx = ErcCtx { store, deck, ext: &ext };
    let rules: Vec<BoxedRule> = rules::FACTORIES.iter().filter_map(|f| f(deck)).collect();
    let violations = crate::rule::run_rules(&rules, &ctx, Backend::Cpu);
    ErcReport { violations }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_failure_is_not_clean() {
        let deck = Deck::from_json(r#"{
            "layers": {"met1": {"layer": 1, "datatype": 0}},
            "drc": {},
            "pex": {}
        }"#).unwrap();
        let report = run_erc(&GeometryStore::new(), &deck);
        assert_eq!(report.by_check("erc_extraction_error").len(), 1);
    }
}
