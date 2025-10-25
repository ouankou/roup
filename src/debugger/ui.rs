//! Interactive command-line interface for the debugger
//!
//! This module handles user input and displays the debugging session interactively.

use super::ast_display::{
    display_all_steps, display_ast_tree, display_compact, display_help, display_step_info,
};
use super::{DebugResult, DebugSession};
use std::io::{self, Write};

/// User commands for navigating the debug session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserCommand {
    /// Move to next step
    Next,
    /// Move to previous step
    Previous,
    /// Go to specific step number
    GoToStep(usize),
    /// Go to first step
    First,
    /// Go to last step
    Last,
    /// Show complete AST
    ShowAst,
    /// Show current step details
    ShowStep,
    /// Show all steps history
    ShowHistory,
    /// Show input
    ShowInput,
    /// Show help
    Help,
    /// Quit
    Quit,
    /// Invalid command
    Invalid(String),
}

impl UserCommand {
    /// Parse a command from user input
    pub fn from_input(input: &str) -> Self {
        let trimmed = input.trim();

        match trimmed {
            "n" | "next" | "→" => UserCommand::Next,
            "p" | "prev" | "previous" | "←" => UserCommand::Previous,
            "0" | "first" => UserCommand::First,
            "$" | "last" => UserCommand::Last,
            "a" | "ast" => UserCommand::ShowAst,
            "s" | "step" => UserCommand::ShowStep,
            "h" | "history" => UserCommand::ShowHistory,
            "i" | "input" => UserCommand::ShowInput,
            "?" | "help" => UserCommand::Help,
            "q" | "quit" | "exit" => UserCommand::Quit,
            _ => {
                // Try to parse as "g <num>" (go to step)
                if let Some(rest) = trimmed.strip_prefix("g ") {
                    if let Ok(num) = rest.trim().parse::<usize>() {
                        return UserCommand::GoToStep(num);
                    }
                }
                // Try parsing as a number directly
                if let Ok(num) = trimmed.parse::<usize>() {
                    return UserCommand::GoToStep(num);
                }

                UserCommand::Invalid(trimmed.to_string())
            }
        }
    }
}

/// Run an interactive debugging session
pub fn run_interactive_session(mut session: DebugSession) -> DebugResult<()> {
    println!("\n{}", display_help());
    println!("Press Enter to begin stepping...\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Display initial step
    display_current_state(&session);

    loop {
        print!("\n[n]ext [p]rev [a]st [h]istory [?]help [q]uit > ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;

        let command = UserCommand::from_input(&input);

        match command {
            UserCommand::Next => {
                if session.next_step() {
                    display_current_state(&session);
                } else {
                    println!("Already at the last step.");
                }
            }
            UserCommand::Previous => {
                if session.prev_step() {
                    display_current_state(&session);
                } else {
                    println!("Already at the first step.");
                }
            }
            UserCommand::GoToStep(num) => {
                if num == 0 {
                    println!("Step numbers start from 1. Use command '0' or 'first' to go to the first step.");
                } else if num > session.total_steps() {
                    println!(
                        "Step {} doesn't exist. Total steps: {}",
                        num,
                        session.total_steps()
                    );
                } else {
                    session.current_step_index = num - 1;
                    display_current_state(&session);
                }
            }
            UserCommand::First => {
                session.current_step_index = 0;
                display_current_state(&session);
            }
            UserCommand::Last => {
                session.current_step_index = session.total_steps().saturating_sub(1);
                display_current_state(&session);
            }
            UserCommand::ShowAst => {
                if let Some(ref directive) = session.final_directive {
                    println!("\n{}", display_ast_tree(directive));
                } else {
                    println!("\nNo AST available (parsing may have failed).");
                }
            }
            UserCommand::ShowStep => {
                display_current_state(&session);
            }
            UserCommand::ShowHistory => {
                println!("\n{}", display_all_steps(&session.steps));
            }
            UserCommand::ShowInput => {
                println!("\nOriginal input:");
                println!("{}", session.original_input);
            }
            UserCommand::Help => {
                println!("\n{}", display_help());
            }
            UserCommand::Quit => {
                println!("Exiting debugger.");
                break;
            }
            UserCommand::Invalid(cmd) => {
                println!("Unknown command: '{}'. Type '?' for help.", cmd);
            }
        }
    }

    Ok(())
}

/// Display the current state of the session
fn display_current_state(session: &DebugSession) {
    if let Some(step) = session.current_step() {
        println!(
            "\n{}",
            display_step_info(step, session.total_steps(), &session.original_input)
        );

        // Show compact directive representation at current step if we have parsed something
        if session.current_step_index > 0 {
            if let Some(ref directive) = session.final_directive {
                println!("Current directive state: {}", display_compact(directive));
                println!();
            }
        }
    } else {
        println!("No step available.");
    }
}

/// Run the debugger in non-interactive mode (just show all steps)
pub fn run_non_interactive(session: &DebugSession) {
    println!("\n{}", display_all_steps(&session.steps));

    if let Some(ref directive) = session.final_directive {
        println!("\nFinal AST:");
        println!("{}", display_ast_tree(directive));

        println!("\nPragma string:");
        println!("{}", directive.to_pragma_string());
    } else if let Some(ref error) = session.error {
        println!("\nParsing failed:");
        println!("{}", error);
    }
}
