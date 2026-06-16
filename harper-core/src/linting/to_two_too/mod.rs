// To/Two/Too confusion module
// This module contains various linters for detecting to/two/too confusion

use crate::{ExprLinter, Lint, LintKind, Suggestion};

mod to_too_adjective_end;
mod to_too_adjective_punct;
mod to_too_adverb;
mod to_too_chunk_start_comma;
mod to_too_pronoun_end;

use to_too_adjective_end::ToTooAdjectiveEnd;
use to_too_adjective_punct::ToTooAdjectivePunct;
use to_too_adverb::ToTooAdverb;
use to_too_chunk_start_comma::ToTooChunkStartComma;
use to_too_pronoun_end::ToTooPronounEnd;

/// Combined To/Two/Too linter that includes all the individual rules
#[derive(Default)]
pub struct ToTwoToo {
    adjective_end: ToTooAdjectiveEnd,
    adjective_punct: ToTooAdjectivePunct,
    adverb: ToTooAdverb,
    chunk_start_comma: ToTooChunkStartComma,
    pronoun_end: ToTooPronounEnd,
}

impl ToTwoToo {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ExprLinter for ToTwoToo {
    fn get_exprs(&self) -> Vec<&dyn crate::expr::Expr> {
        vec![
            &self.adjective_end,
            &self.adjective_punct,
            &self.adverb,
            &self.chunk_start_comma,
            &self.pronoun_end,
        ]
    }
}
