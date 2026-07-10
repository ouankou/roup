//! The C memory boundary.
//!
//! This is the only module in `roup-capi` allowed to contain unsafe Rust. All
//! pointer validation and byte copying belongs here; parsing and handle storage
//! operate only on owned or safely borrowed Rust values.

use core::fmt;
use core::ptr;
use core::slice;
use core::str;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::service::{self, Failure, ServiceResult};
use crate::{
    RoupCallResult, RoupClauseKind, RoupClauseKindResult, RoupDirectiveHandle, RoupDirectiveKind,
    RoupDirectiveKindResult, RoupDirectiveResult, RoupErrorHandle, RoupFieldInfo,
    RoupFieldInfoResult, RoupNodeHandle, RoupNodeKind, RoupNodeKindResult, RoupNodeResult,
    RoupParameterKind, RoupParameterKindResult, RoupParserHandle, RoupParserOptions,
    RoupParserResult, RoupSizeResult, RoupSpan, RoupSpanResult, RoupU32Result, RoupU64Result,
    ROUP_ABI_VERSION,
};

/// A hard failure while validating a caller-provided buffer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryError {
    /// A non-empty buffer was represented by a null pointer.
    NullPointer,
    /// The requested input length cannot be represented by a Rust slice.
    LengthOverflow { length: usize },
    /// Input bytes were not valid UTF-8.
    InvalidUtf8 {
        valid_up_to: usize,
        error_len: Option<usize>,
    },
    /// No bytes were written because the destination was too small.
    BufferTooSmall { required: usize, provided: usize },
}

impl fmt::Display for BoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullPointer => formatter.write_str("non-empty buffer has a null pointer"),
            Self::LengthOverflow { length } => {
                write!(
                    formatter,
                    "buffer length {length} exceeds the addressable range"
                )
            }
            Self::InvalidUtf8 {
                valid_up_to,
                error_len,
            } => write!(
                formatter,
                "input is not UTF-8 at byte {valid_up_to} (invalid length {error_len:?})"
            ),
            Self::BufferTooSmall { required, provided } => write!(
                formatter,
                "output buffer needs {required} bytes but has {provided}"
            ),
        }
    }
}

impl std::error::Error for BoundaryError {}

/// Copy a caller-owned byte span into an owned, validated UTF-8 string.
///
/// A null pointer is accepted only when `length == 0`, representing an empty
/// string. The input is copied before this function returns, so no C pointer or
/// borrowed slice can escape into the safe parser or handle layers.
///
/// # Safety
///
/// When `length > 0`, `input` must be non-null and valid for reads of exactly
/// `length` initialized bytes for the duration of this call. The memory must
/// not be mutated concurrently. The entire region must belong to one allocated
/// object and its size must not exceed `isize::MAX`.
unsafe fn copy_utf8_input(input: *const u8, length: usize) -> Result<String, BoundaryError> {
    if length == 0 {
        return Ok(String::new());
    }
    if input.is_null() {
        return Err(BoundaryError::NullPointer);
    }
    if length > isize::MAX as usize {
        return Err(BoundaryError::LengthOverflow { length });
    }

    // SAFETY: The caller contract above guarantees a single initialized,
    // immutable allocation of `length` bytes; null and oversized spans were
    // rejected before constructing the slice.
    let bytes = unsafe { slice::from_raw_parts(input, length) };
    let text = str::from_utf8(bytes).map_err(|error| BoundaryError::InvalidUtf8 {
        valid_up_to: error.valid_up_to(),
        error_len: error.error_len(),
    })?;
    Ok(text.to_owned())
}

/// Copy UTF-8 bytes into caller-owned output storage without a NUL terminator.
///
/// The operation is all-or-nothing: an undersized destination returns
/// [`BoundaryError::BufferTooSmall`] without writing any bytes. A null pointer
/// is accepted only for an empty string.
///
/// # Safety
///
/// When `text` is non-empty, `output` must be non-null and valid for writes of
/// at least `capacity` bytes for the duration of this call. That memory must not
/// overlap `text` and must not be accessed concurrently.
unsafe fn copy_utf8_output(
    text: &str,
    output: *mut u8,
    capacity: usize,
) -> Result<usize, BoundaryError> {
    if text.is_empty() {
        return Ok(0);
    }
    if output.is_null() {
        return Err(BoundaryError::NullPointer);
    }
    if capacity < text.len() {
        return Err(BoundaryError::BufferTooSmall {
            required: text.len(),
            provided: capacity,
        });
    }

    // SAFETY: The caller contract guarantees writable storage for at least
    // `capacity` bytes and the size check establishes `text.len() <= capacity`.
    // The caller contract also guarantees that the two regions do not overlap.
    unsafe { ptr::copy_nonoverlapping(text.as_ptr(), output, text.len()) };
    Ok(text.len())
}

trait ForeignResult: Sized {
    type Value;

    fn success(value: Self::Value) -> Self;
    fn failure(failure: Failure) -> Self;
}

impl ForeignResult for RoupCallResult {
    type Value = ();

    fn success((): ()) -> Self {
        Self::success()
    }

    fn failure(failure: Failure) -> Self {
        Self::failure(failure.status, failure.error)
    }
}

macro_rules! scalar_foreign_result {
    ($result:ty, $value:ty) => {
        impl ForeignResult for $result {
            type Value = $value;

            fn success(value: Self::Value) -> Self {
                Self::success(value)
            }

            fn failure(failure: Failure) -> Self {
                Self::failure(failure.status, failure.error)
            }
        }
    };
}

scalar_foreign_result!(RoupU32Result, u32);
scalar_foreign_result!(RoupU64Result, u64);
scalar_foreign_result!(RoupSizeResult, usize);
scalar_foreign_result!(RoupSpanResult, RoupSpan);
scalar_foreign_result!(RoupClauseKindResult, RoupClauseKind);
scalar_foreign_result!(RoupDirectiveKindResult, RoupDirectiveKind);
scalar_foreign_result!(RoupParameterKindResult, RoupParameterKind);
scalar_foreign_result!(RoupNodeKindResult, RoupNodeKind);
scalar_foreign_result!(RoupFieldInfoResult, RoupFieldInfo);
scalar_foreign_result!(RoupParserResult, RoupParserHandle);
scalar_foreign_result!(RoupDirectiveResult, RoupDirectiveHandle);
scalar_foreign_result!(RoupNodeResult, RoupNodeHandle);

fn invoke<R: ForeignResult>(operation: impl FnOnce() -> ServiceResult<R::Value>) -> R {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => R::success(value),
        Ok(Err(failure)) => R::failure(failure),
        Err(_) => R::failure(service::record_internal(
            "unexpected panic crossed the safe C ABI service boundary",
        )),
    }
}

fn string_length(operation: impl FnOnce() -> ServiceResult<String>) -> RoupSizeResult {
    invoke(|| operation().map(|text| text.len()))
}

unsafe fn string_copy(
    operation: impl FnOnce() -> ServiceResult<String>,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    invoke(|| {
        let text = operation()?;
        // SAFETY: This helper has the same foreign output-buffer contract as
        // `copy_utf8_output`, and no pointer escapes this boundary module.
        unsafe { copy_utf8_output(&text, output, capacity) }.map_err(service::boundary_failure)
    })
}

/// Return the implemented ABI version.
#[no_mangle]
pub extern "C" fn roup_abi_version() -> RoupU32Result {
    RoupU32Result::success(ROUP_ABI_VERSION)
}

/// Create a strict OpenMP or OpenACC parser from validated raw options.
#[no_mangle]
pub extern "C" fn roup_parser_create(options: RoupParserOptions) -> RoupParserResult {
    invoke(|| service::create_parser(options))
}

/// Release a parser handle. Existing directive handles remain independently owned.
#[no_mangle]
pub extern "C" fn roup_parser_release(parser: RoupParserHandle) -> RoupCallResult {
    invoke(|| service::release_parser(parser))
}

/// Parse one complete UTF-8 directive and return an owned directive handle.
///
/// # Safety
///
/// `input` must satisfy [`copy_utf8_input`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_parse(
    parser: RoupParserHandle,
    input: *const u8,
    length: usize,
) -> RoupDirectiveResult {
    invoke(|| {
        // SAFETY: The exported function's contract is exactly the helper's
        // contract, and the bytes are copied before entering the safe service.
        let source =
            unsafe { copy_utf8_input(input, length) }.map_err(service::boundary_failure)?;
        service::parse(parser, source)
    })
}

