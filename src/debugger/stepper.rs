//! Step-by-step parsing instrumentation
//!
//! This module implements the core stepping logic that breaks down the parsing process
//! into discrete, observable steps.

use crate::lexer::{self, Language};
use crate::parser::{openacc, openmp, Clause, Dialect, Directive};
use std::borrow::Cow;

use super::{DebugConfig, DebugError, DebugResult};

/// Represents a single step in the parsing process
#[derive(Debug, Clone)]
pub struct DebugStep {
    /// Sequential step number (0-indexed)
    pub step_number: usize,
    /// Type/category of this step
    pub kind: StepKind,
    /// Description of what's happening in this step
    pub description: String,
    /// The portion of input consumed in this step
    pub consumed: String,
    /// The remaining unparsed text after this step
    pub remaining: String,
    /// Current position in the original input
    pub position: usize,
    /// Parser context stack (e.g., ["DirectiveRegistry::parse", "ClauseRegistry::parse_sequence"])
    pub context_stack: Vec<String>,
    /// Token or structure created in this step (if any)
    pub token_info: Option<String>,
}

/// Categories of parsing steps
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Skipping whitespace or comments
    SkipWhitespace,
    /// Parsing the pragma prefix (#pragma omp, !$omp, etc.)
    PragmaPrefix,
    /// Parsing a directive name (parallel, for, etc.)
    DirectiveName,
    /// Parsing directive parameter (e.g., "parallel(n)" -> parameter is "n")
    DirectiveParameter,
    /// Parsing a clause name (shared, private, etc.)
    ClauseName,
    /// Parsing clause arguments (the content inside parentheses)
    ClauseArguments,
    /// Completed parsing entire directive
    Complete,
    /// Error occurred
    Error,
}

impl StepKind {
    pub fn as_str(&self) -> &str {
        match self {
            StepKind::SkipWhitespace => "Skip Whitespace",
            StepKind::PragmaPrefix => "Pragma Prefix",
            StepKind::DirectiveName => "Directive Name",
            StepKind::DirectiveParameter => "Directive Parameter",
            StepKind::ClauseName => "Clause Name",
            StepKind::ClauseArguments => "Clause Arguments",
            StepKind::Complete => "Complete",
            StepKind::Error => "Error",
        }
    }
}

/// A complete debugging session that tracks all steps
#[derive(Debug)]
pub struct DebugSession {
    /// Original input string
    pub original_input: String,
    /// Configuration for this session
    pub config: DebugConfig,
    /// All steps collected during parsing
    pub steps: Vec<DebugStep>,
    /// Final parsed directive (if successful)
    pub final_directive: Option<Directive<'static>>,
    /// Parse error (if failed)
    pub error: Option<String>,
    /// Current step index for interactive navigation
    pub current_step_index: usize,
}

impl DebugSession {
    /// Create a new debug session and parse the input step-by-step
    pub fn new(input: &str, config: DebugConfig) -> DebugResult<Self> {
        let mut session = Self {
            original_input: input.to_string(),
            config: config.clone(),
            steps: Vec::new(),
            final_directive: None,
            error: None,
            current_step_index: 0,
        };

        session.parse_step_by_step()?;
        Ok(session)
    }

