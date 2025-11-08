//! AST tree visualization using box-drawing characters
//!
//! This module provides functions to display the parsed directive as a tree structure.

use crate::parser::{ClauseKind, Directive};

/// Display a directive as a formatted tree structure
pub fn display_ast_tree(directive: &Directive) -> String {
    let mut output = String::new();

    output.push_str("Directive\n");
    output.push_str(&format!("├─ name: \"{}\"\n", directive.name));

    // Parameter
    match &directive.parameter {
        Some(param) => output.push_str(&format!("├─ parameter: Some(\"{}\")\n", param)),
        None => output.push_str("├─ parameter: None\n"),
    }

    // Clauses
    if directive.clauses.is_empty() {
        output.push_str("└─ clauses: []\n");
    } else {
        output.push_str(&format!("└─ clauses: [{}]\n", directive.clauses.len()));

        for (idx, clause) in directive.clauses.iter().enumerate() {
            let is_last = idx == directive.clauses.len() - 1;
            let prefix = if is_last { "   └─ " } else { "   ├─ " };
            let continuation = if is_last { "      " } else { "   │  " };

            output.push_str(&format!("{}Clause\n", prefix));
            output.push_str(&format!("{}├─ name: \"{}\"\n", continuation, clause.name));

            match &clause.kind {
                ClauseKind::Bare => {
                    output.push_str(&format!("{}└─ kind: Bare\n", continuation));
                }
                ClauseKind::Parenthesized(args) => {
                    output.push_str(&format!(
                        "{}└─ kind: Parenthesized(\"{}\")\n",
                        continuation, args
                    ));
                }
                ClauseKind::VariableList(variables) => {
                    output.push_str(&format!(
                        "{}└─ kind: VariableList [{}]\n",
                        continuation,
                        variables.join(", ")
                    ));
                }
                ClauseKind::CopyinClause {
                    modifier,
                    variables,
                } => {
                    let mod_str = if modifier.is_some() { " readonly" } else { "" };
                    output.push_str(&format!(
                        "{}└─ kind: CopyinClause{} [{}]\n",
                        continuation,
                        mod_str,
                        variables.join(", ")
                    ));
                }
                ClauseKind::CopyoutClause {
                    modifier,
                    variables,
                } => {
                    let mod_str = if modifier.is_some() { " zero" } else { "" };
                    output.push_str(&format!(
                        "{}└─ kind: CopyoutClause{} [{}]\n",
                        continuation,
                        mod_str,
                        variables.join(", ")
                    ));
                }
                ClauseKind::CreateClause {
                    modifier,
                    variables,
                } => {
                    let mod_str = if modifier.is_some() { " zero" } else { "" };
                    output.push_str(&format!(
                        "{}└─ kind: CreateClause{} [{}]\n",
                        continuation,
                        mod_str,
                        variables.join(", ")
                    ));
                }
                ClauseKind::ReductionClause {
                    operator,
                    variables,
                } => {
                    output.push_str(&format!(
                        "{}└─ kind: ReductionClause {:?} [{}]\n",
                        continuation,
                        operator,
                        variables.join(", ")
                    ));
                }
                ClauseKind::GangClause {
                    modifier,
                    variables,
                } => {
                    let mod_str = match modifier {
                        Some(crate::parser::GangModifier::Num) => " num",
                        Some(crate::parser::GangModifier::Static) => " static",
                        None => "",
                    };
                    output.push_str(&format!(
                        "{}└─ kind: GangClause{} [{}]\n",
                        continuation,
                        mod_str,
                        variables.join(", ")
                    ));
                }
                ClauseKind::WorkerClause {
                    modifier,
                    variables,
                } => {
                    let mod_str = if modifier.is_some() { " num" } else { "" };
                    output.push_str(&format!(
                        "{}└─ kind: WorkerClause{} [{}]\n",
                        continuation,
                        mod_str,
                        variables.join(", ")
                    ));
                }
                ClauseKind::VectorClause {
                    modifier,
                    variables,
                } => {
                    let mod_str = if modifier.is_some() { " length" } else { "" };
                    output.push_str(&format!(
                        "{}└─ kind: VectorClause{} [{}]\n",
                        continuation,
                        mod_str,
                        variables.join(", ")
                    ));
                }
            }
        }
    }

    output
}