/// Release an owned directive handle.
#[no_mangle]
pub extern "C" fn roup_directive_release(directive: RoupDirectiveHandle) -> RoupCallResult {
    invoke(|| service::release_directive(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_dialect(directive: RoupDirectiveHandle) -> RoupU32Result {
    invoke(|| service::directive_dialect(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_kind(directive: RoupDirectiveHandle) -> RoupDirectiveKindResult {
    invoke(|| service::directive_kind(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_span(directive: RoupDirectiveHandle) -> RoupSpanResult {
    invoke(|| service::directive_span(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_has_parameter(directive: RoupDirectiveHandle) -> RoupU32Result {
    invoke(|| service::directive_has_parameter(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_kind(
    directive: RoupDirectiveHandle,
) -> RoupParameterKindResult {
    invoke(|| service::directive_parameter_kind(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_count(
    directive: RoupDirectiveHandle,
) -> RoupSizeResult {
    invoke(|| service::directive_parameter_field_count(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_info(
    directive: RoupDirectiveHandle,
    field_index: usize,
) -> RoupFieldInfoResult {
    invoke(|| service::directive_parameter_field_info(directive, field_index))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_name_length(
    directive: RoupDirectiveHandle,
    field_index: usize,
) -> RoupSizeResult {
    string_length(|| service::directive_parameter_field_name(directive, field_index))
}

/// Copy a directive-parameter field name without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_directive_parameter_field_name_copy(
    directive: RoupDirectiveHandle,
    field_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::directive_parameter_field_name(directive, field_index),
            output,
            capacity,
        )
    }
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_u32(
    directive: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> RoupU32Result {
    invoke(|| service::directive_parameter_field_u32(directive, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_u64(
    directive: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> RoupU64Result {
    invoke(|| service::directive_parameter_field_u64(directive, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_bool(
    directive: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> RoupU32Result {
    invoke(|| service::directive_parameter_field_bool(directive, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_string_length(
    directive: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> RoupSizeResult {
    string_length(|| service::directive_parameter_field_string(directive, field_index, value_index))
}

/// Copy one typed parameter string field/list element without a trailing NUL.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_directive_parameter_field_string_copy(
    directive: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::directive_parameter_field_string(directive, field_index, value_index),
            output,
            capacity,
        )
    }
}

/// Acquire an independently owned semantic child-node handle.
#[no_mangle]
pub extern "C" fn roup_directive_parameter_field_node(
    directive: RoupDirectiveHandle,
    field_index: usize,
    value_index: usize,
) -> RoupNodeResult {
    invoke(|| service::directive_parameter_field_node(directive, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_directive_name_length(directive: RoupDirectiveHandle) -> RoupSizeResult {
    string_length(|| service::directive_name(directive))
}

/// Copy the canonical directive name without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_directive_name_copy(
    directive: RoupDirectiveHandle,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe { string_copy(|| service::directive_name(directive), output, capacity) }
}

#[no_mangle]
pub extern "C" fn roup_directive_clause_count(directive: RoupDirectiveHandle) -> RoupSizeResult {
    invoke(|| service::directive_clause_count(directive))
}

#[no_mangle]
pub extern "C" fn roup_directive_compatible_versions(
    directive: RoupDirectiveHandle,
) -> RoupU64Result {
    invoke(|| service::directive_compatible_versions(directive))
}

#[no_mangle]
pub extern "C" fn roup_clause_kind(
    directive: RoupDirectiveHandle,
    clause_index: usize,
) -> RoupClauseKindResult {
    invoke(|| service::clause_kind(directive, clause_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_span(
    directive: RoupDirectiveHandle,
    clause_index: usize,
) -> RoupSpanResult {
    invoke(|| service::clause_span(directive, clause_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_name_length(
    directive: RoupDirectiveHandle,
    clause_index: usize,
) -> RoupSizeResult {
    string_length(|| service::clause_name(directive, clause_index))
}

/// Copy a clause's canonical name without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_clause_name_copy(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::clause_name(directive, clause_index),
            output,
            capacity,
        )
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_field_count(
    directive: RoupDirectiveHandle,
    clause_index: usize,
) -> RoupSizeResult {
    invoke(|| service::clause_field_count(directive, clause_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_field_info(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
) -> RoupFieldInfoResult {
    invoke(|| service::clause_field_info(directive, clause_index, field_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_field_name_length(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
) -> RoupSizeResult {
    string_length(|| service::clause_field_name(directive, clause_index, field_index))
}

/// Copy a stable typed-field name without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_clause_field_name_copy(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::clause_field_name(directive, clause_index, field_index),
            output,
            capacity,
        )
    }
}

#[no_mangle]
pub extern "C" fn roup_clause_field_u32(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> RoupU32Result {
    invoke(|| service::clause_field_u32(directive, clause_index, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_field_u64(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> RoupU64Result {
    invoke(|| service::clause_field_u64(directive, clause_index, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_field_bool(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> RoupU32Result {
    invoke(|| service::clause_field_bool(directive, clause_index, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_clause_field_string_length(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> RoupSizeResult {
    string_length(|| {
        service::clause_field_string(directive, clause_index, field_index, value_index)
    })
}

/// Copy one typed string field or string-list element without a trailing NUL.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_clause_field_string_copy(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::clause_field_string(directive, clause_index, field_index, value_index),
            output,
            capacity,
        )
    }
}

/// Acquire an independently owned semantic child-node handle.
#[no_mangle]
pub extern "C" fn roup_clause_field_node(
    directive: RoupDirectiveHandle,
    clause_index: usize,
    field_index: usize,
    value_index: usize,
) -> RoupNodeResult {
    invoke(|| service::clause_field_node(directive, clause_index, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_node_kind(node: RoupNodeHandle) -> RoupNodeKindResult {
    invoke(|| service::node_kind(node))
}

#[no_mangle]
pub extern "C" fn roup_node_field_count(node: RoupNodeHandle) -> RoupSizeResult {
    invoke(|| service::node_field_count(node))
}

#[no_mangle]
pub extern "C" fn roup_node_field_info(
    node: RoupNodeHandle,
    field_index: usize,
) -> RoupFieldInfoResult {
    invoke(|| service::node_field_info(node, field_index))
}

#[no_mangle]
pub extern "C" fn roup_node_field_name_length(
    node: RoupNodeHandle,
    field_index: usize,
) -> RoupSizeResult {
    string_length(|| service::node_field_name(node, field_index))
}

/// Copy a semantic child-node field name without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_node_field_name_copy(
    node: RoupNodeHandle,
    field_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::node_field_name(node, field_index),
            output,
            capacity,
        )
    }
}

#[no_mangle]
pub extern "C" fn roup_node_field_u32(
    node: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> RoupU32Result {
    invoke(|| service::node_field_u32(node, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_node_field_u64(
    node: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> RoupU64Result {
    invoke(|| service::node_field_u64(node, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_node_field_bool(
    node: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> RoupU32Result {
    invoke(|| service::node_field_bool(node, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_node_field_string_length(
    node: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> RoupSizeResult {
    string_length(|| service::node_field_string(node, field_index, value_index))
}

/// Copy one semantic child-node string leaf without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_node_field_string_copy(
    node: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe {
        string_copy(
            || service::node_field_string(node, field_index, value_index),
            output,
            capacity,
        )
    }
}

/// Acquire an independently owned nested semantic child-node handle.
#[no_mangle]
pub extern "C" fn roup_node_field_node(
    node: RoupNodeHandle,
    field_index: usize,
    value_index: usize,
) -> RoupNodeResult {
    invoke(|| service::node_field_node(node, field_index, value_index))
}

#[no_mangle]
pub extern "C" fn roup_node_release(node: RoupNodeHandle) -> RoupCallResult {
    invoke(|| service::release_node(node))
}

#[no_mangle]
pub extern "C" fn roup_error_code(error: RoupErrorHandle) -> RoupU32Result {
    invoke(|| service::error_code(error))
}

#[no_mangle]
pub extern "C" fn roup_error_span(error: RoupErrorHandle) -> RoupSpanResult {
    invoke(|| service::error_span(error))
}

#[no_mangle]
pub extern "C" fn roup_error_message_length(error: RoupErrorHandle) -> RoupSizeResult {
    string_length(|| service::error_message(error))
}

/// Copy an error message without a trailing NUL byte.
///
/// # Safety
///
/// `output` must satisfy [`copy_utf8_output`]'s safety contract.
#[no_mangle]
pub unsafe extern "C" fn roup_error_message_copy(
    error: RoupErrorHandle,
    output: *mut u8,
    capacity: usize,
) -> RoupSizeResult {
    // SAFETY: The exported function forwards its documented buffer contract.
    unsafe { string_copy(|| service::error_message(error), output, capacity) }
}

/// Release an error handle after all diagnostic fields have been copied.
#[no_mangle]
pub extern "C" fn roup_error_release(error: RoupErrorHandle) -> RoupCallResult {
    invoke(|| service::release_error(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RoupStatus, ROUP_DIALECT_OPENMP, ROUP_HOST_C, ROUP_SOURCE_PRAGMA, ROUP_VERSION_ANY,
    };

    fn openmp_options() -> RoupParserOptions {
        RoupParserOptions {
            abi_version: ROUP_ABI_VERSION,
            struct_size: core::mem::size_of::<RoupParserOptions>() as u32,
            dialect: ROUP_DIALECT_OPENMP,
            version_policy: ROUP_VERSION_ANY,
            version: 0,
            host_language: ROUP_HOST_C,
            host_standard: 23,
            source_form: ROUP_SOURCE_PRAGMA,
            flags: 0,
            reserved: [0; 3],
        }
    }

    fn exact_openmp_options(
        version: u32,
        host_language: u32,
        host_standard: u32,
        source_form: u32,
    ) -> RoupParserOptions {
        let mut options = openmp_options();
        options.version_policy = crate::ROUP_VERSION_EXACT;
        options.version = version;
        options.host_language = host_language;
        options.host_standard = host_standard;
        options.source_form = source_form;
        options
    }

    fn openacc_options() -> RoupParserOptions {
        let mut options = openmp_options();
        options.dialect = crate::ROUP_DIALECT_OPENACC;
        options
    }

    fn openacc_fortran_options() -> RoupParserOptions {
        let mut options = openacc_options();
        options.host_language = crate::ROUP_HOST_FORTRAN;
        options.host_standard = 2023;
        options.source_form = crate::ROUP_SOURCE_FORTRAN_FREE;
        options
    }

    unsafe fn copied_string(
        length: RoupSizeResult,
        copy: impl FnOnce(*mut u8, usize) -> RoupSizeResult,
    ) -> String {
        assert!(length.result.status.is_ok());
        let mut bytes = vec![0_u8; length.value];
        let copied = copy(bytes.as_mut_ptr(), bytes.len());
        assert!(copied.result.status.is_ok());
        assert_eq!(copied.value, bytes.len());
        String::from_utf8(bytes).unwrap()
    }

    fn parse_success(parser: RoupParserHandle, source: &str) -> RoupDirectiveHandle {
        // SAFETY: `source` remains live and immutable for the duration of this
        // call, and the boundary copies exactly `source.len()` bytes.
        let directive = unsafe { roup_parse(parser, source.as_ptr(), source.len()) };
        if !directive.result.status.is_ok() {
            // SAFETY: the queried length sizes the owned destination exactly.
            let message = unsafe {
                copied_string(
                    roup_error_message_length(directive.result.error),
                    |out, len| roup_error_message_copy(directive.result.error, out, len),
                )
            };
            assert!(roup_error_release(directive.result.error).status.is_ok());
            panic!("parse failed for {source:?}: {message}");
        }
        directive.value
    }

    fn parameter_string(
        directive: RoupDirectiveHandle,
        field_index: usize,
        value_index: usize,
    ) -> String {
        // SAFETY: the queried length sizes the owned destination exactly.
        unsafe {
            copied_string(
                roup_directive_parameter_field_string_length(directive, field_index, value_index),
                |out, len| {
                    roup_directive_parameter_field_string_copy(
                        directive,
                        field_index,
                        value_index,
                        out,
                        len,
                    )
                },
            )
        }
    }

    fn clause_string(
        directive: RoupDirectiveHandle,
        clause_index: usize,
        field_index: usize,
        value_index: usize,
    ) -> String {
        // SAFETY: the queried length sizes the owned destination exactly.
        unsafe {
            copied_string(
                roup_clause_field_string_length(directive, clause_index, field_index, value_index),
                |out, len| {
                    roup_clause_field_string_copy(
                        directive,
                        clause_index,
                        field_index,
                        value_index,
                        out,
                        len,
                    )
                },
            )
        }
    }

    fn node_string(node: RoupNodeHandle, field_index: usize, value_index: usize) -> String {
        // SAFETY: the queried length sizes the owned destination exactly.
        unsafe {
            copied_string(
                roup_node_field_string_length(node, field_index, value_index),
                |out, len| roup_node_field_string_copy(node, field_index, value_index, out, len),
            )
        }
    }

    #[test]
    fn input_is_copied_and_validated() {
        let source = "parallel λ";
        let copied = unsafe { copy_utf8_input(source.as_ptr(), source.len()) }.unwrap();
        assert_eq!(copied, source);
    }

    #[test]
    fn u32_scalar_and_list_fields_cross_the_boundary_with_hard_type_errors() {
        let node = service::store_test_u32_node().expect("synthetic typed node");

        let scalar_info = roup_node_field_info(node, 0);
        assert!(scalar_info.result.status.is_ok());
        assert_eq!(scalar_info.value.value_kind, crate::ROUP_FIELD_VALUE_U32);
        assert_eq!(scalar_info.value.count, 1);
        assert_eq!(roup_node_field_u32(node, 0, 0).value, 17);

        let list_info = roup_node_field_info(node, 1);
        assert!(list_info.result.status.is_ok());
        assert_eq!(list_info.value.value_kind, crate::ROUP_FIELD_VALUE_U32_LIST);
        assert_eq!(list_info.value.count, 3);
        assert_eq!(roup_node_field_u32(node, 1, 0).value, 3);
        assert_eq!(roup_node_field_u32(node, 1, 1).value, 5);
        assert_eq!(roup_node_field_u32(node, 1, 2).value, 8);

        let out_of_range = roup_node_field_u32(node, 1, 3);
        assert_eq!(out_of_range.result.status, RoupStatus::INVALID_ARGUMENT);
        assert!(out_of_range.result.error.is_active());
        assert!(roup_error_release(out_of_range.result.error).status.is_ok());

        let wrong_type = roup_node_field_bool(node, 0, 0);
        assert_eq!(wrong_type.result.status, RoupStatus::INVALID_ARGUMENT);
        assert!(wrong_type.result.error.is_active());
        assert!(roup_error_release(wrong_type.result.error).status.is_ok());

        assert!(roup_node_release(node).status.is_ok());
    }

    #[test]
    fn u64_position_fields_cross_the_boundary_without_truncation() {
        let parser = roup_parser_create(exact_openmp_options(
            crate::ROUP_OMP_VERSION_6_0,
            crate::ROUP_HOST_C,
            crate::ROUP_C_23,
            crate::ROUP_SOURCE_PRAGMA,
        ));
        assert!(parser.result.status.is_ok());
        let directive = parse_success(
            parser.value,
            "#pragma omp declare variant(fast) match(construct={parallel}) adjust_args(need_device_ptr: 4294967297)",
        );

        let clause_kind = roup_clause_kind(directive, 1);
        assert!(clause_kind.result.status.is_ok());
        assert_eq!(
            clause_kind.value.ordinal,
            crate::ROUP_OMP_CLAUSE_ADJUST_ARGS
        );

        let field_count = roup_clause_field_count(directive, 1);
        assert!(field_count.result.status.is_ok());
        let parameter_field = (0..field_count.value)
            .find(|field| {
                let info = roup_clause_field_info(directive, 1, *field);
                assert!(info.result.status.is_ok());
                info.value.id == crate::ROUP_FIELD_PARAMETERS
            })
            .expect("adjust_args parameters field");
        let parameter = roup_clause_field_node(directive, 1, parameter_field, 0);
        assert!(parameter.result.status.is_ok());
        assert_eq!(
            roup_node_kind(parameter.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_PARAMETER_LIST_ITEM,
                variant: crate::ROUP_OMP_PARAMETER_POSITION,
            }
        );
        let value_info = roup_node_field_info(parameter.value, 0);
        assert!(value_info.result.status.is_ok());
        assert_eq!(value_info.value.id, crate::ROUP_FIELD_VALUE);
        assert_eq!(value_info.value.value_kind, crate::ROUP_FIELD_VALUE_U64);
        assert_eq!(
            roup_node_field_u64(parameter.value, 0, 0).value,
            4_294_967_297
        );

        let wrong_width = roup_node_field_u32(parameter.value, 0, 0);
        assert_eq!(wrong_width.result.status, RoupStatus::INVALID_ARGUMENT);
        assert!(roup_error_release(wrong_width.result.error).status.is_ok());
        let out_of_range = roup_node_field_u64(parameter.value, 0, 1);
        assert_eq!(out_of_range.result.status, RoupStatus::INVALID_ARGUMENT);
        assert!(roup_error_release(out_of_range.result.error).status.is_ok());

        assert!(roup_node_release(parameter.value).status.is_ok());
        assert!(roup_directive_release(directive).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn empty_input_may_use_a_null_pointer() {
        assert_eq!(
            unsafe { copy_utf8_input(core::ptr::null(), 0) },
            Ok(String::new())
        );
    }

    #[test]
    fn nonempty_null_input_is_rejected() {
        assert_eq!(
            unsafe { copy_utf8_input(core::ptr::null(), 1) },
            Err(BoundaryError::NullPointer)
        );
    }

    #[test]
    fn invalid_utf8_reports_the_exact_byte() {
        let source = [b'a', 0xff, b'b'];
        assert_eq!(
            unsafe { copy_utf8_input(source.as_ptr(), source.len()) },
            Err(BoundaryError::InvalidUtf8 {
                valid_up_to: 1,
                error_len: Some(1),
            })
        );
    }

    #[test]
    fn output_is_all_or_nothing() {
        let mut destination = [0xaa; 3];
        assert_eq!(
            unsafe { copy_utf8_output("four", destination.as_mut_ptr(), destination.len()) },
            Err(BoundaryError::BufferTooSmall {
                required: 4,
                provided: 3,
            })
        );
        assert_eq!(destination, [0xaa; 3]);
    }

    #[test]
    fn output_copies_utf8_without_a_terminator() {
        let source = "λ";
        let mut destination = [0_u8; 4];
        let written =
            unsafe { copy_utf8_output(source, destination.as_mut_ptr(), destination.len()) }
                .unwrap();

        assert_eq!(written, source.len());
        assert_eq!(&destination[..written], source.as_bytes());
        assert_eq!(destination[written], 0);
    }

    #[test]
    fn opaque_abi_parses_and_queries_a_typed_directive() {
        let parser = roup_parser_create(openmp_options());
        assert!(parser.result.status.is_ok());
        let source = "#pragma omp parallel num_threads(4) private(x)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        assert_eq!(
            roup_directive_dialect(directive.value).value,
            ROUP_DIALECT_OPENMP
        );
        assert_eq!(roup_directive_clause_count(directive.value).value, 2);
        assert_ne!(roup_directive_compatible_versions(directive.value).value, 0);
        let name = unsafe {
            copied_string(roup_directive_name_length(directive.value), |out, len| {
                roup_directive_name_copy(directive.value, out, len)
            })
        };
        assert_eq!(name, "parallel");
        let directive_span = roup_directive_span(directive.value);
        assert!(directive_span.result.status.is_ok());
        assert_eq!(
            directive_span.value.start_byte,
            source.find("parallel").unwrap()
        );
        assert_eq!(
            directive_span.value.end_byte,
            source.find("parallel").unwrap() + 8
        );
        assert_eq!(directive_span.value.start_line, 1);
        assert_eq!(directive_span.value.start_column, 13);

        let clause_name = unsafe {
            copied_string(roup_clause_name_length(directive.value, 0), |out, len| {
                roup_clause_name_copy(directive.value, 0, out, len)
            })
        };
        assert_eq!(clause_name, "num_threads");
        let clause_span = roup_clause_span(directive.value, 0);
        assert!(clause_span.result.status.is_ok());
        assert_eq!(
            clause_span.value.start_byte,
            source.find("num_threads").unwrap()
        );
        assert_eq!(
            clause_span.value.end_byte,
            source.find("num_threads").unwrap() + 11
        );
        assert_eq!(clause_span.value.start_line, 1);
        assert_eq!(clause_span.value.start_column, 22);
        assert_eq!(
            roup_clause_kind(directive.value, 0).value.dialect,
            ROUP_DIALECT_OPENMP
        );
        assert_eq!(roup_clause_field_count(directive.value, 0).value, 2);
        let modifiers = roup_clause_field_info(directive.value, 0, 0);
        assert_eq!(modifiers.value.id, crate::ROUP_FIELD_MODIFIERS);
        assert_eq!(modifiers.value.count, 0);
        let field = roup_clause_field_info(directive.value, 0, 1);
        assert_eq!(field.value.id, crate::ROUP_FIELD_VALUES);
        assert_eq!(field.value.value_kind, crate::ROUP_FIELD_VALUE_STRING_LIST);
        let typed_value = unsafe {
            copied_string(
                roup_clause_field_string_length(directive.value, 0, 1, 0),
                |out, len| roup_clause_field_string_copy(directive.value, 0, 1, 0, out, len),
            )
        };
        assert_eq!(typed_value, "4");

        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn fortran_common_block_clause_items_cross_the_c_boundary_intact() {
        let parser = roup_parser_create(exact_openmp_options(
            crate::ROUP_OMP_VERSION_5_2,
            crate::ROUP_HOST_FORTRAN,
            crate::ROUP_FORTRAN_2023,
            crate::ROUP_SOURCE_FORTRAN_FREE,
        ));
        assert!(parser.result.status.is_ok());

        let directive = parse_success(
            parser.value,
            "!$omp parallel private(/WORK/) copyin(/STATE/)",
        );
        for (clause_index, expected) in [(0, "work"), (1, "state")] {
            let field = roup_clause_field_info(directive, clause_index, 0);
            assert!(field.result.status.is_ok());
            assert_eq!(field.value.id, crate::ROUP_FIELD_ITEMS);
            assert_eq!(field.value.value_kind, crate::ROUP_FIELD_VALUE_NODE_LIST);
            assert_eq!(field.value.count, 1);
            let item = roup_clause_field_node(directive, clause_index, 0, 0);
            assert!(item.result.status.is_ok());
            assert_eq!(
                roup_node_kind(item.value).value,
                RoupNodeKind {
                    family: crate::ROUP_NODE_FAMILY_CLAUSE_ITEM,
                    variant: crate::ROUP_CLAUSE_ITEM_FORTRAN_COMMON_BLOCK,
                }
            );
            let name = roup_node_field_info(item.value, 0);
            assert!(name.result.status.is_ok());
            assert_eq!(name.value.id, crate::ROUP_FIELD_NAME);
            assert_eq!(node_string(item.value, 0, 0), expected);
            assert!(roup_node_release(item.value).status.is_ok());
        }

        assert!(roup_directive_release(directive).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn every_clause_item_variant_has_a_distinct_c_node_tag_and_field_shape() {
        let parser = roup_parser_create(openmp_options());
        assert!(parser.result.status.is_ok());

        let private = parse_success(
            parser.value,
            "#pragma omp parallel private(value, array[0:length])",
        );
        let private_items = roup_clause_field_info(private, 0, 0);
        assert_eq!(
            private_items.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE_LIST
        );
        assert_eq!(private_items.value.count, 2);
        for (index, variant, field_id, expected) in [
            (
                0,
                crate::ROUP_CLAUSE_ITEM_IDENTIFIER,
                crate::ROUP_FIELD_NAME,
                "value",
            ),
            (
                1,
                crate::ROUP_CLAUSE_ITEM_VARIABLE,
                crate::ROUP_FIELD_VARIABLE,
                "array[0:length]",
            ),
        ] {
            let item = roup_clause_field_node(private, 0, 0, index);
            assert!(item.result.status.is_ok());
            assert_eq!(
                roup_node_kind(item.value).value,
                RoupNodeKind {
                    family: crate::ROUP_NODE_FAMILY_CLAUSE_ITEM,
                    variant,
                }
            );
            assert_eq!(roup_node_field_info(item.value, 0).value.id, field_id);
            assert_eq!(node_string(item.value, 0, 0), expected);
            assert!(roup_node_release(item.value).status.is_ok());
        }
        assert!(roup_directive_release(private).status.is_ok());

        let doacross = parse_success(parser.value, "#pragma omp ordered depend(sink: i - 1)");
        let iteration = roup_clause_field_node(doacross, 0, 1, 0);
        assert!(iteration.result.status.is_ok());
        assert_eq!(
            roup_node_kind(iteration.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_DOACROSS_ITERATION,
                variant: crate::ROUP_OMP_DOACROSS_VECTOR,
            }
        );
        let vector_item = roup_node_field_node(iteration.value, 0, 0);
        assert!(vector_item.result.status.is_ok());
        assert_eq!(
            roup_node_kind(vector_item.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_DOACROSS_VECTOR_ITEM,
                variant: crate::ROUP_OMP_DOACROSS_VECTOR_ITEM,
            }
        );
        assert_eq!(node_string(vector_item.value, 0, 0), "i");
        assert_eq!(
            roup_node_field_u32(vector_item.value, 1, 0).value,
            crate::ROUP_OMP_DOACROSS_OFFSET_SUBTRACT
        );
        assert_eq!(node_string(vector_item.value, 2, 0), "1");
        assert!(roup_node_release(vector_item.value).status.is_ok());
        assert!(roup_node_release(iteration.value).status.is_ok());
        assert!(roup_directive_release(doacross).status.is_ok());

        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn optional_openmp_semantics_are_visible_as_named_c_fields() {
        let parser = roup_parser_create(openmp_options());
        assert!(parser.result.status.is_ok());

        let source = "#pragma omp requires reverse_offload(required_flag)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(roup_clause_field_count(directive.value, 0).value, 2);
        let required = roup_clause_field_info(directive.value, 0, 1);
        assert!(required.result.status.is_ok());
        assert_eq!(required.value.id, crate::ROUP_FIELD_REQUIRED);
        assert_eq!(required.value.value_kind, crate::ROUP_FIELD_VALUE_STRING);
        let required_value = unsafe {
            copied_string(
                roup_clause_field_string_length(directive.value, 0, 1, 0),
                |out, len| roup_clause_field_string_copy(directive.value, 0, 1, 0, out, len),
            )
        };
        assert_eq!(required_value, "required_flag");
        assert!(roup_directive_release(directive.value).status.is_ok());

        let source = "#pragma omp atomic read(use_it)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(roup_clause_field_count(directive.value, 0).value, 2);
        let semantics = roup_clause_field_info(directive.value, 0, 1);
        assert!(semantics.result.status.is_ok());
        assert_eq!(semantics.value.id, crate::ROUP_FIELD_USE_SEMANTICS);
        let semantics_value = unsafe {
            copied_string(
                roup_clause_field_string_length(directive.value, 0, 1, 0),
                |out, len| roup_clause_field_string_copy(directive.value, 0, 1, 0, out, len),
            )
        };
        assert_eq!(semantics_value, "use_it");
        assert!(roup_directive_release(directive.value).status.is_ok());

        let source = "#pragma omp for nowait";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(roup_clause_field_count(directive.value, 0).value, 0);
        assert!(roup_directive_release(directive.value).status.is_ok());

        let source = "#pragma omp for nowait(skip_barrier)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(roup_clause_field_count(directive.value, 0).value, 1);
        let nowait = roup_clause_field_info(directive.value, 0, 0);
        assert!(nowait.result.status.is_ok());
        assert_eq!(nowait.value.id, crate::ROUP_FIELD_DO_NOT_SYNCHRONIZE);
        assert!(roup_directive_release(directive.value).status.is_ok());

        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openmp6_modifiers_shapes_and_mapper_ids_cross_the_c_abi_typed() {
        fn field_index(directive: RoupDirectiveHandle, clause: usize, expected_id: u32) -> usize {
            let count = roup_clause_field_count(directive, clause);
            assert!(count.result.status.is_ok());
            (0..count.value)
                .find(|index| {
                    let info = roup_clause_field_info(directive, clause, *index);
                    assert!(info.result.status.is_ok());
                    info.value.id == expected_id
                })
                .expect("required C ABI field must be present")
        }

        let parser = roup_parser_create(openmp_options());
        assert!(parser.result.status.is_ok());

        let source = "#pragma omp task threadset(task: omp_pool)";
        let threadset = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(threadset.result.status.is_ok());
        let modifier = field_index(
            threadset.value,
            0,
            crate::ROUP_FIELD_DIRECTIVE_NAME_MODIFIER,
        );
        let kind = field_index(threadset.value, 0, crate::ROUP_FIELD_KIND);
        assert_eq!(
            roup_clause_field_u32(threadset.value, 0, modifier, 0).value,
            crate::ROUP_OMP_DIRECTIVE_TASK
        );
        assert_eq!(
            roup_clause_field_u32(threadset.value, 0, kind, 0).value,
            crate::ROUP_OMP_THREADSET_OMP_POOL
        );
        let wrong_type = roup_clause_field_string_length(threadset.value, 0, modifier, 0);
        assert!(!wrong_type.result.status.is_ok());
        assert!(roup_error_release(wrong_type.result.error).status.is_ok());
        assert!(roup_directive_release(threadset.value).status.is_ok());

        let source = "#pragma omp fuse looprange(2, number_of_loops)";
        let looprange = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(looprange.result.status.is_ok());
        let first = field_index(looprange.value, 0, crate::ROUP_FIELD_FIRST);
        let count = field_index(looprange.value, 0, crate::ROUP_FIELD_COUNT);
        assert_eq!(clause_string(looprange.value, 0, first, 0), "2");
        assert_eq!(
            clause_string(looprange.value, 0, count, 0),
            "number_of_loops"
        );
        assert!(roup_directive_release(looprange.value).status.is_ok());

        let source = "#pragma omp target map(mapper(default), to: x)";
        let mapped = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(mapped.result.status.is_ok());
        let mapper = field_index(mapped.value, 0, crate::ROUP_FIELD_MAPPER);
        let mapper_info = roup_clause_field_info(mapped.value, 0, mapper);
        assert_eq!(mapper_info.value.value_kind, crate::ROUP_FIELD_VALUE_NODE);
        let mapper_node = roup_clause_field_node(mapped.value, 0, mapper, 0);
        assert!(mapper_node.result.status.is_ok());
        assert_eq!(
            roup_node_kind(mapper_node.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_MAPPER_ID,
                variant: crate::ROUP_OMP_MAPPER_ID_DEFAULT,
            }
        );
        assert_eq!(roup_node_field_count(mapper_node.value).value, 0);
        assert!(roup_node_release(mapper_node.value).status.is_ok());
        assert!(roup_directive_release(mapped.value).status.is_ok());

        let source = "#pragma omp taskgraph graph_reset";
        let reset = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(reset.result.status.is_ok());
        assert_eq!(roup_clause_field_count(reset.value, 0).value, 0);
        assert!(roup_directive_release(reset.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn parse_error_exposes_code_span_and_message() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp definitely_not_a_directive";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert_eq!(directive.result.status, RoupStatus::PARSE_ERROR);
        assert!(directive.result.error.is_active());

        let code = roup_error_code(directive.result.error);
        let span = roup_error_span(directive.result.error);
        assert!(code.result.status.is_ok());
        assert_ne!(code.value, 0);
        assert_eq!(span.value.start_byte, 0);
        assert_eq!(span.value.end_byte, source.len());
        let message = unsafe {
            copied_string(
                roup_error_message_length(directive.result.error),
                |out, len| roup_error_message_copy(directive.result.error, out, len),
            )
        };
        assert!(!message.is_empty());

        assert!(roup_error_release(directive.result.error).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn undersized_output_is_untouched_and_returns_an_error_handle() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp parallel";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        let mut output = [0xaa_u8; 3];
        let result =
            unsafe { roup_directive_name_copy(directive.value, output.as_mut_ptr(), output.len()) };

        assert_eq!(result.result.status, RoupStatus::BUFFER_TOO_SMALL);
        assert!(result.result.error.is_active());
        assert_eq!(output, [0xaa; 3]);
        assert_eq!(
            roup_error_code(result.result.error).value,
            crate::ROUP_DIAGNOSTIC_BUFFER_TOO_SMALL
        );

        assert!(roup_error_release(result.result.error).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn directive_parameter_variant_and_fields_are_typed() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp critical(lock_name)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        assert_eq!(roup_directive_has_parameter(directive.value).value, 1);
        let kind = roup_directive_parameter_kind(directive.value);
        assert_eq!(kind.value.dialect, ROUP_DIALECT_OPENMP);
        assert_eq!(
            kind.value.variant,
            crate::ROUP_OMP_PARAMETER_CRITICAL_SECTION
        );
        let field = roup_directive_parameter_field_info(directive.value, 0);
        assert_eq!(field.value.id, crate::ROUP_FIELD_VALUE);
        let value = unsafe {
            copied_string(
                roup_directive_parameter_field_string_length(directive.value, 0, 0),
                |out, len| {
                    roup_directive_parameter_field_string_copy(directive.value, 0, 0, out, len)
                },
            )
        };
        assert_eq!(value, "lock_name");

        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openmp_parameter_tags_preserve_directive_grammar_and_variant_fields() {
        let parser = roup_parser_create(openmp_options());
        for (source, expected_variant, item_family, item_variant) in [
            (
                "#pragma omp allocate(value)",
                crate::ROUP_OMP_PARAMETER_ALLOCATE_LIST,
                crate::ROUP_NODE_FAMILY_OMP_STORAGE_ITEM,
                crate::ROUP_OMP_STORAGE_ITEM_NAME,
            ),
            (
                "#pragma omp threadprivate(value)",
                crate::ROUP_OMP_PARAMETER_THREADPRIVATE_LIST,
                crate::ROUP_NODE_FAMILY_OMP_STORAGE_ITEM,
                crate::ROUP_OMP_STORAGE_ITEM_NAME,
            ),
            (
                "#pragma omp groupprivate(value)",
                crate::ROUP_OMP_PARAMETER_GROUPPRIVATE_LIST,
                crate::ROUP_NODE_FAMILY_OMP_STORAGE_ITEM,
                crate::ROUP_OMP_STORAGE_ITEM_NAME,
            ),
            (
                "#pragma omp declare target(value)",
                crate::ROUP_OMP_PARAMETER_DECLARE_TARGET_LIST,
                crate::ROUP_NODE_FAMILY_OMP_DECLARE_TARGET_ITEM,
                crate::ROUP_OMP_DECLARE_TARGET_ITEM_NAME,
            ),
        ] {
            let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
            assert!(
                directive.result.status.is_ok(),
                "failed to parse {source:?}"
            );
            assert_eq!(
                roup_directive_parameter_kind(directive.value).value.variant,
                expected_variant
            );
            let field = roup_directive_parameter_field_info(directive.value, 0);
            assert_eq!(field.value.id, crate::ROUP_FIELD_ITEMS);
            assert_eq!(field.value.value_kind, crate::ROUP_FIELD_VALUE_NODE_LIST);
            assert_eq!(field.value.count, 1);
            let wrong = roup_directive_parameter_field_string_length(directive.value, 0, 0);
            assert!(!wrong.result.status.is_ok());
            assert!(roup_error_release(wrong.result.error).status.is_ok());
            let item = roup_directive_parameter_field_node(directive.value, 0, 0);
            assert!(item.result.status.is_ok());
            assert_eq!(
                roup_node_kind(item.value).value,
                RoupNodeKind {
                    family: item_family,
                    variant: item_variant,
                }
            );
            assert_eq!(node_string(item.value, 0, 0), "value");
            assert!(roup_node_release(item.value).status.is_ok());
            assert!(roup_directive_release(directive.value).status.is_ok());
        }

        let source = "#pragma omp declare variant(base:fast) match(construct={parallel})";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(
            roup_directive_parameter_kind(directive.value).value.variant,
            crate::ROUP_OMP_PARAMETER_DECLARE_VARIANT
        );
        assert_eq!(
            roup_directive_parameter_field_count(directive.value).value,
            2
        );
        assert_eq!(
            roup_directive_parameter_field_info(directive.value, 0)
                .value
                .id,
            crate::ROUP_FIELD_BASE
        );
        assert_eq!(parameter_string(directive.value, 0, 0), "base");
        let function_info = roup_directive_parameter_field_info(directive.value, 1);
        assert_eq!(function_info.value.id, crate::ROUP_FIELD_FUNCTION);
        assert_eq!(function_info.value.value_kind, crate::ROUP_FIELD_VALUE_NODE);
        let function = roup_directive_parameter_field_node(directive.value, 1, 0);
        assert_eq!(
            roup_node_kind(function.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
                variant: crate::ROUP_OMP_ID_EXPRESSION_NAME,
            }
        );
        assert_eq!(node_string(function.value, 0, 0), "fast");
        assert!(roup_node_release(function.value).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_cache_items_remain_typed_across_the_c_abi() {
        let parser = roup_parser_create(openacc_options());
        let source = "#pragma acc cache(readonly: values[index], tile[0:n])";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        let parameter_kind = roup_directive_parameter_kind(directive.value);
        assert_eq!(parameter_kind.value.dialect, crate::ROUP_DIALECT_OPENACC);
        assert_eq!(
            parameter_kind.value.variant,
            crate::ROUP_ACC_PARAMETER_CACHE
        );
        assert_eq!(
            roup_directive_parameter_field_bool(directive.value, 0, 0).value,
            1
        );
        let items = roup_directive_parameter_field_info(directive.value, 1);
        assert_eq!(items.value.id, crate::ROUP_FIELD_ITEMS);
        assert_eq!(items.value.value_kind, crate::ROUP_FIELD_VALUE_NODE_LIST);
        assert_eq!(items.value.count, 2);

        let element = roup_directive_parameter_field_node(directive.value, 1, 0);
        let subarray = roup_directive_parameter_field_node(directive.value, 1, 1);
        assert_eq!(
            roup_node_kind(element.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_ACC_CACHE_ITEM,
                variant: crate::ROUP_ACC_CACHE_ARRAY_ELEMENT,
            }
        );
        assert_eq!(
            roup_node_kind(subarray.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_ACC_CACHE_ITEM,
                variant: crate::ROUP_ACC_CACHE_CONTIGUOUS_SUBARRAY,
            }
        );
        let element_text = unsafe {
            copied_string(
                roup_node_field_string_length(element.value, 0, 0),
                |out, len| roup_node_field_string_copy(element.value, 0, 0, out, len),
            )
        };
        let subarray_text = unsafe {
            copied_string(
                roup_node_field_string_length(subarray.value, 0, 0),
                |out, len| roup_node_field_string_copy(subarray.value, 0, 0, out, len),
            )
        };
        assert_eq!(element_text, "values[index]");
        assert_eq!(subarray_text, "tile[0:n]");

        assert!(roup_node_release(element.value).status.is_ok());
        assert!(roup_node_release(subarray.value).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());

        for invalid in [
            "#pragma acc cache(scalar)",
            "#pragma acc cache(values[0:n:2])",
        ] {
            let result = unsafe { roup_parse(parser.value, invalid.as_ptr(), invalid.len()) };
            assert!(!result.result.status.is_ok(), "accepted {invalid:?}");
            assert!(roup_error_release(result.result.error).status.is_ok());
        }
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_end_kind_is_a_closed_c_abi_node() {
        let parser = roup_parser_create(openacc_fortran_options());
        let source = "!$acc end PARALLEL LOOP";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(
            roup_directive_parameter_kind(directive.value).value.variant,
            crate::ROUP_ACC_PARAMETER_END
        );
        let field = roup_directive_parameter_field_info(directive.value, 0);
        assert_eq!(field.value.id, crate::ROUP_FIELD_KIND);
        assert_eq!(field.value.value_kind, crate::ROUP_FIELD_VALUE_NODE);
        let kind = roup_directive_parameter_field_node(directive.value, 0, 0);
        assert_eq!(
            roup_node_kind(kind.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_ACC_END_KIND,
                variant: crate::ROUP_ACC_END_PARALLEL_LOOP,
            }
        );

        assert!(roup_node_release(kind.value).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn declare_reduction_semantics_cross_the_c_abi_without_raw_payloads() {
        struct Case {
            options: RoupParserOptions,
            source: &'static str,
            type_name: &'static str,
        }

        let cases = [
            Case {
                options: exact_openmp_options(
                    crate::ROUP_OMP_VERSION_5_2,
                    crate::ROUP_HOST_C,
                    crate::ROUP_C_23,
                    crate::ROUP_SOURCE_PRAGMA,
                ),
                source: "#pragma omp declare reduction(sum : int : omp_out += omp_in) initializer(omp_priv = 0)",
                type_name: "int",
            },
            Case {
                options: exact_openmp_options(
                    crate::ROUP_OMP_VERSION_6_0,
                    crate::ROUP_HOST_CPP,
                    crate::ROUP_CPP_23,
                    crate::ROUP_SOURCE_PRAGMA,
                ),
                source: "#pragma omp declare_reduction(ns::merge<int> : std::vector<int>) combiner(omp_out += omp_in) initializer(omp_priv(omp_orig))",
                type_name: "std :: vector < int >",
            },
            Case {
                options: exact_openmp_options(
                    crate::ROUP_OMP_VERSION_6_0,
                    crate::ROUP_HOST_CPP,
                    crate::ROUP_CPP_23,
                    crate::ROUP_SOURCE_PRAGMA,
                ),
                source: "#pragma omp declare_reduction(ns::operator+ : widget) combiner(omp_out += omp_in) initializer(omp_priv = omp_orig)",
                type_name: "widget",
            },
            Case {
                options: exact_openmp_options(
                    crate::ROUP_OMP_VERSION_5_2,
                    crate::ROUP_HOST_FORTRAN,
                    crate::ROUP_FORTRAN_2023,
                    crate::ROUP_SOURCE_FORTRAN_FREE,
                ),
                source: "!$omp declare reduction(IAND : integer : omp_out = iand(omp_in, omp_out)) initializer(omp_priv = 0)",
                type_name: "integer",
            },
            Case {
                options: exact_openmp_options(
                    crate::ROUP_OMP_VERSION_6_0,
                    crate::ROUP_HOST_FORTRAN,
                    crate::ROUP_FORTRAN_2023,
                    crate::ROUP_SOURCE_FORTRAN_FREE,
                ),
                source: "!$omp declare_reduction(.COMBINE. : integer) combiner(omp_out = combine_values(omp_out, omp_in)) initializer(omp_priv = omp_orig)",
                type_name: "integer",
            },
        ];

        for case in cases {
            let parser = roup_parser_create(case.options);
            assert!(parser.result.status.is_ok());
            let directive = parse_success(parser.value, case.source);

            let kind = roup_directive_parameter_kind(directive);
            assert!(kind.result.status.is_ok());
            assert_eq!(kind.value.dialect, ROUP_DIALECT_OPENMP);
            assert_eq!(
                kind.value.variant,
                crate::ROUP_OMP_PARAMETER_DECLARE_REDUCTION
            );
            assert_eq!(roup_directive_parameter_field_count(directive).value, 4);

            for (field_index, expected_id, expected_kind, expected_count) in [
                (0, crate::ROUP_FIELD_NAME, crate::ROUP_FIELD_VALUE_NODE, 1),
                (
                    1,
                    crate::ROUP_FIELD_VALUES,
                    crate::ROUP_FIELD_VALUE_STRING_LIST,
                    1,
                ),
                (
                    2,
                    crate::ROUP_FIELD_COMBINER,
                    crate::ROUP_FIELD_VALUE_NODE,
                    1,
                ),
                (
                    3,
                    crate::ROUP_FIELD_INITIALIZER,
                    crate::ROUP_FIELD_VALUE_NODE,
                    1,
                ),
            ] {
                let field = roup_directive_parameter_field_info(directive, field_index);
                assert!(field.result.status.is_ok());
                assert_eq!(field.value.id, expected_id, "source: {:?}", case.source);
                assert_eq!(
                    field.value.value_kind, expected_kind,
                    "source: {:?}",
                    case.source
                );
                assert_eq!(
                    field.value.count, expected_count,
                    "source: {:?}",
                    case.source
                );
            }

            assert_eq!(parameter_string(directive, 1, 0), case.type_name);
            for (field_index, family) in [
                (0, crate::ROUP_NODE_FAMILY_OMP_IDENTIFIER),
                (2, crate::ROUP_NODE_FAMILY_OMP_STYLIZED_EXPRESSION),
                (3, crate::ROUP_NODE_FAMILY_OMP_REDUCTION_INITIALIZER),
            ] {
                let node = roup_directive_parameter_field_node(directive, field_index, 0);
                assert!(node.result.status.is_ok());
                assert_eq!(roup_node_kind(node.value).value.family, family);
                assert!(roup_node_release(node.value).status.is_ok());
            }

            assert!(roup_directive_release(directive).status.is_ok());
            assert!(roup_parser_release(parser.value).status.is_ok());
        }
    }

    #[test]
    fn declare_induction_parameter_exposes_typed_type_specifiers() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp declare induction (+ : int, (long, short)) collector(omp_out + omp_in) inductor(omp_priv + omp_step)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        if !directive.result.status.is_ok() {
            let message = unsafe {
                copied_string(
                    roup_error_message_length(directive.result.error),
                    |out, len| roup_error_message_copy(directive.result.error, out, len),
                )
            };
            panic!("declare induction parse failed: {message}");
        }

        let kind = roup_directive_parameter_kind(directive.value);
        assert!(kind.result.status.is_ok());
        assert_eq!(kind.value.dialect, ROUP_DIALECT_OPENMP);
        assert_eq!(
            kind.value.variant,
            crate::ROUP_OMP_PARAMETER_DECLARE_INDUCTION
        );
        let identifier_info = roup_directive_parameter_field_info(directive.value, 0);
        assert!(identifier_info.result.status.is_ok());
        assert_eq!(identifier_info.value.id, crate::ROUP_FIELD_NAME);
        assert_eq!(
            identifier_info.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE
        );
        let identifier = roup_directive_parameter_field_node(directive.value, 0, 0);
        assert!(identifier.result.status.is_ok());
        assert_eq!(
            roup_node_kind(identifier.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_IDENTIFIER,
                variant: crate::ROUP_OMP_IDENTIFIER_ADD,
            }
        );

        let type_info = roup_directive_parameter_field_info(directive.value, 1);
        assert!(type_info.result.status.is_ok());
        assert_eq!(type_info.value.id, crate::ROUP_FIELD_TYPE_SPECIFIERS);
        assert_eq!(
            type_info.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE_LIST
        );
        assert_eq!(type_info.value.count, 2);

        let same = roup_directive_parameter_field_node(directive.value, 1, 0);
        let pair = roup_directive_parameter_field_node(directive.value, 1, 1);
        assert!(same.result.status.is_ok());
        assert!(pair.result.status.is_ok());
        assert_eq!(
            roup_node_kind(same.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_INDUCTION_TYPE,
                variant: crate::ROUP_INDUCTION_TYPE_SAME,
            }
        );
        assert_eq!(
            roup_node_kind(pair.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_INDUCTION_TYPE,
                variant: crate::ROUP_INDUCTION_TYPE_PAIR,
            }
        );
        assert_eq!(
            roup_node_field_info(same.value, 0).value.id,
            crate::ROUP_FIELD_TYPE_NAME
        );
        assert_eq!(
            roup_node_field_info(pair.value, 0).value.id,
            crate::ROUP_FIELD_VARIABLE_TYPE
        );
        assert_eq!(
            roup_node_field_info(pair.value, 1).value.id,
            crate::ROUP_FIELD_STEP_TYPE
        );
        assert_eq!(node_string(same.value, 0, 0), "int");
        assert_eq!(node_string(pair.value, 0, 0), "long");
        assert_eq!(node_string(pair.value, 1, 0), "short");

        assert!(roup_node_release(identifier.value).status.is_ok());
        assert!(roup_node_release(same.value).status.is_ok());
        assert!(roup_node_release(pair.value).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn cpp_induction_id_and_type_pair_remain_separate_c_abi_fields() {
        let parser = roup_parser_create(exact_openmp_options(
            crate::ROUP_OMP_VERSION_6_0,
            crate::ROUP_HOST_CPP,
            crate::ROUP_CPP_23,
            crate::ROUP_SOURCE_PRAGMA,
        ));
        assert!(parser.result.status.is_ok());
        let source = "#pragma omp declare_induction(ns::step<int> : (state_t, step_t)) inductor(omp_var += omp_step) collector(omp_step * omp_idx)";
        let directive = parse_success(parser.value, source);

        assert_eq!(
            roup_directive_parameter_kind(directive).value.variant,
            crate::ROUP_OMP_PARAMETER_DECLARE_INDUCTION
        );
        let identifier = roup_directive_parameter_field_node(directive, 0, 0);
        assert!(identifier.result.status.is_ok());
        assert_eq!(
            roup_node_kind(identifier.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_IDENTIFIER,
                variant: crate::ROUP_OMP_IDENTIFIER_NAME,
            }
        );
        let id_expression = roup_node_field_node(identifier.value, 0, 0);
        assert!(id_expression.result.status.is_ok());
        assert_eq!(
            roup_node_kind(id_expression.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_ID_EXPRESSION,
                variant: crate::ROUP_OMP_ID_EXPRESSION_CPP_TEMPLATE_ID,
            }
        );
        assert_eq!(node_string(id_expression.value, 0, 0), "ns::step<int>");
        let type_info = roup_directive_parameter_field_info(directive, 1);
        assert_eq!(type_info.value.id, crate::ROUP_FIELD_TYPE_SPECIFIERS);
        assert_eq!(
            type_info.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE_LIST
        );
        assert_eq!(type_info.value.count, 1);

        let pair = roup_directive_parameter_field_node(directive, 1, 0);
        assert!(pair.result.status.is_ok());
        assert_eq!(
            roup_node_kind(pair.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_INDUCTION_TYPE,
                variant: crate::ROUP_INDUCTION_TYPE_PAIR,
            }
        );
        assert_eq!(node_string(pair.value, 0, 0), "state_t");
        assert_eq!(node_string(pair.value, 1, 0), "step_t");

        assert!(roup_node_release(id_expression.value).status.is_ok());
        assert!(roup_node_release(identifier.value).status.is_ok());
        assert!(roup_node_release(pair.value).status.is_ok());
        assert!(roup_directive_release(directive).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn ordered_expression_lists_and_detach_events_have_distinct_fields() {
        let parser = roup_parser_create(openmp_options());

        let list_source = "#pragma omp tile sizes(f(1, 2), n + 1)";
        let list = unsafe { roup_parse(parser.value, list_source.as_ptr(), list_source.len()) };
        assert!(list.result.status.is_ok());
        let values = roup_clause_field_info(list.value, 0, 0);
        assert!(values.result.status.is_ok());
        assert_eq!(values.value.id, crate::ROUP_FIELD_VALUES);
        assert_eq!(values.value.value_kind, crate::ROUP_FIELD_VALUE_STRING_LIST);
        assert_eq!(values.value.count, 2);
        let first = unsafe {
            copied_string(
                roup_clause_field_string_length(list.value, 0, 0, 0),
                |out, len| roup_clause_field_string_copy(list.value, 0, 0, 0, out, len),
            )
        };
        let second = unsafe {
            copied_string(
                roup_clause_field_string_length(list.value, 0, 0, 1),
                |out, len| roup_clause_field_string_copy(list.value, 0, 0, 1, out, len),
            )
        };
        assert_eq!(first, "f(1, 2)");
        assert_eq!(second, "n + 1");
        assert!(roup_directive_release(list.value).status.is_ok());

        let detach_source = "#pragma omp task detach(event_handle)";
        let detach =
            unsafe { roup_parse(parser.value, detach_source.as_ptr(), detach_source.len()) };
        assert!(detach.result.status.is_ok());
        let event = roup_clause_field_info(detach.value, 0, 0);
        assert!(event.result.status.is_ok());
        assert_eq!(event.value.id, crate::ROUP_FIELD_EVENT);
        assert_eq!(event.value.value_kind, crate::ROUP_FIELD_VALUE_STRING);
        let event_name = unsafe {
            copied_string(
                roup_clause_field_string_length(detach.value, 0, 0, 0),
                |out, len| roup_clause_field_string_copy(detach.value, 0, 0, 0, out, len),
            )
        };
        assert_eq!(event_name, "event_handle");
        assert!(roup_directive_release(detach.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn invalid_raw_configuration_is_a_queryable_hard_error() {
        let mut options = openmp_options();
        options.dialect = 99;
        let parser = roup_parser_create(options);
        assert_eq!(parser.result.status, RoupStatus::INVALID_ARGUMENT);
        assert!(parser.result.error.is_active());
        assert_eq!(roup_error_code(parser.result.error).value, 1000);
        assert!(roup_error_release(parser.result.error).status.is_ok());
    }

    #[test]
    fn invalid_utf8_is_rejected_before_the_safe_parser() {
        let parser = roup_parser_create(openmp_options());
        let source = [b'#', 0xff];
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert_eq!(directive.result.status, RoupStatus::INVALID_UTF8);
        assert_eq!(
            roup_error_code(directive.result.error).value,
            crate::ROUP_DIAGNOSTIC_INVALID_UTF8
        );
        assert!(roup_error_release(directive.result.error).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn released_directive_handle_is_stale() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp parallel";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(roup_directive_release(directive.value).status.is_ok());

        let stale = roup_directive_clause_count(directive.value);
        assert_eq!(stale.result.status, RoupStatus::STALE_HANDLE);
        assert!(roup_error_release(stale.result.error).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_structured_fields_preserve_wait_components() {
        let parser = roup_parser_create(openacc_options());
        let source = "#pragma acc parallel wait(devnum: 1: queues: 2, 3)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());
        assert_eq!(roup_clause_field_count(directive.value, 0).value, 2);

        let device = roup_clause_field_info(directive.value, 0, 0);
        let queues = roup_clause_field_info(directive.value, 0, 1);
        assert_eq!(device.value.id, crate::ROUP_FIELD_DEVICE_NUM);
        assert_eq!(queues.value.id, crate::ROUP_FIELD_VALUES);
        assert_eq!(queues.value.count, 2);

        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_worker_and_vector_use_optional_scalar_fields() {
        let parser = roup_parser_create(openacc_options());
        let source = "#pragma acc loop worker(num: 8) vector";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        assert_eq!(roup_clause_field_count(directive.value, 0).value, 2);
        let modifier = roup_clause_field_info(directive.value, 0, 0);
        let value = roup_clause_field_info(directive.value, 0, 1);
        assert_eq!(modifier.value.id, crate::ROUP_FIELD_MODIFIER);
        assert_eq!(modifier.value.value_kind, crate::ROUP_FIELD_VALUE_U32);
        assert_eq!(
            roup_clause_field_u32(directive.value, 0, 0, 0).value,
            crate::ROUP_ACC_WORKER_NUM
        );
        let wrong = roup_clause_field_string_length(directive.value, 0, 0, 0);
        assert!(!wrong.result.status.is_ok());
        assert!(roup_error_release(wrong.result.error).status.is_ok());
        assert_eq!(value.value.id, crate::ROUP_FIELD_VALUE);
        assert_eq!(value.value.value_kind, crate::ROUP_FIELD_VALUE_STRING);
        assert_eq!(value.value.count, 1);

        assert_eq!(roup_clause_field_count(directive.value, 1).value, 0);

        let malformed = "#pragma acc loop worker(first, second)";
        let error = unsafe { roup_parse(parser.value, malformed.as_ptr(), malformed.len()) };
        assert!(!error.result.status.is_ok());
        assert!(error.result.error.is_active());
        assert!(roup_error_release(error.result.error).status.is_ok());

        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_bind_target_is_a_tagged_name_or_string_node() {
        let parser = roup_parser_create(openacc_options());
        for (source, variant, expected) in [
            (
                "#pragma acc routine bind(device_entry)",
                crate::ROUP_ACC_BIND_NAME,
                "device_entry",
            ),
            (
                "#pragma acc routine bind(\"device_entry\")",
                crate::ROUP_ACC_BIND_STRING_LITERAL,
                "device_entry",
            ),
        ] {
            let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
            assert!(directive.result.status.is_ok());
            let field = roup_clause_field_info(directive.value, 0, 0);
            assert_eq!(field.value.id, crate::ROUP_FIELD_VALUE);
            assert_eq!(field.value.value_kind, crate::ROUP_FIELD_VALUE_NODE);

            let target = roup_clause_field_node(directive.value, 0, 0, 0);
            assert!(target.result.status.is_ok());
            assert_eq!(
                roup_node_kind(target.value).value,
                RoupNodeKind {
                    family: crate::ROUP_NODE_FAMILY_ACC_BIND_TARGET,
                    variant,
                }
            );
            let value = unsafe {
                copied_string(
                    roup_node_field_string_length(target.value, 0, 0),
                    |out, len| roup_node_field_string_copy(target.value, 0, 0, out, len),
                )
            };
            assert_eq!(value, expected);

            assert!(roup_node_release(target.value).status.is_ok());
            assert!(roup_directive_release(directive.value).status.is_ok());
        }
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_automatic_sizes_are_tagged_nodes() {
        let parser = roup_parser_create(openacc_options());
        let source = "#pragma acc loop tile(*, 8)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        let sizes = roup_clause_field_info(directive.value, 0, 0);
        assert!(sizes.result.status.is_ok());
        assert_eq!(sizes.value.id, crate::ROUP_FIELD_VALUES);
        assert_eq!(sizes.value.value_kind, crate::ROUP_FIELD_VALUE_NODE_LIST);
        assert_eq!(sizes.value.count, 2);

        let automatic = roup_clause_field_node(directive.value, 0, 0, 0);
        assert!(automatic.result.status.is_ok());
        assert_eq!(
            roup_node_kind(automatic.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_ACC_SIZE_EXPRESSION,
                variant: crate::ROUP_ACC_SIZE_AUTOMATIC,
            }
        );
        assert_eq!(roup_node_field_count(automatic.value).value, 0);

        let expression = roup_clause_field_node(directive.value, 0, 0, 1);
        assert!(expression.result.status.is_ok());
        assert_eq!(
            roup_node_kind(expression.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_ACC_SIZE_EXPRESSION,
                variant: crate::ROUP_ACC_SIZE_EXPRESSION,
            }
        );
        let value = unsafe {
            copied_string(
                roup_node_field_string_length(expression.value, 0, 0),
                |out, len| roup_node_field_string_copy(expression.value, 0, 0, out, len),
            )
        };
        assert_eq!(value, "8");

        assert!(roup_node_release(automatic.value).status.is_ok());
        assert!(roup_node_release(expression.value).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn openacc_gang_arguments_keep_order_and_argument_kinds() {
        let parser = roup_parser_create(openacc_options());
        let source = "#pragma acc loop gang(4, dim: 2, static: *)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        let arguments = roup_clause_field_info(directive.value, 0, 0);
        assert!(arguments.result.status.is_ok());
        assert_eq!(arguments.value.id, crate::ROUP_FIELD_ARGUMENTS);
        assert_eq!(
            arguments.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE_LIST
        );
        assert_eq!(arguments.value.count, 3);

        for (index, variant) in [
            crate::ROUP_ACC_GANG_POSITIONAL,
            crate::ROUP_ACC_GANG_DIM,
            crate::ROUP_ACC_GANG_STATIC,
        ]
        .into_iter()
        .enumerate()
        {
            let argument = roup_clause_field_node(directive.value, 0, 0, index);
            assert!(argument.result.status.is_ok());
            assert_eq!(
                roup_node_kind(argument.value).value,
                RoupNodeKind {
                    family: crate::ROUP_NODE_FAMILY_ACC_GANG_ARGUMENT,
                    variant,
                }
            );
            assert_eq!(roup_node_field_count(argument.value).value, 1);
            assert!(roup_node_release(argument.value).status.is_ok());
        }

        assert!(roup_directive_release(directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn tagged_payload_records_use_owned_recursive_node_handles() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp tile sizes(4) apply(grid: reverse)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        let modifier_field = roup_clause_field_info(directive.value, 1, 0);
        assert!(modifier_field.result.status.is_ok());
        assert_eq!(modifier_field.value.id, crate::ROUP_FIELD_LOOP_MODIFIER);
        let modifier = roup_clause_field_node(directive.value, 1, 0, 0);
        assert!(modifier.result.status.is_ok());
        assert_eq!(
            roup_node_kind(modifier.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER,
                variant: crate::ROUP_OMP_APPLY_GRID,
            }
        );

        let directives_field = roup_clause_field_info(directive.value, 1, 1);
        assert!(directives_field.result.status.is_ok());
        assert_eq!(
            directives_field.value.id,
            crate::ROUP_FIELD_APPLIED_DIRECTIVES
        );
        let applied = roup_clause_field_node(directive.value, 1, 1, 0);
        assert!(applied.result.status.is_ok());
        assert_eq!(
            roup_node_kind(applied.value).value.family,
            crate::ROUP_NODE_FAMILY_OMP_DIRECTIVE
        );

        assert!(roup_directive_release(directive.value).status.is_ok());
        assert_eq!(
            roup_node_kind(modifier.value).value.family,
            crate::ROUP_NODE_FAMILY_OMP_APPLY_MODIFIER
        );
        assert_eq!(
            roup_node_kind(applied.value).value.family,
            crate::ROUP_NODE_FAMILY_OMP_DIRECTIVE
        );
        assert!(roup_node_release(modifier.value).status.is_ok());
        assert!(roup_node_release(applied.value).status.is_ok());
        let stale = roup_node_kind(modifier.value);
        assert_eq!(stale.result.status, RoupStatus::STALE_HANDLE);
        assert!(roup_error_release(stale.result.error).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }

    #[test]
    fn uses_allocator_nodes_expose_only_canonical_semantics() {
        let parser = roup_parser_create(openmp_options());
        let source = "#pragma omp target uses_allocators(memspace(omp_high_bw_mem_space), traits(custom_traits): custom_allocator)";
        let directive = unsafe { roup_parse(parser.value, source.as_ptr(), source.len()) };
        assert!(directive.result.status.is_ok());

        let allocators = roup_clause_field_info(directive.value, 0, 0);
        assert!(allocators.result.status.is_ok());
        assert_eq!(allocators.value.id, crate::ROUP_FIELD_ALLOCATORS);
        assert_eq!(
            allocators.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE_LIST
        );
        assert_eq!(allocators.value.count, 1);
        let node = roup_clause_field_node(directive.value, 0, 0, 0);
        assert!(node.result.status.is_ok());
        assert_eq!(roup_node_field_count(node.value).value, 3);

        let allocator_info = roup_node_field_info(node.value, 0);
        assert_eq!(allocator_info.value.id, crate::ROUP_FIELD_ALLOCATOR);
        assert_eq!(
            allocator_info.value.value_kind,
            crate::ROUP_FIELD_VALUE_NODE
        );
        let allocator = roup_node_field_node(node.value, 0, 0);
        assert!(allocator.result.status.is_ok());
        assert_eq!(
            roup_node_kind(allocator.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_ALLOCATOR_KIND,
                variant: crate::ROUP_OMP_ALLOCATOR_CUSTOM,
            }
        );
        assert_eq!(node_string(allocator.value, 0, 0), "custom_allocator");
        assert_eq!(
            roup_node_field_info(node.value, 1).value.id,
            crate::ROUP_FIELD_TRAITS
        );
        assert_eq!(
            roup_node_field_info(node.value, 2).value.id,
            crate::ROUP_FIELD_MEMSPACE
        );
        assert!(roup_node_release(allocator.value).status.is_ok());
        assert!(roup_node_release(node.value).status.is_ok());
        assert!(roup_directive_release(directive.value).status.is_ok());

        let null_source = "#pragma omp target uses_allocators(omp_null_allocator)";
        let null_directive =
            unsafe { roup_parse(parser.value, null_source.as_ptr(), null_source.len()) };
        assert!(null_directive.result.status.is_ok());
        let null_node = roup_clause_field_node(null_directive.value, 0, 0, 0);
        let null_allocator = roup_node_field_node(null_node.value, 0, 0);
        assert!(null_allocator.result.status.is_ok());
        assert_eq!(
            roup_node_kind(null_allocator.value).value,
            RoupNodeKind {
                family: crate::ROUP_NODE_FAMILY_OMP_ALLOCATOR_KIND,
                variant: crate::ROUP_OMP_ALLOCATOR_NULL,
            }
        );
        assert_eq!(roup_node_field_count(null_allocator.value).value, 0);
        assert!(roup_node_release(null_allocator.value).status.is_ok());
        assert!(roup_node_release(null_node.value).status.is_ok());
        assert!(roup_directive_release(null_directive.value).status.is_ok());
        assert!(roup_parser_release(parser.value).status.is_ok());
    }
}
