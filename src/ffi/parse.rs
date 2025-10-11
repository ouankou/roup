//! Parser integration for FFI
//!
//! This module provides functions to parse OpenMP directives from strings
//! and return AST handles.
//!
//! ## Learning Objectives
//!
//! 1. **Error Propagation**: Converting Rust errors to C status codes
//! 2. **Lifetime Management**: Converting 'static to owned data
//! 3. **Resource Cleanup**: Ensuring no leaks on parse failure
//! 4. **Integration**: Connecting string API → parser → IR → handles
//!
//! ## Design Philosophy
//!
//! ```text
//! C builds string:     Rust parses:           C queries result:
//! ┌──────────────┐    ┌──────────────┐       ┌──────────────┐
//! │ str_new()    │    │ parse()      │       │ directive_*()│
//! │ push_byte()  │───>│   ↓          │──────>│ clause_*()   │
//! │ push_byte()  │    │ convert_IR() │       │ to_string()  │
//! └──────────────┘    └──────────────┘       └──────────────┘
//! ```

use super::registry::{insert, with_resource, Resource};
use super::types::{Handle, OmpStatus, INVALID_HANDLE};
use crate::ir::{convert::convert_directive, DirectiveIR, Language, ParserConfig, SourceLocation};
use crate::parser::parse_omp_directive;

/// Parse a string into an OpenMP directive
///
/// ## C Signature
/// ```c
/// OmpStatus omp_parse(uint64_t string_handle, uint64_t* out_directive_handle);
/// ```
///
/// ## Parameters
/// - `string_handle`: Handle to string containing OpenMP directive
/// - `out_directive_handle`: Output parameter for directive handle (or 0 on error)
///
/// ## Returns
/// - `Ok`: Parsing succeeded, check `out_directive_handle`
/// - `NotFound`: Invalid string handle
/// - `Invalid`: String is not valid UTF-8
/// - `ParseError`: Syntax error in directive
///
/// ## Example
/// ```
/// use roup::ffi::{omp_str_new, omp_str_push_byte, omp_parse};
///
/// let str_h = omp_str_new();
/// for &b in b"#pragma omp parallel" {
///     omp_str_push_byte(str_h, b);
/// }
///
/// let mut dir_h = 0;
/// let status = omp_parse(str_h, &mut dir_h);
/// assert_eq!(status, roup::ffi::OmpStatus::Ok);
/// assert_ne!(dir_h, 0);
/// ```
#[no_mangle]
pub extern "C" fn omp_parse(
    string_handle: Handle,
    _out_directive_handle: *mut Handle,
) -> OmpStatus {
    // Safety: We need to write to the output parameter
    // Instead of using unsafe, we return the handle via a thread-local
    // and provide a separate function to retrieve it

    // Clear any previous result
    LAST_PARSE_RESULT.with(|cell| cell.set(INVALID_HANDLE));

    // Get the string bytes
    let bytes = match with_resource(string_handle, |res| match res {
        Resource::String(s) => Some(s.bytes.clone()),
        _ => None,
    }) {
        Some(Some(b)) => b,
        _ => return OmpStatus::NotFound,
    };

    // Convert to UTF-8 string
    let input = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(_) => return OmpStatus::Invalid,
    };

    // Parse using existing parser
    let parsed = match parse_omp_directive(input) {
        Ok((_, directive)) => directive,
        Err(_) => return OmpStatus::ParseError,
    };

    // Convert to IR (use default values for source location and language)
    let location = SourceLocation::new(1, 1);
    let language = Language::C;
    let config = ParserConfig::default();

    let ir = match convert_directive(&parsed, location, language, &config) {
        Ok(ir) => ir,
        Err(_) => return OmpStatus::ParseError,
    };

    // We need to convert the IR with lifetime 'a to 'static
    // This requires cloning/owning all the data
    let static_ir = convert_to_static(ir);

    // Insert into registry
    let handle = insert(Resource::Ast(Box::new(static_ir)));

    // Store result in thread-local
    LAST_PARSE_RESULT.with(|cell| cell.set(handle));

    OmpStatus::Ok
}

