pub mod ast;
pub mod compile;
pub mod error;
pub mod eval;
pub mod expr_parser;
pub mod facts;
pub mod format;
mod fuzzy;
pub mod highlight;
pub mod ir;
pub mod parser;
pub mod query;
pub mod span;
pub mod validate;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub mod wasm;

pub use ast::{Program, Span};
pub use compile::{CompileError, compile_match_expr};
pub use error::{FormatError, ParseError};
pub use eval::{eval_predicate, eval_program};
pub use facts::{RequestFacts, RequestFactsBuilder};
pub use format::{
    DslFormatValidationResult, can_format_dsl, format_dsl, is_dsl_formatted, validate_dsl_document,
};
pub use highlight::HighlightSpan;
pub use ir::{EvalPlan, MatchProgram, Predicate};
pub use parser::{
    ParseProgramOutcome, has_parse_errors, mask_line_comments, normalize_logic_keywords,
    parse_program, parse_program_partial, prepare_source,
};
pub use validate::{Diagnostic, ValidationResult, collect_syntax_diagnostics, validate};