/// Display a compact representation of the directive
pub fn display_compact(directive: &Directive) -> String {
    let mut output = String::new();

    output.push_str(&format!("Directive {{ name: \"{}\"", directive.name));

    if let Some(ref param) = directive.parameter {
        output.push_str(&format!(", parameter: \"{}\"", param));
    }

    if !directive.clauses.is_empty() {
        output.push_str(", clauses: [");
        for (idx, clause) in directive.clauses.iter().enumerate() {
            if idx > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("{}", clause));
        }
        output.push(']');
    }

    output.push_str(" }");
    output
}

/// Display detailed step information in a formatted box
pub fn display_step_info(
    step: &super::DebugStep,
    total_steps: usize,
    original_input: &str,
) -> String {
    let mut output = String::new();
    let width = 70;

    // Top border
    output.push_str(&"═".repeat(width));
    output.push('\n');

    // Title
    let title = format!(
        "ROUP Parser Debugger - Step {}/{} - {}",
        step.step_number + 1,
        total_steps,
        step.kind.as_str()
    );
    output.push_str(&title);
    output.push('\n');

    output.push_str(&"═".repeat(width));
    output.push('\n');

    // Input with position indicator
    output.push_str("Input: ");
    output.push_str(original_input);
    output.push('\n');

    // Position indicator (show where we are in the input)
    output.push_str("       ");
    output.push_str(&" ".repeat(step.position));
    output.push('^');
    output.push('\n');
    output.push('\n');

    // Step description
    output.push_str(&format!("Step: {}\n", step.description));
    output.push('\n');

    // Token info (if any)
    if let Some(ref token_info) = step.token_info {
        output.push_str(&format!("Token: {}\n", token_info));
        output.push('\n');
    }

    // Consumed text
    if !step.consumed.is_empty() {
        output.push_str(&format!("Consumed: {}\n", step.consumed));
    }

    // Remaining text
    if !step.remaining.is_empty() {
        output.push_str(&format!("Remaining: \"{}\"\n", step.remaining.trim()));
    } else {
        output.push_str("Remaining: (none)\n");
    }
    output.push('\n');

    // Parser context
    if !step.context_stack.is_empty() {
        output.push_str("Parser context: ");
        output.push_str(&step.context_stack.join(" → "));
        output.push('\n');
        output.push('\n');
    }

    // Bottom border
    output.push_str(&"═".repeat(width));
    output.push('\n');

    output
}

/// Display help information
pub fn display_help() -> String {
    let mut output = String::new();
    output.push_str("═════════════════════════════════════════════════════════════\n");
    output.push_str("                    ROUP Debugger Help\n");
    output.push_str("═════════════════════════════════════════════════════════════\n");
    output.push('\n');
    output.push_str("Navigation Commands:\n");
    output.push_str("  n / →     Next step\n");
    output.push_str("  p / ←     Previous step\n");
    output.push_str("  g <num>   Go to specific step number\n");
    output.push_str("  0         Go to first step\n");
    output.push_str("  $         Go to last step\n");
    output.push('\n');
    output.push_str("Display Commands:\n");
    output.push_str("  a         Show complete AST tree\n");
    output.push_str("  s         Show current step details\n");
    output.push_str("  h         Show all steps history\n");
    output.push_str("  i         Show input again\n");
    output.push('\n');
    output.push_str("Other Commands:\n");
    output.push_str("  ?         Show this help\n");
    output.push_str("  q         Quit\n");
    output.push('\n');
    output.push_str("═════════════════════════════════════════════════════════════\n");
    output
}

/// Display all steps history
pub fn display_all_steps(steps: &[super::DebugStep]) -> String {
    let mut output = String::new();

    output.push_str("═════════════════════════════════════════════════════════════\n");
    output.push_str("                    All Parsing Steps\n");
    output.push_str("═════════════════════════════════════════════════════════════\n");
    output.push('\n');

    for step in steps {
        output.push_str(&format!(
            "{}. {} - {}\n",
            step.step_number + 1,
            step.kind.as_str(),
            step.description
        ));

        if !step.consumed.is_empty() && step.kind != super::StepKind::SkipWhitespace {
            output.push_str(&format!("   Consumed: {}\n", step.consumed));
        }

        if let Some(ref token_info) = step.token_info {
            output.push_str(&format!("   {}\n", token_info));
        }

        output.push('\n');
    }

    output.push_str("═════════════════════════════════════════════════════════════\n");
    output
}