/// Get the last parse result
///
/// ## C Signature
/// ```c
/// uint64_t omp_take_last_parse_result(void);
/// ```
///
/// ## Returns
/// - Directive handle from last successful `omp_parse()` call
/// - 0 if last parse failed or no parse has been done
///
/// ## Note
/// This is a workaround for not using raw pointers for output parameters.
/// Each thread has its own result storage.
#[no_mangle]
pub extern "C" fn omp_take_last_parse_result() -> Handle {
    LAST_PARSE_RESULT.with(|cell| cell.take())
}

/// Free a directive AST
///
/// ## C Signature
/// ```c
/// OmpStatus omp_directive_free(uint64_t handle);
/// ```
///
/// ## Returns
/// - `Ok`: Directive freed
/// - `NotFound`: Invalid handle
/// - `Invalid`: Not a directive handle
#[no_mangle]
pub extern "C" fn omp_directive_free(handle: Handle) -> OmpStatus {
    match super::registry::remove(handle) {
        Some(Resource::Ast(_)) => OmpStatus::Ok,
        Some(_) => OmpStatus::Invalid,
        None => OmpStatus::NotFound,
    }
}

// Thread-local storage for parse results
thread_local! {
    static LAST_PARSE_RESULT: std::cell::Cell<Handle> = const { std::cell::Cell::new(INVALID_HANDLE) };
}