    /// Parse the input and collect all steps
    fn parse_step_by_step(&mut self) -> DebugResult<()> {
        // Clone the input to avoid lifetime issues with the borrow checker
        let input_copy = self.original_input.clone();
        let dialect = self.config.dialect;
        let language = self.config.language;

        let mut current_input = input_copy.as_str();
        let mut position = 0;
        let mut step_number = 0;
        let mut context_stack = vec!["Parser::parse".to_string()];

        // Step 1: Handle line continuations (preprocessing step)
        let normalized = lexer::collapse_line_continuations(current_input);
        if normalized != current_input {
            let consumed = format!(
                "Normalized {} line continuation(s)",
                current_input.matches('\\').count()
            );
            self.steps.push(DebugStep {
                step_number,
                kind: StepKind::SkipWhitespace,
                description: "Collapse line continuations".to_string(),
                consumed,
                remaining: normalized.to_string(),
                position,
                context_stack: context_stack.clone(),
                token_info: Some(format!("Normalized input: {}", normalized)),
            });
            step_number += 1;
            current_input = normalized.as_ref();
        }

        // Step 2: Skip leading whitespace
        match lexer::skip_space_and_comments(current_input) {
            Ok((remaining, _)) => {
                let consumed_len = current_input.len() - remaining.len();
                if consumed_len > 0 {
                    let consumed = &current_input[..consumed_len];
                    self.steps.push(DebugStep {
                        step_number,
                        kind: StepKind::SkipWhitespace,
                        description: "Skip leading whitespace/comments".to_string(),
                        consumed: format!("{:?}", consumed),
                        remaining: remaining.to_string(),
                        position,
                        context_stack: context_stack.clone(),
                        token_info: None,
                    });
                    step_number += 1;
                    position += consumed_len;
                    current_input = remaining;
                }
            }
            Err(e) => {
                return Err(DebugError::ParseError(format!(
                    "Failed to skip whitespace: {:?}",
                    e
                )));
            }
        }

        // Step 3: Parse pragma prefix / Fortran sentinel
        context_stack.push("lex_pragma".to_string());
        let (prefix, remaining_after_prefix) = Self::parse_pragma_prefix_static(
            current_input,
            step_number,
            position,
            &context_stack,
            &mut self.steps,
            dialect,
            language,
        )?;

        step_number += 1;
        position += current_input.len() - remaining_after_prefix.len();
        current_input = remaining_after_prefix;
        context_stack.pop();

        // Check if dialect keyword was already consumed in the prefix/sentinel
        // For Fortran full forms (!$omp, c$acc, etc.), the dialect is included
        // For C and Fortran short forms (#pragma, !$, c$), we need to consume it separately
        let dialect_keyword = match dialect {
            Dialect::OpenMp => "omp",
            Dialect::OpenAcc => "acc",
        };
        let dialect_already_consumed = prefix.to_lowercase().contains(dialect_keyword);

        // Step 4: Skip whitespace and consume dialect keyword (omp/acc)
        // Only do this if the dialect wasn't already consumed in the sentinel
        if !dialect_already_consumed {
            if let Ok((remaining, _)) = lexer::skip_space_and_comments(current_input) {
                let consumed_len = current_input.len() - remaining.len();
                if consumed_len > 0 {
                    self.steps.push(DebugStep {
                        step_number,
                        kind: StepKind::SkipWhitespace,
                        description: "Skip whitespace after pragma".to_string(),
                        consumed: format!("{:?}", &current_input[..consumed_len]),
                        remaining: remaining.to_string(),
                        position,
                        context_stack: context_stack.clone(),
                        token_info: None,
                    });
                    step_number += 1;
                    position += consumed_len;
                    current_input = remaining;
                }
            }

            // Consume dialect keyword (omp or acc)
            if current_input.starts_with(dialect_keyword) {
                self.steps.push(DebugStep {
                    step_number,
                    kind: StepKind::PragmaPrefix, // Treat dialect as part of pragma
                    description: format!("Parse dialect keyword '{}'", dialect_keyword),
                    consumed: dialect_keyword.to_string(),
                    remaining: current_input[dialect_keyword.len()..].to_string(),
                    position,
                    context_stack: context_stack.clone(),
                    token_info: Some(format!("Dialect: \"{}\"", dialect_keyword)),
                });
                step_number += 1;
                position += dialect_keyword.len();
                current_input = &current_input[dialect_keyword.len()..];
            }
        }

        // Step 5: Use the full parser to get the directive and collect more detailed steps
        context_stack.push("DirectiveRegistry::parse".to_string());
        let parser = match dialect {
            Dialect::OpenMp => openmp::parser().with_language(language),
            Dialect::OpenAcc => openacc::parser().with_language(language),
        };

        // Parse the directive (using the copy)
        match parser.parse(&input_copy) {
            Ok((remaining, directive)) => {
                // Now we need to decompose the directive into steps
                self.decompose_directive(
                    &directive,
                    current_input,
                    &mut step_number,
                    &mut position,
                    &context_stack,
                )?;

                // Convert Directive<'_> to Directive<'static> by cloning all Cow fields
                // We need to explicitly collect into a Vec first to avoid lifetime issues
                let owned_clauses: Vec<Clause<'static>> = directive
                    .clauses
                    .iter()
                    .map(|c| Clause {
                        name: Cow::Owned(c.name.to_string()),
                        kind: match &c.kind {
                            crate::parser::ClauseKind::Bare => crate::parser::ClauseKind::Bare,
                            crate::parser::ClauseKind::Parenthesized(s) => {
                                crate::parser::ClauseKind::Parenthesized(Cow::Owned(s.to_string()))
                            }
                            crate::parser::ClauseKind::VariableList(variables) => {
                                crate::parser::ClauseKind::VariableList(
                                    variables
                                        .iter()
                                        .map(|v| Cow::Owned(v.to_string()))
                                        .collect(),
                                )
                            }
                            crate::parser::ClauseKind::CopyinClause {
                                modifier,
                                variables,
                            } => crate::parser::ClauseKind::CopyinClause {
                                modifier: *modifier,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                            },
                            crate::parser::ClauseKind::CopyoutClause {
                                modifier,
                                variables,
                            } => crate::parser::ClauseKind::CopyoutClause {
                                modifier: *modifier,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                            },
                            crate::parser::ClauseKind::CreateClause {
                                modifier,
                                variables,
                            } => crate::parser::ClauseKind::CreateClause {
                                modifier: *modifier,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                            },
                            crate::parser::ClauseKind::ReductionClause {
                                operator,
                                variables,
                                space_after_colon,
                            } => crate::parser::ClauseKind::ReductionClause {
                                operator: *operator,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                                space_after_colon: *space_after_colon,
                            },
                            crate::parser::ClauseKind::GangClause {
                                modifier,
                                variables,
                            } => crate::parser::ClauseKind::GangClause {
                                modifier: *modifier,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                            },
                            crate::parser::ClauseKind::WorkerClause {
                                modifier,
                                variables,
                            } => crate::parser::ClauseKind::WorkerClause {
                                modifier: *modifier,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                            },
                            crate::parser::ClauseKind::VectorClause {
                                modifier,
                                variables,
                            } => crate::parser::ClauseKind::VectorClause {
                                modifier: *modifier,
                                variables: variables
                                    .iter()
                                    .map(|v| Cow::Owned(v.to_string()))
                                    .collect(),
                            },
                        },
                    })
                    .collect();

                let static_directive = Directive {
                    name: directive.name.clone(),
                    parameter: directive
                        .parameter
                        .as_ref()
                        .map(|p| Cow::Owned(p.to_string())),
                    clauses: owned_clauses,
                    wait_data: directive.wait_data.as_ref().map(|wd| {
                        crate::parser::WaitDirectiveData {
                            devnum: wd.devnum.as_ref().map(|d| Cow::Owned(d.to_string())),
                            has_queues: wd.has_queues,
                            queue_exprs: wd
                                .queue_exprs
                                .iter()
                                .map(|e| Cow::Owned(e.to_string()))
                                .collect(),
                        }
                    }),
                    cache_data: directive.cache_data.as_ref().map(|cd| {
                        crate::parser::CacheDirectiveData {
                            readonly: cd.readonly,
                            variables: cd
                                .variables
                                .iter()
                                .map(|v| Cow::Owned(v.to_string()))
                                .collect(),
                        }
                    }),
                };

                self.final_directive = Some(static_directive);

                // Final step: Complete
                self.steps.push(DebugStep {
                    step_number,
                    kind: StepKind::Complete,
                    description: "Parsing complete".to_string(),
                    consumed: String::new(),
                    remaining: remaining.to_string(),
                    position,
                    context_stack: vec!["Parser::parse".to_string()],
                    token_info: Some("Successfully parsed directive".to_string()),
                });
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                self.error = Some(error_msg.clone());
                self.steps.push(DebugStep {
                    step_number,
                    kind: StepKind::Error,
                    description: "Parse error".to_string(),
                    consumed: String::new(),
                    remaining: current_input.to_string(),
                    position,
                    context_stack,
                    token_info: Some(error_msg.clone()),
                });
                return Err(DebugError::ParseError(error_msg));
            }
        }

        Ok(())
    }

