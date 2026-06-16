// Noun/verb confusion module
// This module contains various linters for detecting noun/verb confusion

use crate::{ExprLinter, Lint, LintKind, Suggestion};

mod effect_affect;

use effect_affect::effect_to_affect::EffectToAffect;

/// Combined Noun/Verb confusion linter
#[derive(Default)]
pub struct NounVerbConfusion {
    effect_to_affect: EffectToAffect,
}

impl NounVerbConfusion {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ExprLinter for NounVerbConfusion {
    fn get_exprs(&self) -> Vec<&dyn crate::expr::Expr> {
        vec![&self.effect_to_affect]
    }
}