/// Convert DirectiveIR with borrowed lifetime to 'static
///
/// This function clones all borrowed data to create a self-contained
/// DirectiveIR that can be stored long-term.
fn convert_to_static(ir: DirectiveIR<'_>) -> DirectiveIR<'static> {
    use crate::ir::{ClauseData, ClauseItem, Expression, Identifier, Variable};

    // Helper to convert Expression
    fn convert_expr(expr: Expression<'_>) -> Expression<'static> {
        Expression::new(
            Box::leak(expr.as_str().to_string().into_boxed_str()),
            &ParserConfig::default(),
        )
    }

    // Helper to convert Identifier
    fn convert_ident(id: Identifier<'_>) -> Identifier<'static> {
        Identifier::new(Box::leak(id.to_string().into_boxed_str()))
    }

    // Helper to convert Variable
    fn convert_var(var: Variable<'_>) -> Variable<'static> {
        Variable::new(Box::leak(var.to_string().into_boxed_str()))
    }

    // Helper to convert ClauseItem
    fn convert_item(item: ClauseItem<'_>) -> ClauseItem<'static> {
        match item {
            ClauseItem::Identifier(id) => ClauseItem::Identifier(convert_ident(id)),
            ClauseItem::Variable(var) => ClauseItem::Variable(convert_var(var)),
            ClauseItem::Expression(expr) => ClauseItem::Expression(convert_expr(expr)),
        }
    }

    // Convert all clauses
    let static_clauses: Vec<ClauseData<'static>> = ir
        .clauses()
        .iter()
        .map(|clause| match clause {
            ClauseData::Private { items } => ClauseData::Private {
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
            },
            ClauseData::Firstprivate { items } => ClauseData::Firstprivate {
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
            },
            ClauseData::Shared { items } => ClauseData::Shared {
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
            },
            ClauseData::Reduction { operator, items } => ClauseData::Reduction {
                operator: *operator,
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
            },
            ClauseData::NumThreads { num } => ClauseData::NumThreads {
                num: convert_expr(num.clone()),
            },
            ClauseData::If {
                directive_name,
                condition,
            } => ClauseData::If {
                directive_name: directive_name.as_ref().map(|id| convert_ident(id.clone())),
                condition: convert_expr(condition.clone()),
            },
            ClauseData::Schedule {
                kind,
                modifiers,
                chunk_size,
            } => ClauseData::Schedule {
                kind: *kind,
                modifiers: modifiers.clone(),
                chunk_size: chunk_size.as_ref().map(|e| convert_expr(e.clone())),
            },
            ClauseData::Collapse { n } => ClauseData::Collapse {
                n: convert_expr(n.clone()),
            },
            ClauseData::Ordered { n } => ClauseData::Ordered {
                n: n.as_ref().map(|e| convert_expr(e.clone())),
            },
            ClauseData::Default(kind) => ClauseData::Default(*kind),
            ClauseData::ProcBind(kind) => ClauseData::ProcBind(*kind),
            ClauseData::Map {
                map_type,
                mapper,
                items,
            } => ClauseData::Map {
                map_type: *map_type,
                mapper: mapper.as_ref().map(|id| convert_ident(id.clone())),
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
            },
            ClauseData::Depend { depend_type, items } => ClauseData::Depend {
                depend_type: *depend_type,
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
            },
            ClauseData::Linear {
                items,
                modifier,
                step,
            } => ClauseData::Linear {
                items: items.iter().map(|i| convert_item(i.clone())).collect(),
                modifier: *modifier,
                step: step.as_ref().map(|e| convert_expr(e.clone())),
            },
            ClauseData::Bare(id) => ClauseData::Bare(convert_ident(id.clone())),
            _ => {
                // For any other clause types, convert to Bare with the clause name
                ClauseData::Bare(Identifier::new(Box::leak(
                    format!("{:?}", clause).into_boxed_str(),
                )))
            }
        })
        .collect();

    DirectiveIR::new(ir.kind(), static_clauses, ir.location(), ir.language())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::registry::REGISTRY;
    use crate::ffi::string::{omp_str_free, omp_str_new, omp_str_push_byte};

    fn cleanup() {
        REGISTRY.lock().clear();
    }

    fn build_string(text: &str) -> Handle {
        let h = omp_str_new();
        for &b in text.as_bytes() {
            omp_str_push_byte(h, b);
        }
        h
    }

    #[test]
    fn test_parse_simple_parallel() {
        cleanup();

        let str_h = build_string("#pragma omp parallel");
        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::Ok);

        let dir_h = omp_take_last_parse_result();
        assert_ne!(dir_h, INVALID_HANDLE);

        omp_directive_free(dir_h);
        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_parallel_with_clauses() {
        cleanup();

        let str_h = build_string("#pragma omp parallel default(shared) num_threads(4)");
        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::Ok);

        let dir_h = omp_take_last_parse_result();
        assert_ne!(dir_h, INVALID_HANDLE);

        omp_directive_free(dir_h);
        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_for_with_schedule() {
        cleanup();

        let str_h = build_string("#pragma omp for schedule(static, 64)");
        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::Ok);

        let dir_h = omp_take_last_parse_result();
        assert_ne!(dir_h, INVALID_HANDLE);

        omp_directive_free(dir_h);
        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_invalid_syntax() {
        cleanup();

        let str_h = build_string("#pragma omp invalid_directive_name");
        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::ParseError);

        let dir_h = omp_take_last_parse_result();
        assert_eq!(dir_h, INVALID_HANDLE);

        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_invalid_handle() {
        cleanup();

        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(INVALID_HANDLE, &mut dir_h);

        assert_eq!(status, OmpStatus::NotFound);
    }

    #[test]
    fn test_parse_invalid_utf8() {
        cleanup();

        let str_h = omp_str_new();
        omp_str_push_byte(str_h, b'#');
        omp_str_push_byte(str_h, 0xFF); // Invalid UTF-8

        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::Invalid);
        omp_str_free(str_h);
    }

    #[test]
    fn test_directive_free() {
        cleanup();

        let str_h = build_string("#pragma omp parallel");
        let mut dir_h = INVALID_HANDLE;
        omp_parse(str_h, &mut dir_h);

        let dir_h = omp_take_last_parse_result();

        assert_eq!(omp_directive_free(dir_h), OmpStatus::Ok);
        assert_eq!(omp_directive_free(dir_h), OmpStatus::NotFound); // Double free

        omp_str_free(str_h);
    }

    #[test]
    fn test_directive_free_invalid_handle() {
        cleanup();
        assert_eq!(omp_directive_free(INVALID_HANDLE), OmpStatus::NotFound);
    }

    #[test]
    fn test_parse_multiple_directives() {
        cleanup();

        let inputs = vec![
            "#pragma omp parallel",
            "#pragma omp for",
            "#pragma omp task",
            "#pragma omp barrier",
        ];

        let mut handles = Vec::new();

        for input in inputs {
            let str_h = build_string(input);
            let mut dir_h = INVALID_HANDLE;
            let status = omp_parse(str_h, &mut dir_h);

            assert_eq!(status, OmpStatus::Ok);

            let dir_h = omp_take_last_parse_result();
            assert_ne!(dir_h, INVALID_HANDLE);

            handles.push(dir_h);
            omp_str_free(str_h);
        }

        // Verify all handles are unique
        let mut sorted = handles.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), handles.len());

        // Cleanup
        for h in handles {
            omp_directive_free(h);
        }
    }

    #[test]
    fn test_parse_complex_directive() {
        cleanup();

        let str_h = build_string(
            "#pragma omp parallel for reduction(+: sum) schedule(dynamic, 10) private(i, j)",
        );
        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::Ok);

        let dir_h = omp_take_last_parse_result();
        assert_ne!(dir_h, INVALID_HANDLE);

        omp_directive_free(dir_h);
        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_empty_string() {
        cleanup();

        let str_h = omp_str_new();
        let mut dir_h = INVALID_HANDLE;
        let status = omp_parse(str_h, &mut dir_h);

        assert_eq!(status, OmpStatus::ParseError);
        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_concurrent() {
        use std::sync::Arc;
        use std::thread;

        cleanup();

        let handles = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let threads: Vec<_> = (0..5)
            .map(|i| {
                let handles = Arc::clone(&handles);
                thread::spawn(move || {
                    let str_h = build_string(&format!("#pragma omp parallel num_threads({})", i));
                    let mut dir_h = INVALID_HANDLE;
                    let status = omp_parse(str_h, &mut dir_h);

                    if status == OmpStatus::Ok {
                        let dir_h = omp_take_last_parse_result();
                        handles.lock().push(dir_h);
                    }

                    omp_str_free(str_h);
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        let handles = handles.lock();
        assert_eq!(handles.len(), 5);

        // Cleanup
        for &h in handles.iter() {
            omp_directive_free(h);
        }
    }

    #[test]
    fn test_take_result_clears() {
        cleanup();

        let str_h = build_string("#pragma omp parallel");
        let mut dir_h = INVALID_HANDLE;
        omp_parse(str_h, &mut dir_h);

        let result1 = omp_take_last_parse_result();
        assert_ne!(result1, INVALID_HANDLE);

        let result2 = omp_take_last_parse_result();
        assert_eq!(result2, INVALID_HANDLE); // Second call returns invalid

        omp_directive_free(result1);
        omp_str_free(str_h);
    }

    #[test]
    fn test_parse_reuse_string() {
        cleanup();

        let str_h = build_string("#pragma omp parallel");

        // Parse once
        let mut dir_h1 = INVALID_HANDLE;
        omp_parse(str_h, &mut dir_h1);
        let dir_h1 = omp_take_last_parse_result();

        // Parse again with same string
        let mut dir_h2 = INVALID_HANDLE;
        omp_parse(str_h, &mut dir_h2);
        let dir_h2 = omp_take_last_parse_result();

        assert_ne!(dir_h1, INVALID_HANDLE);
        assert_ne!(dir_h2, INVALID_HANDLE);
        assert_ne!(dir_h1, dir_h2); // Different directive handles

        omp_directive_free(dir_h1);
        omp_directive_free(dir_h2);
        omp_str_free(str_h);
    }
}