    /// Parse the pragma prefix and add step (static version to avoid borrow checker issues)
    fn parse_pragma_prefix_static<'a>(
        input: &'a str,
        step_number: usize,
        position: usize,
        context_stack: &[String],
        steps: &mut Vec<DebugStep>,
        dialect: Dialect,
        language: Language,
    ) -> DebugResult<(String, &'a str)> {
        let dialect_keyword = match dialect {
            Dialect::OpenMp => "omp",
            Dialect::OpenAcc => "acc",
        };

        let result = match language {
            Language::C => lexer::lex_pragma(input),
            Language::FortranFree => {
                lexer::lex_fortran_free_sentinel_with_prefix(input, dialect_keyword)
            }
            Language::FortranFixed => {
                lexer::lex_fortran_fixed_sentinel_with_prefix(input, dialect_keyword)
            }
        };

        match result {
            Ok((remaining, prefix)) => {
                let description = match language {
                    Language::C => "Parse pragma prefix".to_string(),
                    Language::FortranFree => "Parse Fortran free-form sentinel".to_string(),
                    Language::FortranFixed => "Parse Fortran fixed-form sentinel".to_string(),
                };

                steps.push(DebugStep {
                    step_number,
                    kind: StepKind::PragmaPrefix,
                    description,
                    consumed: prefix.to_string(),
                    remaining: remaining.to_string(),
                    position,
                    context_stack: context_stack.to_vec(),
                    token_info: Some(format!("Sentinel: \"{}\"", prefix)),
                });
                Ok((prefix.to_string(), remaining))
            }
            Err(e) => {
                let lang_str = match language {
                    Language::C => "C pragma prefix",
                    Language::FortranFree => "Fortran free-form sentinel",
                    Language::FortranFixed => "Fortran fixed-form sentinel",
                };
                Err(DebugError::ParseError(format!(
                    "Failed to parse {}: {:?}",
                    lang_str, e
                )))
            }
        }
    }

    /// Decompose a parsed directive into individual steps
    fn decompose_directive(
        &mut self,
        directive: &Directive,
        mut current_input: &str,
        step_number: &mut usize,
        position: &mut usize,
        context_stack: &[String],
    ) -> DebugResult<()> {
        // Skip whitespace before directive name
        if let Ok((remaining, _)) = lexer::skip_space_and_comments(current_input) {
            let consumed_len = current_input.len() - remaining.len();
            if consumed_len > 0 {
                let consumed = &current_input[..consumed_len];
                self.steps.push(DebugStep {
                    step_number: *step_number,
                    kind: StepKind::SkipWhitespace,
                    description: "Skip whitespace before directive".to_string(),
                    consumed: format!("{:?}", consumed),
                    remaining: remaining.to_string(),
                    position: *position,
                    context_stack: context_stack.to_vec(),
                    token_info: None,
                });
                *step_number += 1;
                *position += consumed_len;
                current_input = remaining;
            }
        }

        // Directive name
        self.steps.push(DebugStep {
            step_number: *step_number,
            kind: StepKind::DirectiveName,
            description: format!("Parse directive name '{}'", directive.name),
            consumed: directive.name.to_string(),
            remaining: current_input[directive.name.len()..].to_string(),
            position: *position,
            context_stack: context_stack.to_vec(),
            token_info: Some(format!("Directive: \"{}\"", directive.name)),
        });
        *step_number += 1;
        *position += directive.name.len();
        current_input = &current_input[directive.name.len()..];

        // Directive parameter (if present)
        if let Some(ref param) = directive.parameter {
            // Skip whitespace before parameter
            if let Ok((remaining, _)) = lexer::skip_space_and_comments(current_input) {
                let consumed_len = current_input.len() - remaining.len();
                if consumed_len > 0 {
                    *position += consumed_len;
                    current_input = remaining;
                }
            }

            // The parameter may or may not have parentheses already
            // For scan: parameter is "exclusive(x, y)" which needs wrapping: "(exclusive(x, y))"
            // For allocate: parameter might be "(x, y)" already with parens
            let param_str = param.as_ref();
            let param_with_parens = if param_str.starts_with('(') {
                // Already has parentheses
                param_str.to_string()
            } else {
                // Needs parentheses wrapped
                format!("({})", param_str)
            };

            // Safely compute remaining based on actual consumed length
            let consumed_len = param_with_parens.len().min(current_input.len());
            let remaining_text = if consumed_len < current_input.len() {
                current_input[consumed_len..].to_string()
            } else {
                String::new()
            };

            self.steps.push(DebugStep {
                step_number: *step_number,
                kind: StepKind::DirectiveParameter,
                description: format!("Parse directive parameter '{}'", param),
                consumed: param_with_parens.clone(),
                remaining: remaining_text,
                position: *position,
                context_stack: context_stack.to_vec(),
                token_info: Some(format!("Parameter: \"{}\"", param)),
            });
            *step_number += 1;
            *position += consumed_len;
            if consumed_len < current_input.len() {
                current_input = &current_input[consumed_len..];
            } else {
                current_input = "";
            }
        }

        // Parse clauses
        let mut clause_context = context_stack.to_vec();
        clause_context.push("ClauseRegistry::parse_sequence".to_string());

        for clause in &directive.clauses {
            // Skip whitespace before clause
            if let Ok((remaining, _)) = lexer::skip_space_and_comments(current_input) {
                let consumed_len = current_input.len() - remaining.len();
                if consumed_len > 0 {
                    let consumed = &current_input[..consumed_len];
                    self.steps.push(DebugStep {
                        step_number: *step_number,
                        kind: StepKind::SkipWhitespace,
                        description: "Skip whitespace before clause".to_string(),
                        consumed: format!("{:?}", consumed),
                        remaining: remaining.to_string(),
                        position: *position,
                        context_stack: clause_context.clone(),
                        token_info: None,
                    });
                    *step_number += 1;
                    *position += consumed_len;
                    current_input = remaining;
                }
            }

            // Clause name
            self.steps.push(DebugStep {
                step_number: *step_number,
                kind: StepKind::ClauseName,
                description: format!("Parse clause name '{}'", clause.name),
                consumed: clause.name.to_string(),
                remaining: current_input[clause.name.len()..].to_string(),
                position: *position,
                context_stack: clause_context.clone(),
                token_info: Some(format!("Clause: \"{}\"", clause.name)),
            });
            *step_number += 1;
            *position += clause.name.len();
            current_input = &current_input[clause.name.len()..];

            // Clause arguments (if parenthesized)
            if let crate::parser::ClauseKind::Parenthesized(ref args) = clause.kind {
                let args_with_parens = format!("({})", args);
                self.steps.push(DebugStep {
                    step_number: *step_number,
                    kind: StepKind::ClauseArguments,
                    description: format!("Parse clause arguments '{}'", args),
                    consumed: args_with_parens.clone(),
                    remaining: if current_input.len() >= args_with_parens.len() {
                        current_input[args_with_parens.len()..].to_string()
                    } else {
                        String::new()
                    },
                    position: *position,
                    context_stack: clause_context.clone(),
                    token_info: Some(format!("Arguments: \"{}\"", args)),
                });
                *step_number += 1;
                *position += args_with_parens.len();
                if current_input.len() >= args_with_parens.len() {
                    current_input = &current_input[args_with_parens.len()..];
                }
            }
        }

        Ok(())
    }

    /// Get the current step
    pub fn current_step(&self) -> Option<&DebugStep> {
        self.steps.get(self.current_step_index)
    }

    /// Move to the next step
    pub fn next_step(&mut self) -> bool {
        if self.current_step_index < self.steps.len() - 1 {
            self.current_step_index += 1;
            true
        } else {
            false
        }
    }

    /// Move to the previous step
    pub fn prev_step(&mut self) -> bool {
        if self.current_step_index > 0 {
            self.current_step_index -= 1;
            true
        } else {
            false
        }
    }

    /// Get total number of steps
    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    /// Get all steps up to and including the current step
    pub fn steps_so_far(&self) -> &[DebugStep] {
        &self.steps[..=self
            .current_step_index
            .min(self.steps.len().saturating_sub(1))]
    }
}
