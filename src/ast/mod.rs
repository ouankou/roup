//! Enum-based AST shared between the parser, IR, and compat layers.
//!
//! This module defines language-specific directive/clause enums plus the
//! strongly typed payload structures that downstream consumers will rely on.
//! The goal is to eliminate every post-parse string/number inspection so the
//! parser becomes the single place that interprets tokens.
#![forbid(unsafe_code)]

use std::convert::TryFrom;
use std::fmt;

use crate::host::TypeName;
use crate::ir::{ClauseData, ClauseItem, Expression, Identifier, LValue, Variable};
use crate::parser::ClauseName;
use crate::parser::directive_kind::DirectiveName;
use crate::source::Span;

/// Re-export the typed OpenMP clause payload primitives from the semantic IR.
pub type OmpClausePayload = ClauseData;
pub type OmpClauseItem = ClauseItem;
pub type OmpIdentifier = Identifier;
pub type OmpVariable = Variable;

/// A parsed directive whose enum tag is the single source of dialect truth.
#[derive(Debug, Clone)]
pub enum RoupDirective {
    OpenMp(Box<OmpDirective>),
    OpenAcc(Box<AccDirective>),
}

impl RoupDirective {
    #[must_use]
    pub const fn dialect(&self) -> crate::version::Dialect {
        match self {
            Self::OpenMp(_) => crate::version::Dialect::OpenMp,
            Self::OpenAcc(_) => crate::version::Dialect::OpenAcc,
        }
    }

    #[must_use]
    pub const fn as_openmp(&self) -> Option<&OmpDirective> {
        match self {
            Self::OpenMp(directive) => Some(directive),
            Self::OpenAcc(_) => None,
        }
    }

    #[must_use]
    pub const fn as_openacc(&self) -> Option<&AccDirective> {
        match self {
            Self::OpenAcc(directive) => Some(directive),
            Self::OpenMp(_) => None,
        }
    }
}

/// Fully structured OpenMP directive.
#[derive(Debug, Clone)]
pub struct OmpDirective {
    kind: OmpDirectiveKind,
    parameter: Option<OmpDirectiveParameter>,
    clauses: Vec<OmpClause>,
    source_alias: Option<OmpDirectiveSourceAlias>,
    span: Span,
}

/// Standardized source spelling canonicalized to an [`OmpDirectiveKind`].
///
/// This provenance is private because it affects syntax-version checks, not
/// the directive's semantic identity. Raw source text never enters the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OmpDirectiveSourceAlias {
    /// An underscore-bearing directive name standardized in OpenMP 6.0.
    OpenMp60Underscore,
    /// A Fortran spelling with omitted blanks between adjacent keywords.
    FortranCompact,
}

/// Fully structured OpenACC directive.
#[derive(Debug, Clone)]
pub struct AccDirective {
    kind: AccDirectiveKind,
    parameter: Option<AccDirectiveParameter>,
    clauses: Vec<AccClause>,
    span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AstInvariantError {
    message: &'static str,
}

impl AstInvariantError {
    const fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl fmt::Display for AstInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for AstInvariantError {}

impl OmpDirective {
    pub(crate) fn new(
        kind: OmpDirectiveKind,
        parameter: Option<OmpDirectiveParameter>,
        clauses: Vec<OmpClause>,
        source_alias: Option<OmpDirectiveSourceAlias>,
        span: Span,
    ) -> Result<Self, AstInvariantError> {
        if !omp_parameter_matches_kind(kind, parameter.as_ref()) {
            return Err(AstInvariantError::new(
                "OpenMP directive kind does not match its typed parameter",
            ));
        }
        if span.is_empty() {
            return Err(AstInvariantError::new(
                "OpenMP directive name span must not be empty",
            ));
        }
        if !omp_directive_alias_matches_kind(source_alias, kind) {
            return Err(AstInvariantError::new(
                "OpenMP directive alias provenance does not match its canonical kind",
            ));
        }
        Ok(Self {
            kind,
            parameter,
            clauses,
            source_alias,
            span,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> OmpDirectiveKind {
        self.kind
    }

    #[must_use]
    pub const fn parameter(&self) -> Option<&OmpDirectiveParameter> {
        self.parameter.as_ref()
    }

    #[must_use]
    pub fn clauses(&self) -> &[OmpClause] {
        &self.clauses
    }

    #[must_use]
    pub(crate) const fn source_alias(&self) -> Option<OmpDirectiveSourceAlias> {
        self.source_alias
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for OmpDirective {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.parameter == other.parameter
            && self.clauses == other.clauses
    }
}

impl AccDirective {
    pub(crate) fn new(
        kind: AccDirectiveKind,
        parameter: Option<AccDirectiveParameter>,
        clauses: Vec<AccClause>,
        span: Span,
    ) -> Result<Self, AstInvariantError> {
        if !acc_parameter_matches_kind(kind, parameter.as_ref()) {
            return Err(AstInvariantError::new(
                "OpenACC directive kind does not match its typed parameter",
            ));
        }
        if span.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC directive name span must not be empty",
            ));
        }
        Ok(Self {
            kind,
            parameter,
            clauses,
            span,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> AccDirectiveKind {
        self.kind
    }

    #[must_use]
    pub const fn parameter(&self) -> Option<&AccDirectiveParameter> {
        self.parameter.as_ref()
    }

    #[must_use]
    pub fn clauses(&self) -> &[AccClause] {
        &self.clauses
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for AccDirective {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.parameter == other.parameter
            && self.clauses == other.clauses
    }
}

/// Typed OpenMP clause record.
#[derive(Debug, Clone)]
pub struct OmpClause {
    kind: OmpClauseKind,
    payload: OmpClausePayload,
    directive_name_modifier: Option<OmpDirectiveKind>,
    source_alias: Option<OmpClauseSourceAlias>,
    span: Span,
}

/// Historical standardized spelling accepted before canonical AST lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OmpClauseSourceAlias {
    DependSource,
    DependSourceCurrent,
    DependSink,
    DependSinkPreviousCurrent,
    MetadirectiveDefault,
    DeclareTargetTo,
    ProcBindMaster,
}

impl OmpClause {
    pub(crate) fn new(
        kind: OmpClauseKind,
        payload: OmpClausePayload,
        directive_name_modifier: Option<OmpDirectiveKind>,
        source_alias: Option<OmpClauseSourceAlias>,
        span: Span,
    ) -> Result<Self, AstInvariantError> {
        if !omp_payload_matches_kind(kind, &payload) {
            return Err(AstInvariantError::new(
                "OpenMP clause kind does not match its typed payload",
            ));
        }
        if !omp_payload_has_required_contents(&payload) {
            return Err(AstInvariantError::new(
                "OpenMP clause payload is missing required typed data",
            ));
        }
        if !omp_alias_matches_kind(source_alias, kind) {
            return Err(AstInvariantError::new(
                "OpenMP clause alias provenance does not match its canonical kind",
            ));
        }
        if span.is_empty() {
            return Err(AstInvariantError::new(
                "OpenMP clause name span must not be empty",
            ));
        }
        Ok(Self {
            kind,
            payload,
            directive_name_modifier,
            source_alias,
            span,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> OmpClauseKind {
        self.kind
    }

    #[must_use]
    pub const fn payload(&self) -> &OmpClausePayload {
        &self.payload
    }

    /// The construct or constituent construct to which this clause applies.
    ///
    /// OpenMP 6.0 makes this modifier universal. It remains represented here
    /// for historical `if` syntax introduced by OpenMP 4.5 as well.
    #[must_use]
    pub const fn directive_name_modifier(&self) -> Option<OmpDirectiveKind> {
        self.directive_name_modifier
    }

    #[must_use]
    pub(crate) const fn source_alias(&self) -> Option<OmpClauseSourceAlias> {
        self.source_alias
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for OmpClause {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.payload == other.payload
            && self.directive_name_modifier == other.directive_name_modifier
    }
}

/// Typed OpenACC clause record.
#[derive(Debug, Clone)]
pub struct AccClause {
    kind: AccClauseKind,
    payload: AccClausePayload,
    source_alias: Option<AccClauseSourceAlias>,
    span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AccClauseSourceAlias {
    PCopy,
    PresentOrCopy,
    PCopyIn,
    PresentOrCopyIn,
    PCopyOut,
    PresentOrCopyOut,
    PCreate,
    PresentOrCreate,
    UpdateHost,
}

impl AccClause {
    pub(crate) fn new(
        kind: AccClauseKind,
        payload: AccClausePayload,
        source_alias: Option<AccClauseSourceAlias>,
        span: Span,
    ) -> Result<Self, AstInvariantError> {
        if !acc_payload_matches_kind(kind, &payload) {
            return Err(AstInvariantError::new(
                "OpenACC clause kind does not match its typed payload",
            ));
        }
        if matches!(kind, AccClauseKind::DevicePtr | AccClauseKind::Present)
            && matches!(
                &payload,
                AccClausePayload::ItemList(items)
                    if items.iter().any(|item| matches!(
                        item,
                        ClauseItem::FortranCommonBlock(_)
                    ))
            )
        {
            return Err(AstInvariantError::new(
                "OpenACC deviceptr and present clauses do not accept common block names",
            ));
        }
        if !acc_payload_has_required_contents(&payload) {
            return Err(AstInvariantError::new(
                "OpenACC clause payload is missing required typed data",
            ));
        }
        if !acc_alias_matches_kind(source_alias, kind) {
            return Err(AstInvariantError::new(
                "OpenACC clause alias provenance does not match its canonical kind",
            ));
        }
        if span.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC clause name span must not be empty",
            ));
        }
        Ok(Self {
            kind,
            payload,
            source_alias,
            span,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> AccClauseKind {
        self.kind
    }

    #[must_use]
    pub const fn payload(&self) -> &AccClausePayload {
        &self.payload
    }

    #[must_use]
    pub(crate) const fn source_alias(&self) -> Option<AccClauseSourceAlias> {
        self.source_alias
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for AccClause {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.payload == other.payload
    }
}

/// Metadirective selector payload (typed, no post-parse strings).
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelector {
    entries: Vec<OmpSelectorEntry>,
    nested_directive: Option<Box<OmpDirective>>,
}

impl OmpSelector {
    pub(crate) fn new(
        entries: Vec<OmpSelectorEntry>,
        nested_directive: Option<Box<OmpDirective>>,
    ) -> Result<Self, AstInvariantError> {
        if entries.is_empty() && nested_directive.is_none() {
            return Err(AstInvariantError::new(
                "OpenMP selector requires a context selector or nested directive",
            ));
        }

        let mut saw_device = false;
        let mut saw_target_device = false;
        let mut saw_implementation = false;
        let mut saw_user = false;
        let mut saw_construct = false;
        for entry in &entries {
            match entry {
                OmpSelectorEntry::Device { traits } => {
                    if traits.is_empty() || saw_device {
                        return Err(AstInvariantError::new(
                            "OpenMP selector device traits must be non-empty and unique",
                        ));
                    }
                    validate_device_selector_traits(traits, false)?;
                    saw_device = true;
                }
                OmpSelectorEntry::TargetDevice { traits } => {
                    if traits.is_empty() || saw_target_device {
                        return Err(AstInvariantError::new(
                            "OpenMP selector target_device traits must be non-empty and unique",
                        ));
                    }
                    validate_device_selector_traits(traits, true)?;
                    saw_target_device = true;
                }
                OmpSelectorEntry::Implementation { traits } => {
                    if traits.is_empty() || saw_implementation {
                        return Err(AstInvariantError::new(
                            "OpenMP selector implementation traits must be non-empty and unique",
                        ));
                    }
                    validate_implementation_selector_traits(traits)?;
                    saw_implementation = true;
                }
                OmpSelectorEntry::User { .. } => {
                    if saw_user {
                        return Err(AstInvariantError::new(
                            "OpenMP selector user trait must be unique",
                        ));
                    }
                    saw_user = true;
                }
                OmpSelectorEntry::Construct { constructs } => {
                    if constructs.is_empty() || saw_construct {
                        return Err(AstInvariantError::new(
                            "OpenMP selector constructs must be non-empty and unique",
                        ));
                    }
                    let mut kinds = std::collections::HashSet::new();
                    if constructs
                        .iter()
                        .any(|construct| !kinds.insert(construct.directive().kind()))
                    {
                        return Err(AstInvariantError::new(
                            "each construct trait may appear at most once in a selector set",
                        ));
                    }
                    saw_construct = true;
                }
            }
        }

        Ok(Self {
            entries,
            nested_directive,
        })
    }

    #[must_use]
    pub fn entries(&self) -> &[OmpSelectorEntry] {
        &self.entries
    }

    #[must_use]
    pub fn nested_directive(&self) -> Option<&OmpDirective> {
        self.nested_directive.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OmpSelectorEntry {
    Device {
        traits: Vec<OmpSelectorDeviceTrait>,
    },
    TargetDevice {
        traits: Vec<OmpSelectorDeviceTrait>,
    },
    Implementation {
        traits: Vec<OmpSelectorImplementationTrait>,
    },
    User {
        score: Option<Box<Expression>>,
        condition: Box<Expression>,
    },
    Construct {
        constructs: Vec<OmpSelectorConstruct>,
    },
}

/// A standardized name-list trait selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpSelectorNameListKind {
    Kind,
    Isa,
    Arch,
    Vendor,
    Extension,
}

/// One name-list trait with all of its properties retained as one semantic
/// node. A repeated property is invalid even when one occurrence uses an
/// identifier and the other uses the corresponding string literal.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorNameListTrait {
    kind: OmpSelectorNameListKind,
    properties: Vec<OmpSelectorTraitValue>,
}

impl OmpSelectorNameListTrait {
    pub(crate) fn new(
        kind: OmpSelectorNameListKind,
        properties: Vec<OmpSelectorTraitValue>,
    ) -> Result<Self, AstInvariantError> {
        if properties.is_empty() {
            return Err(AstInvariantError::new(
                "a name-list selector trait requires at least one property",
            ));
        }
        let mut names = std::collections::HashSet::new();
        if properties
            .iter()
            .any(|property| !names.insert(property.semantic_name()))
        {
            return Err(AstInvariantError::new(
                "a name-list selector trait cannot repeat a property",
            ));
        }
        Ok(Self { kind, properties })
    }

    #[must_use]
    pub const fn kind(&self) -> OmpSelectorNameListKind {
        self.kind
    }

    #[must_use]
    pub fn properties(&self) -> &[OmpSelectorTraitValue] {
        &self.properties
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OmpSelectorDeviceTrait {
    NameList(OmpSelectorNameListTrait),
    DeviceNum(Expression),
    Uid(OmpSelectorTraitValue),
    Extension(OmpSelectorExtensionTrait),
}

/// One implementation trait. Scores live only on this node, so the AST cannot
/// attach a score to a construct, device, target-device, or user trait.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorImplementationTrait {
    score: Option<Expression>,
    kind: OmpSelectorImplementationTraitKind,
}

impl OmpSelectorImplementationTrait {
    pub(crate) fn new(score: Option<Expression>, kind: OmpSelectorImplementationTraitKind) -> Self {
        Self { score, kind }
    }

    #[must_use]
    pub const fn score(&self) -> Option<&Expression> {
        self.score.as_ref()
    }

    #[must_use]
    pub const fn kind(&self) -> &OmpSelectorImplementationTraitKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OmpSelectorImplementationTraitKind {
    NameList(OmpSelectorNameListTrait),
    AtomicDefaultMemOrder(crate::ir::MemoryOrder),
    Requirement(crate::ir::RequireModifier),
    Requires(Vec<OmpSelectorRequirement>),
    Extension(OmpSelectorExtensionTrait),
}

/// One typed clause property in an implementation `requires(...)` trait.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorRequirement {
    requirement: crate::ir::RequireModifier,
    required: Option<Expression>,
}

impl OmpSelectorRequirement {
    pub(crate) fn new(
        requirement: crate::ir::RequireModifier,
        required: Option<Expression>,
    ) -> Self {
        Self {
            requirement,
            required,
        }
    }

    #[must_use]
    pub const fn requirement(&self) -> &crate::ir::RequireModifier {
        &self.requirement
    }

    #[must_use]
    pub const fn required(&self) -> Option<&Expression> {
        self.required.as_ref()
    }
}

/// An implementation-defined selector trait and its recursively typed
/// extension properties.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorExtensionTrait {
    name: Identifier,
    properties: Vec<OmpSelectorExtensionProperty>,
}

impl OmpSelectorExtensionTrait {
    pub(crate) fn new(
        name: Identifier,
        properties: Vec<OmpSelectorExtensionProperty>,
    ) -> Result<Self, AstInvariantError> {
        let mut unique = Vec::new();
        for property in &properties {
            validate_selector_extension_property(property)?;
            if unique.contains(property) {
                return Err(AstInvariantError::new(
                    "an extension selector trait cannot repeat a property",
                ));
            }
            unique.push(property.clone());
        }
        Ok(Self { name, properties })
    }

    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }

    #[must_use]
    pub fn properties(&self) -> &[OmpSelectorExtensionProperty] {
        &self.properties
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OmpSelectorExtensionProperty {
    Name(OmpSelectorTraitValue),
    Call {
        name: Identifier,
        properties: Vec<OmpSelectorExtensionProperty>,
    },
    ConstantInteger(Expression),
}

fn validate_device_selector_traits(
    traits: &[OmpSelectorDeviceTrait],
    target_device: bool,
) -> Result<(), AstInvariantError> {
    let mut names = std::collections::HashSet::new();
    let mut kind_any = false;
    for selector in traits {
        let key = match selector {
            OmpSelectorDeviceTrait::NameList(name_list) => {
                if !matches!(
                    name_list.kind(),
                    OmpSelectorNameListKind::Kind
                        | OmpSelectorNameListKind::Isa
                        | OmpSelectorNameListKind::Arch
                ) {
                    return Err(AstInvariantError::new(
                        "device selector contains an implementation name-list trait",
                    ));
                }
                if name_list.kind() == OmpSelectorNameListKind::Kind
                    && name_list
                        .properties()
                        .iter()
                        .any(|property| property.semantic_name() == "any")
                {
                    if name_list.properties().len() != 1 {
                        return Err(AstInvariantError::new(
                            "kind(any) cannot contain another property",
                        ));
                    }
                    kind_any = true;
                }
                format!("name-list:{:?}", name_list.kind())
            }
            OmpSelectorDeviceTrait::DeviceNum(_) => {
                if !target_device {
                    return Err(AstInvariantError::new(
                        "device_num is only valid in a target_device selector set",
                    ));
                }
                "device_num".to_string()
            }
            OmpSelectorDeviceTrait::Uid(_) => {
                if !target_device {
                    return Err(AstInvariantError::new(
                        "uid is only valid in a target_device selector set",
                    ));
                }
                "uid".to_string()
            }
            OmpSelectorDeviceTrait::Extension(extension) => {
                format!("extension:{}", extension.name())
            }
        };
        if !names.insert(key) {
            return Err(AstInvariantError::new(
                "each device trait selector may appear at most once",
            ));
        }
    }
    if kind_any && traits.len() != 1 {
        return Err(AstInvariantError::new(
            "kind(any) cannot be combined with another device trait property",
        ));
    }
    Ok(())
}

fn validate_implementation_selector_traits(
    traits: &[OmpSelectorImplementationTrait],
) -> Result<(), AstInvariantError> {
    let mut names = std::collections::HashSet::new();
    for selector in traits {
        let key = match selector.kind() {
            OmpSelectorImplementationTraitKind::NameList(name_list) => {
                if !matches!(
                    name_list.kind(),
                    OmpSelectorNameListKind::Vendor | OmpSelectorNameListKind::Extension
                ) {
                    return Err(AstInvariantError::new(
                        "implementation selector contains a device name-list trait",
                    ));
                }
                format!("name-list:{:?}", name_list.kind())
            }
            OmpSelectorImplementationTraitKind::AtomicDefaultMemOrder(_) => {
                "atomic_default_mem_order".to_string()
            }
            OmpSelectorImplementationTraitKind::Requirement(requirement) => {
                format!("requirement:{}", selector_requirement_name(requirement))
            }
            OmpSelectorImplementationTraitKind::Requires(requirements) => {
                if requirements.is_empty() {
                    return Err(AstInvariantError::new(
                        "requires selector trait needs at least one clause property",
                    ));
                }
                let mut clauses = std::collections::HashSet::new();
                if requirements.iter().any(|requirement| {
                    !clauses.insert(selector_requirement_name(requirement.requirement()))
                }) {
                    return Err(AstInvariantError::new(
                        "a requires selector trait cannot repeat a clause property",
                    ));
                }
                "requires".to_string()
            }
            OmpSelectorImplementationTraitKind::Extension(extension) => {
                format!("extension:{}", extension.name())
            }
        };
        if !names.insert(key) {
            return Err(AstInvariantError::new(
                "each implementation trait selector may appear at most once",
            ));
        }
    }
    Ok(())
}

fn selector_requirement_name(requirement: &crate::ir::RequireModifier) -> String {
    match requirement {
        crate::ir::RequireModifier::ReverseOffload => "reverse_offload".to_string(),
        crate::ir::RequireModifier::UnifiedAddress => "unified_address".to_string(),
        crate::ir::RequireModifier::UnifiedSharedMemory => "unified_shared_memory".to_string(),
        crate::ir::RequireModifier::DynamicAllocators => "dynamic_allocators".to_string(),
        crate::ir::RequireModifier::SelfMaps => "self_maps".to_string(),
        crate::ir::RequireModifier::DeviceSafesync => "device_safesync".to_string(),
        crate::ir::RequireModifier::AtomicDefaultMemOrder(_) => {
            "atomic_default_mem_order".to_string()
        }
        crate::ir::RequireModifier::ExtImplementationDefinedRequirement(Some(name)) => {
            name.to_string()
        }
        crate::ir::RequireModifier::ExtImplementationDefinedRequirement(None) => {
            "ext_implementation_defined_requirement".to_string()
        }
    }
}

fn validate_selector_extension_property(
    property: &OmpSelectorExtensionProperty,
) -> Result<(), AstInvariantError> {
    let OmpSelectorExtensionProperty::Call { properties, .. } = property else {
        return Ok(());
    };
    if properties.is_empty() {
        return Err(AstInvariantError::new(
            "an extension property call requires at least one nested property",
        ));
    }
    let mut unique = Vec::new();
    for nested in properties {
        validate_selector_extension_property(nested)?;
        if unique.contains(nested) {
            return Err(AstInvariantError::new(
                "an extension property call cannot repeat a nested property",
            ));
        }
        unique.push(nested.clone());
    }
    Ok(())
}

/// A validated selector trait property.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OmpSelectorTraitValue {
    Identifier(Identifier),
    StringLiteral(crate::host::StringLiteral),
}

impl OmpSelectorTraitValue {
    fn semantic_name(&self) -> &str {
        match self {
            Self::Identifier(identifier) => identifier.as_str(),
            Self::StringLiteral(literal) => literal.value.as_str(),
        }
    }
}

impl fmt::Display for OmpSelectorTraitValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier(identifier) => write!(f, "{identifier}"),
            Self::StringLiteral(literal)
                if literal.encoding == crate::host::CharacterEncoding::Fortran =>
            {
                f.write_str("'")?;
                for ch in literal.value.chars() {
                    if ch == '\'' {
                        f.write_str("''")?;
                    } else {
                        write!(f, "{ch}")?;
                    }
                }
                f.write_str("'")
            }
            Self::StringLiteral(literal) => {
                let prefix = match literal.encoding {
                    crate::host::CharacterEncoding::Ordinary => "",
                    crate::host::CharacterEncoding::Utf8 => "u8",
                    crate::host::CharacterEncoding::Utf16 => "u",
                    crate::host::CharacterEncoding::Utf32 => "U",
                    crate::host::CharacterEncoding::Wide => "L",
                    crate::host::CharacterEncoding::Fortran => "",
                };
                write!(f, "{prefix}\"")?;
                for ch in literal.value.chars() {
                    match ch {
                        '\\' => f.write_str("\\\\")?,
                        '\n' => f.write_str("\\n")?,
                        '\r' => f.write_str("\\r")?,
                        '\t' => f.write_str("\\t")?,
                        '\0' => f.write_str("\\000")?,
                        '\u{7}' => f.write_str("\\a")?,
                        '\u{8}' => f.write_str("\\b")?,
                        '\u{b}' => f.write_str("\\v")?,
                        '\u{c}' => f.write_str("\\f")?,
                        '\"' => f.write_str("\\\"")?,
                        value if value.is_control() => write!(f, "\\u{:04x}", value as u32)?,
                        value => write!(f, "{value}")?,
                    }
                }
                f.write_str("\"")
            }
        }
    }
}

/// Construct selector entry with optional score.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSelectorConstruct {
    directive: Box<OmpDirective>,
}

impl OmpSelectorConstruct {
    pub(crate) fn new(directive: OmpDirective) -> Self {
        Self {
            directive: Box::new(directive),
        }
    }

    #[must_use]
    pub const fn directive(&self) -> &OmpDirective {
        &self.directive
    }
}

/// One typed entry in an OpenMP `flush` variable list.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpFlushListItem {
    /// An unqualified variable name.
    Identifier(OmpIdentifier),
    /// A host-language variable designator, including array sections and
    /// structure or class members.
    Variable(OmpVariable),
    /// A Fortran named common block, spelled `/name/` in source.
    FortranCommonBlock(OmpIdentifier),
}

impl fmt::Display for OmpFlushListItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identifier(identifier) => identifier.fmt(formatter),
            Self::Variable(variable) => variable.fmt(formatter),
            Self::FortranCommonBlock(identifier) => write!(formatter, "/{identifier}/"),
        }
    }
}

/// One whole storage entity accepted by OpenMP declarative storage lists.
///
/// Array elements, array sections, and object members are deliberately absent:
/// the `allocate`, `threadprivate`, and `groupprivate` restrictions only admit
/// whole variables. C++ qualified names and Fortran named common blocks remain
/// distinct typed cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmpStorageListItem {
    Name(crate::host::QualifiedName),
    FortranCommonBlock(OmpIdentifier),
}

impl fmt::Display for OmpStorageListItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => format_qualified_name(formatter, name),
            Self::FortranCommonBlock(name) => write!(formatter, "/{name}/"),
        }
    }
}

/// One item in the historical `declare target(extended-list)` form.
///
/// The directive-specific type prevents that historical list from silently
/// acquiring the broader grammar of an unrelated variable or locator list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmpDeclareTargetListItem {
    /// A whole variable or procedure name.
    Name(crate::host::QualifiedName),
    /// A Fortran named common block.
    FortranCommonBlock(OmpIdentifier),
}

impl fmt::Display for OmpDeclareTargetListItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => format_qualified_name(formatter, name),
            Self::FortranCommonBlock(name) => write!(formatter, "/{name}/"),
        }
    }
}

/// A fully typed function name used by `declare variant`.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpFunctionName {
    Name(crate::host::QualifiedName),
    /// The C++ template-id form standardized before OpenMP 5.2.
    CppTemplateId(OmpCppTemplateId),
}

impl fmt::Display for OmpFunctionName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => format_qualified_name(formatter, name),
            Self::CppTemplateId(template_id) => template_id.fmt(formatter),
        }
    }
}

/// A lexed, delimiter-balanced C++ template-id.
///
/// [`TypeName`] supplies the existing typed host token storage and profile
/// checks. Construction is private to the parser, which additionally validates
/// the function-name prefix and the outer template argument list.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpCppTemplateId(TypeName);

impl OmpCppTemplateId {
    pub(crate) const fn new(syntax: TypeName) -> Self {
        Self(syntax)
    }

    #[must_use]
    pub fn tokens(&self) -> &[crate::host::TokenKind] {
        self.0.tokens()
    }
}

impl fmt::Display for OmpCppTemplateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_compact(formatter)
    }
}

/// Typed argument of a `declare variant` directive.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpDeclareVariantTarget {
    base: Option<OmpIdentifier>,
    variant: OmpFunctionName,
}

impl OmpDeclareVariantTarget {
    pub(crate) const fn new(base: Option<OmpIdentifier>, variant: OmpFunctionName) -> Self {
        Self { base, variant }
    }

    #[must_use]
    pub const fn base(&self) -> Option<&OmpIdentifier> {
        self.base.as_ref()
    }

    #[must_use]
    pub const fn variant(&self) -> &OmpFunctionName {
        &self.variant
    }
}

fn format_qualified_name(
    formatter: &mut fmt::Formatter<'_>,
    name: &crate::host::QualifiedName,
) -> fmt::Result {
    if name.global {
        formatter.write_str("::")?;
    }
    for (index, segment) in name.segments.iter().enumerate() {
        if index > 0 {
            formatter.write_str("::")?;
        }
        write!(formatter, "{segment}")?;
    }
    Ok(())
}

/// Additional syntax carried by OpenMP directives that accept custom
/// parameters outside the clause stream.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpDirectiveParameter {
    AllocateList(Vec<OmpStorageListItem>),
    ThreadprivateList(Vec<OmpStorageListItem>),
    GroupprivateList(Vec<OmpStorageListItem>),
    DeclareTargetList(Vec<OmpDeclareTargetListItem>),
    DeclareMapper(OmpDeclareMapper),
    DeclareVariant(OmpDeclareVariantTarget),
    Depobj(LValue),
    Construct(OmpConstructType),
    CriticalSection(OmpIdentifier),
    FlushList(Vec<OmpFlushListItem>),
    DeclareReduction(Box<OmpDeclareReduction>),
    DeclareInduction(OmpDeclareInduction),
    DeclareSimd(OmpSimdTarget),
}

/// OpenMP constructs accepted by `cancel` / `cancellation point` parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmpConstructType {
    Parallel,
    Sections,
    For,
    Taskgroup,
}

/// Declare reduction signature.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpDeclareReduction {
    identifier: OmpReductionIdentifier,
    type_names: Vec<TypeName>,
    combiner: OmpReductionCombiner,
    initializer: Option<OmpReductionInitializer>,
    source_syntax: OmpDeclareReductionSyntax,
}

/// Source grammar used for a declare-reduction directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OmpDeclareReductionSyntax {
    /// The standardized historical `identifier : types : combiner` argument.
    InlineCombiner,
    /// The OpenMP 6.0 `identifier : types` argument plus `combiner` clause.
    CombinerClause,
}

/// A base-language id-expression used as an OpenMP identifier.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpIdExpression {
    Name(crate::host::QualifiedName),
    CppTemplateId(OmpCppTemplateId),
    CppOperatorFunction(OmpCppOperatorFunctionId),
}

impl OmpIdExpression {
    #[must_use]
    pub const fn qualified_name(&self) -> Option<&crate::host::QualifiedName> {
        match self {
            Self::Name(name) => Some(name),
            Self::CppTemplateId(_) | Self::CppOperatorFunction(_) => None,
        }
    }
}

impl fmt::Display for OmpIdExpression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => format_qualified_name(formatter, name),
            Self::CppTemplateId(name) => name.fmt(formatter),
            Self::CppOperatorFunction(name) => name.fmt(formatter),
        }
    }
}

/// A validated qualified C++ operator-function-id.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpCppOperatorFunctionId {
    global: bool,
    qualifier: Option<OmpCppOperatorQualifier>,
    operator: OmpCppReductionOperator,
}

impl OmpCppOperatorFunctionId {
    pub(crate) const fn new(
        global: bool,
        qualifier: Option<OmpCppOperatorQualifier>,
        operator: OmpCppReductionOperator,
    ) -> Self {
        Self {
            global,
            qualifier,
            operator,
        }
    }

    #[must_use]
    pub const fn qualifier(&self) -> Option<&OmpCppOperatorQualifier> {
        self.qualifier.as_ref()
    }

    #[must_use]
    pub const fn is_global(&self) -> bool {
        self.global
    }

    #[must_use]
    pub const fn operator(&self) -> OmpCppReductionOperator {
        self.operator
    }
}

impl fmt::Display for OmpCppOperatorFunctionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.global {
            formatter.write_str("::")?;
        }
        if let Some(qualifier) = &self.qualifier {
            qualifier.fmt(formatter)?;
            formatter.write_str("::")?;
        }
        write!(formatter, "operator{}", self.operator)
    }
}

/// Qualifier of a C++ operator-function-id.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpCppOperatorQualifier {
    Name(crate::host::QualifiedName),
    TemplateId(OmpCppTemplateId),
}

impl fmt::Display for OmpCppOperatorQualifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => format_qualified_name(formatter, name),
            Self::TemplateId(name) => name.fmt(formatter),
        }
    }
}

/// Operator token of an id-expression usable as a reduction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpCppReductionOperator {
    Add,
    Subtract,
    Multiply,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LogicalAnd,
    LogicalOr,
}

impl fmt::Display for OmpCppReductionOperator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::BitwiseAnd => "&",
            Self::BitwiseOr => "|",
            Self::BitwiseXor => "^",
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
        })
    }
}

/// An intrinsic Fortran procedure name permitted as a reduction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OmpFortranReductionIntrinsic {
    Max,
    Min,
    Iand,
    Ior,
    Ieor,
}

impl fmt::Display for OmpFortranReductionIntrinsic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Max => "max",
            Self::Min => "min",
            Self::Iand => "iand",
            Self::Ior => "ior",
            Self::Ieor => "ieor",
        })
    }
}

/// Fully classified reduction identifier for the configured base language.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpReductionIdentifier {
    Add,
    /// Standardized through OpenMP 5.2 and accepted cumulatively thereafter.
    Subtract,
    Multiply,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LogicalAnd,
    LogicalOr,
    FortranLogicalAnd,
    FortranLogicalOr,
    FortranLogicalEqv,
    FortranLogicalNeqv,
    Name(OmpIdExpression),
    FortranIntrinsic(OmpFortranReductionIntrinsic),
    FortranDefinedOperator(Identifier),
}

impl fmt::Display for OmpReductionIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => formatter.write_str("+"),
            Self::Subtract => formatter.write_str("-"),
            Self::Multiply => formatter.write_str("*"),
            Self::BitwiseAnd => formatter.write_str("&"),
            Self::BitwiseOr => formatter.write_str("|"),
            Self::BitwiseXor => formatter.write_str("^"),
            Self::LogicalAnd => formatter.write_str("&&"),
            Self::LogicalOr => formatter.write_str("||"),
            Self::FortranLogicalAnd => formatter.write_str(".and."),
            Self::FortranLogicalOr => formatter.write_str(".or."),
            Self::FortranLogicalEqv => formatter.write_str(".eqv."),
            Self::FortranLogicalNeqv => formatter.write_str(".neqv."),
            Self::Name(name) => name.fmt(formatter),
            Self::FortranIntrinsic(name) => name.fmt(formatter),
            Self::FortranDefinedOperator(name) => write!(formatter, ".{name}."),
        }
    }
}

/// Typed combiner grammar. C and C++ use expressions; Fortran permits only an
/// assignment statement or a subroutine reference with an argument list.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpReductionCombiner {
    COrCppExpression(Expression),
    FortranAssignment(OmpFortranAssignment),
    FortranSubroutineCall(Expression),
}

impl OmpReductionCombiner {
    #[must_use]
    pub const fn expression(&self) -> Option<&Expression> {
        match self {
            Self::COrCppExpression(expression) | Self::FortranSubroutineCall(expression) => {
                Some(expression)
            }
            Self::FortranAssignment(_) => None,
        }
    }
}

impl fmt::Display for OmpReductionCombiner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::COrCppExpression(expression) | Self::FortranSubroutineCall(expression) => {
                expression.fmt(formatter)
            }
            Self::FortranAssignment(assignment) => assignment.fmt(formatter),
        }
    }
}

/// A parsed Fortran assignment statement used by OpenMP stylized syntax.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpFortranAssignment {
    target: OmpVariable,
    value: Expression,
}

impl OmpFortranAssignment {
    pub(crate) const fn new(target: OmpVariable, value: Expression) -> Self {
        Self { target, value }
    }

    #[must_use]
    pub const fn target(&self) -> &OmpVariable {
        &self.target
    }

    #[must_use]
    pub const fn value(&self) -> &Expression {
        &self.value
    }
}

impl fmt::Display for OmpFortranAssignment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} = {}", self.target, self.value)
    }
}

/// One recursively typed C or C++ initializer value.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpInitializerValue {
    Expression(Expression),
    Braced(OmpBracedInitializer),
}

impl fmt::Display for OmpInitializerValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expression(expression) => expression.fmt(formatter),
            Self::Braced(initializer) => initializer.fmt(formatter),
        }
    }
}

/// A delimiter-checked C or C++ braced initializer list.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpBracedInitializer {
    elements: Vec<OmpInitializerValue>,
}

impl OmpBracedInitializer {
    pub(crate) const fn new(elements: Vec<OmpInitializerValue>) -> Self {
        Self { elements }
    }

    #[must_use]
    pub fn elements(&self) -> &[OmpInitializerValue] {
        &self.elements
    }
}

impl fmt::Display for OmpBracedInitializer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("{")?;
        for (index, element) in self.elements.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            element.fmt(formatter)?;
        }
        formatter.write_str("}")
    }
}

/// Fully classified initializer expression for a declared reduction.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpReductionInitializer {
    CAssignment(OmpInitializerValue),
    CppCopy(OmpInitializerValue),
    CppDirect(Expression),
    CppList(OmpBracedInitializer),
    COrCppFunctionCall(Expression),
    FortranAssignment(OmpFortranAssignment),
    FortranSubroutineCall(Expression),
}

impl fmt::Display for OmpReductionInitializer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CAssignment(value) | Self::CppCopy(value) => {
                write!(formatter, "omp_priv = {value}")
            }
            Self::CppDirect(initializer) => initializer.fmt(formatter),
            Self::CppList(initializer) => write!(formatter, "omp_priv{initializer}"),
            Self::COrCppFunctionCall(call) | Self::FortranSubroutineCall(call) => {
                call.fmt(formatter)
            }
            Self::FortranAssignment(assignment) => assignment.fmt(formatter),
        }
    }
}

/// Fully classified inductor expression for the configured base language.
/// Fortran uses statement syntax for assignments, so it cannot be represented
/// by the ordinary host expression node used by C and C++.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpInductorExpression {
    COrCppExpression(Expression),
    FortranAssignment(OmpFortranAssignment),
    FortranSubroutineCall(Expression),
}

impl OmpInductorExpression {
    #[must_use]
    pub const fn expression(&self) -> Option<&Expression> {
        match self {
            Self::COrCppExpression(expression) | Self::FortranSubroutineCall(expression) => {
                Some(expression)
            }
            Self::FortranAssignment(_) => None,
        }
    }
}

impl fmt::Display for OmpInductorExpression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::COrCppExpression(expression) | Self::FortranSubroutineCall(expression) => {
                expression.fmt(formatter)
            }
            Self::FortranAssignment(assignment) => assignment.fmt(formatter),
        }
    }
}

/// Declare-induction identifier. OpenMP defines `+` and `*` implicitly and
/// also permits a base-language name or Fortran defined operator.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpInductionIdentifier {
    Add,
    Multiply,
    Name(OmpIdExpression),
    DefinedOperator(Identifier),
}

impl fmt::Display for OmpInductionIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => formatter.write_str("+"),
            Self::Multiply => formatter.write_str("*"),
            Self::Name(name) => name.fmt(formatter),
            Self::DefinedOperator(name) => write!(formatter, ".{name}."),
        }
    }
}

/// One declare-induction type specifier. A pair gives distinct induction
/// variable and step-expression types.
#[derive(Debug, Clone, PartialEq)]
pub enum OmpInductionTypeSpecifier {
    Same(TypeName),
    Pair { variable: TypeName, step: TypeName },
}

/// Typed `declare induction` directive argument.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpDeclareInduction {
    identifier: OmpInductionIdentifier,
    type_specifiers: Vec<OmpInductionTypeSpecifier>,
}

impl OmpDeclareInduction {
    pub(crate) fn new(
        identifier: OmpInductionIdentifier,
        type_specifiers: Vec<OmpInductionTypeSpecifier>,
    ) -> Result<Self, AstInvariantError> {
        if type_specifiers.is_empty() {
            return Err(AstInvariantError::new(
                "OpenMP declare induction requires at least one type specifier",
            ));
        }
        Ok(Self {
            identifier,
            type_specifiers,
        })
    }

    #[must_use]
    pub const fn identifier(&self) -> &OmpInductionIdentifier {
        &self.identifier
    }

    #[must_use]
    pub fn type_specifiers(&self) -> &[OmpInductionTypeSpecifier] {
        &self.type_specifiers
    }
}

impl OmpDeclareReduction {
    pub(crate) fn new(
        identifier: OmpReductionIdentifier,
        type_names: Vec<TypeName>,
        combiner: OmpReductionCombiner,
        initializer: Option<OmpReductionInitializer>,
        source_syntax: OmpDeclareReductionSyntax,
    ) -> Result<Self, AstInvariantError> {
        if type_names.is_empty() {
            return Err(AstInvariantError::new(
                "OpenMP declare reduction requires at least one type name",
            ));
        }
        Ok(Self {
            identifier,
            type_names,
            combiner,
            initializer,
            source_syntax,
        })
    }

    #[must_use]
    pub const fn identifier(&self) -> &OmpReductionIdentifier {
        &self.identifier
    }

    #[must_use]
    pub fn type_names(&self) -> &[TypeName] {
        &self.type_names
    }

    #[must_use]
    pub const fn combiner(&self) -> &OmpReductionCombiner {
        &self.combiner
    }

    #[must_use]
    pub const fn initializer(&self) -> Option<&OmpReductionInitializer> {
        self.initializer.as_ref()
    }

    #[must_use]
    pub(crate) const fn source_syntax(&self) -> OmpDeclareReductionSyntax {
        self.source_syntax
    }
}

/// Present `declare simd(proc-name)` target.
///
/// Bare `declare simd` is represented by an absent directive parameter. Keeping
/// the name required here prevents empty parentheses from becoming a second AST
/// spelling of the same bare directive.
#[derive(Debug, Clone, PartialEq)]
pub struct OmpSimdTarget {
    function: OmpIdentifier,
}

impl OmpSimdTarget {
    pub(crate) const fn new(function: OmpIdentifier) -> Self {
        Self { function }
    }

    #[must_use]
    pub const fn function(&self) -> &OmpIdentifier {
        &self.function
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OmpMapperId {
    Default,
    User(OmpIdentifier),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OmpDeclareMapper {
    identifier: Option<OmpMapperId>,
    type_name: TypeName,
    variable: Identifier,
}

impl OmpDeclareMapper {
    pub(crate) fn new(
        identifier: Option<OmpMapperId>,
        type_name: TypeName,
        variable: Identifier,
    ) -> Self {
        Self {
            identifier,
            type_name,
            variable,
        }
    }

    #[must_use]
    pub const fn identifier(&self) -> Option<&OmpMapperId> {
        self.identifier.as_ref()
    }

    #[must_use]
    pub const fn type_name(&self) -> &TypeName {
        &self.type_name
    }

    #[must_use]
    pub const fn variable(&self) -> &Identifier {
        &self.variable
    }
}

/// OpenACC directive parameter payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum AccDirectiveParameter {
    Cache(AccCacheDirective),
    Wait(AccWaitDirective),
    Routine(AccRoutineDirective),
    End(AccEndKind),
}

/// One syntactically valid OpenACC `cache` item.
///
/// Cache items must identify either one array element or one contiguous
/// subarray. A scalar designator and an array section with an explicit stride
/// cannot be represented by this type.
#[derive(Debug, Clone, PartialEq)]
pub enum AccCacheItem {
    ArrayElement(Variable),
    ContiguousSubarray(Variable),
}

impl AccCacheItem {
    pub(crate) fn new(variable: Variable) -> Result<Self, AstInvariantError> {
        let mut saw_dimension = false;
        let mut saw_section = false;
        if !acc_cache_designator_shape(variable.ast(), &mut saw_dimension, &mut saw_section) {
            return Err(AstInvariantError::new(
                "OpenACC cache item must not contain a stride or keyword subscript",
            ));
        }
        if !saw_dimension {
            return Err(AstInvariantError::new(
                "OpenACC cache item must be an array element or contiguous subarray",
            ));
        }
        if saw_section {
            Ok(Self::ContiguousSubarray(variable))
        } else {
            Ok(Self::ArrayElement(variable))
        }
    }

    #[must_use]
    pub const fn variable(&self) -> &Variable {
        match self {
            Self::ArrayElement(variable) | Self::ContiguousSubarray(variable) => variable,
        }
    }
}

impl fmt::Display for AccCacheItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.variable().fmt(formatter)
    }
}

/// `cache` directive payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccCacheDirective {
    readonly: bool,
    items: Vec<AccCacheItem>,
}

impl AccCacheDirective {
    pub(crate) fn new(readonly: bool, items: Vec<AccCacheItem>) -> Result<Self, AstInvariantError> {
        if items.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC cache directive requires at least one cache item",
            ));
        }
        Ok(Self { readonly, items })
    }

    #[must_use]
    pub const fn readonly(&self) -> bool {
        self.readonly
    }

    #[must_use]
    pub fn items(&self) -> &[AccCacheItem] {
        &self.items
    }
}

fn acc_cache_designator_shape(
    expression: &crate::host::Expr,
    saw_dimension: &mut bool,
    saw_section: &mut bool,
) -> bool {
    use crate::host::{ExprKind, FortranArgument, Subscript};

    match &expression.kind {
        ExprKind::Name(_) => true,
        ExprKind::Parenthesized(inner) | ExprKind::Member { base: inner, .. } => {
            acc_cache_designator_shape(inner, saw_dimension, saw_section)
        }
        ExprKind::Subscript { base, subscript } => {
            if !acc_cache_designator_shape(base, saw_dimension, saw_section) {
                return false;
            }
            *saw_dimension = true;
            match subscript {
                Subscript::Index(_) => true,
                Subscript::Section(section) => {
                    *saw_section = true;
                    section.stride.is_none()
                }
            }
        }
        ExprKind::FortranApply {
            designator,
            arguments,
        } => {
            if !acc_cache_designator_shape(designator, saw_dimension, saw_section) {
                return false;
            }
            for argument in arguments {
                *saw_dimension = true;
                match argument {
                    FortranArgument::Positional(_) => {}
                    FortranArgument::Section(section) => {
                        *saw_section = true;
                        if section.stride.is_some() {
                            return false;
                        }
                    }
                    FortranArgument::Keyword { .. } => return false,
                }
            }
            true
        }
        ExprKind::Literal(_)
        | ExprKind::Unary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::Conditional { .. }
        | ExprKind::Assignment { .. }
        | ExprKind::Call { .. }
        | ExprKind::Postfix { .. } => false,
    }
}

/// `wait` directive payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccWaitDirective {
    devnum: Option<Expression>,
    queues: Vec<Expression>,
}

impl AccWaitDirective {
    pub(crate) fn new(
        devnum: Option<Expression>,
        queues: Vec<Expression>,
    ) -> Result<Self, AstInvariantError> {
        if queues.is_empty() {
            return Err(AstInvariantError::new(
                "parenthesized OpenACC wait directive requires at least one queue",
            ));
        }
        Ok(Self { devnum, queues })
    }

    #[must_use]
    pub const fn devnum(&self) -> Option<&Expression> {
        self.devnum.as_ref()
    }

    #[must_use]
    pub fn queues(&self) -> &[Expression] {
        &self.queues
    }
}

/// Present `routine(name)` directive payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccRoutineDirective {
    name: Identifier,
}

impl AccRoutineDirective {
    pub(crate) const fn new(name: Identifier) -> Self {
        Self { name }
    }

    #[must_use]
    pub const fn name(&self) -> &Identifier {
        &self.name
    }
}

/// Standardized directive kinds accepted after a Fortran OpenACC `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccEndKind {
    Atomic,
    Data,
    HostData,
    Kernels,
    KernelsLoop,
    Loop,
    Parallel,
    ParallelLoop,
    Serial,
    SerialLoop,
}

impl AccEndKind {
    pub const ALL: &'static [Self] = &[
        Self::Atomic,
        Self::Data,
        Self::HostData,
        Self::Kernels,
        Self::KernelsLoop,
        Self::Loop,
        Self::Parallel,
        Self::ParallelLoop,
        Self::Serial,
        Self::SerialLoop,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Atomic => "atomic",
            Self::Data => "data",
            Self::HostData => "host_data",
            Self::Kernels => "kernels",
            Self::KernelsLoop => "kernels loop",
            Self::Loop => "loop",
            Self::Parallel => "parallel",
            Self::ParallelLoop => "parallel loop",
            Self::Serial => "serial",
            Self::SerialLoop => "serial loop",
        }
    }
}

/// OpenACC clause payloads covering the clauses that require structured data.
#[derive(Debug, Clone, PartialEq)]
pub enum AccClausePayload {
    Bare,
    Expression(Expression),
    NumGangs(Vec<Expression>),
    Tile(Vec<AccSizeExpression>),
    ItemList(Vec<ClauseItem>),
    Bind(AccBindTarget),
    Collapse(AccCollapseClause),
    Default(AccDefaultKind),
    Copy(AccCopyClause),
    Create(AccCreateClause),
    Data(AccDataClause),
    DeviceType(Vec<AccDeviceType>),
    Gang(AccGangClause),
    Worker(AccWorkerClause),
    Vector(AccVectorClause),
    Wait(AccWaitClause),
    Reduction(AccReductionClause),
}

/// One syntactically valid OpenACC `bind` target.
///
/// The specification permits a host-language name or a string literal.  An
/// arbitrary host expression cannot be represented by this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccBindTarget {
    Name(Identifier),
    StringLiteral(crate::host::StringLiteral),
}

impl AccBindTarget {
    #[must_use]
    pub const fn name(&self) -> Option<&Identifier> {
        match self {
            Self::Name(name) => Some(name),
            Self::StringLiteral(_) => None,
        }
    }

    #[must_use]
    pub const fn string_literal(&self) -> Option<&crate::host::StringLiteral> {
        match self {
            Self::StringLiteral(literal) => Some(literal),
            Self::Name(_) => None,
        }
    }
}

/// One OpenACC size argument. `*` requests an implementation-selected size
/// and is not a host-language multiplication expression.
#[derive(Debug, Clone, PartialEq)]
pub enum AccSizeExpression {
    Automatic,
    Expression(Box<Expression>),
}

impl AccSizeExpression {
    #[must_use]
    pub fn expression(&self) -> Option<&Expression> {
        match self {
            Self::Automatic => None,
            Self::Expression(expression) => Some(expression.as_ref()),
        }
    }

    #[must_use]
    pub const fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic)
    }
}

/// OpenACC loop-collapse payload, including the 3.4 `force` modifier.
#[derive(Debug, Clone, PartialEq)]
pub struct AccCollapseClause {
    force: bool,
    count: Expression,
}

impl AccCollapseClause {
    pub(crate) const fn new(force: bool, count: Expression) -> Self {
        Self { force, count }
    }

    #[must_use]
    pub const fn force(&self) -> bool {
        self.force
    }

    #[must_use]
    pub const fn count(&self) -> &Expression {
        &self.count
    }
}

/// Copy-like clause payload (`copy`, `pcopy`, `present_or_copy`).
#[derive(Debug, Clone, PartialEq)]
pub struct AccCopyClause {
    kind: AccCopyKind,
    modifiers: Vec<AccDataModifier>,
    variables: Vec<ClauseItem>,
}

impl AccCopyClause {
    pub(crate) fn new(
        kind: AccCopyKind,
        modifiers: Vec<AccDataModifier>,
        variables: Vec<ClauseItem>,
    ) -> Result<Self, AstInvariantError> {
        if variables.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC copy clause requires at least one variable",
            ));
        }
        if has_duplicate_acc_data_modifiers(&modifiers)
            || modifiers.iter().any(|modifier| !kind.allows(*modifier))
        {
            return Err(AstInvariantError::new(
                "OpenACC copy clause has duplicate or incompatible modifiers",
            ));
        }
        Ok(Self {
            kind,
            modifiers,
            variables,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> AccCopyKind {
        self.kind
    }

    #[must_use]
    pub fn modifiers(&self) -> &[AccDataModifier] {
        &self.modifiers
    }

    #[must_use]
    pub fn variables(&self) -> &[ClauseItem] {
        &self.variables
    }
}

/// Create-like clause payload (`create`, `pcreate`, `present_or_create`).
#[derive(Debug, Clone, PartialEq)]
pub struct AccCreateClause {
    kind: AccCreateKind,
    modifiers: Vec<AccDataModifier>,
    variables: Vec<ClauseItem>,
}

impl AccCreateClause {
    pub(crate) fn new(
        kind: AccCreateKind,
        modifiers: Vec<AccDataModifier>,
        variables: Vec<ClauseItem>,
    ) -> Result<Self, AstInvariantError> {
        if variables.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC create clause requires at least one variable",
            ));
        }
        if has_duplicate_acc_data_modifiers(&modifiers)
            || modifiers.iter().any(|modifier| {
                !matches!(modifier, AccDataModifier::Zero | AccDataModifier::Capture)
            })
        {
            return Err(AstInvariantError::new(
                "OpenACC create clause has duplicate or incompatible modifiers",
            ));
        }
        Ok(Self {
            kind,
            modifiers,
            variables,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> AccCreateKind {
        self.kind
    }

    #[must_use]
    pub fn modifiers(&self) -> &[AccDataModifier] {
        &self.modifiers
    }

    #[must_use]
    pub fn variables(&self) -> &[ClauseItem] {
        &self.variables
    }
}

/// Generic data movement clauses (`attach`, `detach`, `link`, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct AccDataClause {
    kind: AccDataKind,
    variables: Vec<ClauseItem>,
}

impl AccDataClause {
    pub(crate) fn new(
        kind: AccDataKind,
        variables: Vec<ClauseItem>,
    ) -> Result<Self, AstInvariantError> {
        if variables.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC data clause requires at least one variable",
            ));
        }
        Ok(Self { kind, variables })
    }

    #[must_use]
    pub const fn kind(&self) -> AccDataKind {
        self.kind
    }

    #[must_use]
    pub fn variables(&self) -> &[ClauseItem] {
        &self.variables
    }
}

/// OpenACC data clause modifiers (shared across copy/copyin/copyout/create).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccDataModifier {
    Always,
    AlwaysIn,
    AlwaysOut,
    Capture,
    Readonly,
    Zero,
}

/// OpenACC device types for `device_type(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccDeviceType {
    Host,
    Wildcard,
    Multicore,
    Default,
    Named(Identifier),
}

/// Ordered OpenACC gang arguments. Positional `num`, explicit `num`, `dim`,
/// and `static` remain distinct typed forms.
#[derive(Debug, Clone, PartialEq)]
pub struct AccGangClause {
    arguments: Vec<AccGangArgument>,
}

impl AccGangClause {
    pub(crate) fn new(arguments: Vec<AccGangArgument>) -> Result<Self, AstInvariantError> {
        let mut saw_num = false;
        let mut saw_dim = false;
        let mut saw_static = false;
        for argument in &arguments {
            let duplicate = match argument {
                AccGangArgument::Positional(_) | AccGangArgument::Num(_) => {
                    let duplicate = saw_num;
                    saw_num = true;
                    duplicate
                }
                AccGangArgument::Dim(_) => {
                    let duplicate = saw_dim;
                    saw_dim = true;
                    duplicate
                }
                AccGangArgument::Static(_) => {
                    let duplicate = saw_static;
                    saw_static = true;
                    duplicate
                }
            };
            if duplicate {
                return Err(AstInvariantError::new(
                    "OpenACC gang clause contains a duplicate argument kind",
                ));
            }
        }
        Ok(Self { arguments })
    }

    #[must_use]
    pub fn arguments(&self) -> &[AccGangArgument] {
        &self.arguments
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccGangArgument {
    Positional(Box<Expression>),
    Num(Box<Expression>),
    Dim(Box<Expression>),
    Static(AccSizeExpression),
}

/// Exact OpenACC `worker` payload shape.
///
/// Parentheses contain exactly one integer expression, optionally introduced
/// by the explicit `num:` spelling.  Keeping the bare form as its own
/// variant prevents empty and multi-expression states.
#[derive(Debug, Clone, PartialEq)]
pub enum AccWorkerClause {
    Bare,
    Num(Box<Expression>),
    Expression(Box<Expression>),
}

impl AccWorkerClause {
    #[must_use]
    pub const fn modifier(&self) -> Option<AccWorkerModifier> {
        match self {
            Self::Bare => None,
            Self::Num(_) => Some(AccWorkerModifier::Num),
            Self::Expression(_) => Some(AccWorkerModifier::ExprOnly),
        }
    }

    #[must_use]
    pub fn expression(&self) -> Option<&Expression> {
        match self {
            Self::Bare => None,
            Self::Num(expression) | Self::Expression(expression) => Some(expression.as_ref()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccWorkerModifier {
    Num,
    ExprOnly,
}

/// Exact OpenACC `vector` payload shape.
///
/// Parentheses contain exactly one integer expression, optionally introduced
/// by the explicit `length:` spelling.  Keeping the bare form as its own
/// variant prevents empty and multi-expression states.
#[derive(Debug, Clone, PartialEq)]
pub enum AccVectorClause {
    Bare,
    Length(Box<Expression>),
    Expression(Box<Expression>),
}

impl AccVectorClause {
    #[must_use]
    pub const fn modifier(&self) -> Option<AccVectorModifier> {
        match self {
            Self::Bare => None,
            Self::Length(_) => Some(AccVectorModifier::Length),
            Self::Expression(_) => Some(AccVectorModifier::ExprOnly),
        }
    }

    #[must_use]
    pub fn expression(&self) -> Option<&Expression> {
        match self {
            Self::Bare => None,
            Self::Length(expression) | Self::Expression(expression) => Some(expression.as_ref()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccVectorModifier {
    Length,
    ExprOnly,
}

/// Wait clause payload (mirrors directive form).
#[derive(Debug, Clone, PartialEq)]
pub struct AccWaitClause {
    devnum: Option<Expression>,
    queues: Vec<Expression>,
}

impl AccWaitClause {
    pub(crate) fn new(
        devnum: Option<Expression>,
        queues: Vec<Expression>,
    ) -> Result<Self, AstInvariantError> {
        if devnum.is_some() && queues.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC wait devnum modifier requires at least one queue",
            ));
        }
        Ok(Self { devnum, queues })
    }

    #[must_use]
    pub const fn devnum(&self) -> Option<&Expression> {
        self.devnum.as_ref()
    }

    #[must_use]
    pub fn queues(&self) -> &[Expression] {
        &self.queues
    }
}

/// OpenACC reduction clause payload.
#[derive(Debug, Clone, PartialEq)]
pub struct AccReductionClause {
    operator: AccReductionOperator,
    variables: Vec<ClauseItem>,
}

impl AccReductionClause {
    pub(crate) fn new(
        operator: AccReductionOperator,
        variables: Vec<ClauseItem>,
    ) -> Result<Self, AstInvariantError> {
        if variables.is_empty() {
            return Err(AstInvariantError::new(
                "OpenACC reduction clause requires at least one variable",
            ));
        }
        if variables
            .iter()
            .any(|item| matches!(item, ClauseItem::FortranCommonBlock(_)))
        {
            return Err(AstInvariantError::new(
                "OpenACC reduction clauses do not accept common block names",
            ));
        }
        Ok(Self {
            operator,
            variables,
        })
    }

    #[must_use]
    pub const fn operator(&self) -> &AccReductionOperator {
        &self.operator
    }

    #[must_use]
    pub fn variables(&self) -> &[ClauseItem] {
        &self.variables
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccReductionOperator {
    Add,
    Mul,
    Max,
    Min,
    BitAnd,
    BitOr,
    BitXor,
    LogAnd,
    LogOr,
    FortAnd,
    FortOr,
    FortEqv,
    FortNeqv,
    FortIand,
    FortIor,
    FortIeor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccDefaultKind {
    None,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccCopyKind {
    Copy,
    CopyIn,
    CopyOut,
}

impl AccCopyKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            AccCopyKind::Copy => "copy",
            AccCopyKind::CopyIn => "copyin",
            AccCopyKind::CopyOut => "copyout",
        }
    }

    const fn allows(self, modifier: AccDataModifier) -> bool {
        match self {
            Self::Copy => matches!(
                modifier,
                AccDataModifier::Always
                    | AccDataModifier::AlwaysIn
                    | AccDataModifier::AlwaysOut
                    | AccDataModifier::Capture
            ),
            Self::CopyIn => matches!(
                modifier,
                AccDataModifier::Always
                    | AccDataModifier::AlwaysIn
                    | AccDataModifier::Capture
                    | AccDataModifier::Readonly
            ),
            Self::CopyOut => matches!(
                modifier,
                AccDataModifier::Always
                    | AccDataModifier::AlwaysOut
                    | AccDataModifier::Capture
                    | AccDataModifier::Zero
            ),
        }
    }
}

fn has_duplicate_acc_data_modifiers(modifiers: &[AccDataModifier]) -> bool {
    modifiers
        .iter()
        .enumerate()
        .any(|(index, modifier)| modifiers[..index].contains(modifier))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccCreateKind {
    Create,
}

impl AccCreateKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            AccCreateKind::Create => "create",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccDataKind {
    Attach,
    Detach,
    UseDevice,
    Link,
    DeviceResident,
    Device,
    Delete,
}

fn omp_parameter_matches_kind(
    kind: OmpDirectiveKind,
    parameter: Option<&OmpDirectiveParameter>,
) -> bool {
    let Some(parameter) = parameter else {
        return !matches!(
            kind,
            OmpDirectiveKind::Allocate
                | OmpDirectiveKind::Threadprivate
                | OmpDirectiveKind::Groupprivate
                | OmpDirectiveKind::DeclareMapper
                | OmpDirectiveKind::DeclareVariant
                | OmpDirectiveKind::Cancel
                | OmpDirectiveKind::CancellationPoint
                | OmpDirectiveKind::DeclareReduction
                | OmpDirectiveKind::DeclareInduction
        );
    };
    match parameter {
        OmpDirectiveParameter::AllocateList(items) => {
            !items.is_empty() && kind == OmpDirectiveKind::Allocate
        }
        OmpDirectiveParameter::ThreadprivateList(items) => {
            !items.is_empty() && kind == OmpDirectiveKind::Threadprivate
        }
        OmpDirectiveParameter::GroupprivateList(items) => {
            !items.is_empty() && kind == OmpDirectiveKind::Groupprivate
        }
        OmpDirectiveParameter::DeclareTargetList(items) => {
            !items.is_empty() && kind == OmpDirectiveKind::DeclareTarget
        }
        OmpDirectiveParameter::DeclareMapper(_) => kind == OmpDirectiveKind::DeclareMapper,
        OmpDirectiveParameter::DeclareVariant(_) => kind == OmpDirectiveKind::DeclareVariant,
        OmpDirectiveParameter::Depobj(_) => kind == OmpDirectiveKind::Depobj,
        OmpDirectiveParameter::Construct(_) => matches!(
            kind,
            OmpDirectiveKind::Cancel | OmpDirectiveKind::CancellationPoint
        ),
        OmpDirectiveParameter::CriticalSection(_) => matches!(
            kind,
            OmpDirectiveKind::Critical | OmpDirectiveKind::EndCritical
        ),
        OmpDirectiveParameter::FlushList(items) => {
            !items.is_empty() && kind == OmpDirectiveKind::Flush
        }
        OmpDirectiveParameter::DeclareReduction(_) => kind == OmpDirectiveKind::DeclareReduction,
        OmpDirectiveParameter::DeclareInduction(_) => kind == OmpDirectiveKind::DeclareInduction,
        OmpDirectiveParameter::DeclareSimd(_) => kind == OmpDirectiveKind::DeclareSimd,
    }
}

fn acc_parameter_matches_kind(
    kind: AccDirectiveKind,
    parameter: Option<&AccDirectiveParameter>,
) -> bool {
    match parameter {
        None => !matches!(kind, AccDirectiveKind::Cache | AccDirectiveKind::End),
        Some(AccDirectiveParameter::Cache(_)) => kind == AccDirectiveKind::Cache,
        Some(AccDirectiveParameter::Wait(_)) => kind == AccDirectiveKind::Wait,
        Some(AccDirectiveParameter::Routine(_)) => kind == AccDirectiveKind::Routine,
        Some(AccDirectiveParameter::End(_)) => kind == AccDirectiveKind::End,
    }
}

fn omp_directive_alias_matches_kind(
    alias: Option<OmpDirectiveSourceAlias>,
    kind: OmpDirectiveKind,
) -> bool {
    match alias {
        None => true,
        Some(OmpDirectiveSourceAlias::FortranCompact) => kind.as_str().contains(' '),
        Some(OmpDirectiveSourceAlias::OpenMp60Underscore) => matches!(
            kind,
            OmpDirectiveKind::CancellationPoint
                | OmpDirectiveKind::DeclareInduction
                | OmpDirectiveKind::DeclareMapper
                | OmpDirectiveKind::DeclareReduction
                | OmpDirectiveKind::DeclareSimd
                | OmpDirectiveKind::DeclareTarget
                | OmpDirectiveKind::DeclareVariant
                | OmpDirectiveKind::BeginDeclareTarget
                | OmpDirectiveKind::EndDeclareTarget
                | OmpDirectiveKind::BeginDeclareVariant
                | OmpDirectiveKind::EndDeclareVariant
                | OmpDirectiveKind::TaskIteration
                | OmpDirectiveKind::TargetData
                | OmpDirectiveKind::EndTargetData
                | OmpDirectiveKind::TargetEnterData
                | OmpDirectiveKind::TargetExitData
                | OmpDirectiveKind::TargetUpdate
        ),
    }
}

fn omp_alias_matches_kind(alias: Option<OmpClauseSourceAlias>, kind: OmpClauseKind) -> bool {
    match alias {
        None => true,
        Some(
            OmpClauseSourceAlias::DependSource
            | OmpClauseSourceAlias::DependSourceCurrent
            | OmpClauseSourceAlias::DependSink
            | OmpClauseSourceAlias::DependSinkPreviousCurrent,
        ) => kind == OmpClauseKind::Doacross,
        Some(OmpClauseSourceAlias::MetadirectiveDefault) => kind == OmpClauseKind::Otherwise,
        Some(OmpClauseSourceAlias::DeclareTargetTo) => kind == OmpClauseKind::Enter,
        Some(OmpClauseSourceAlias::ProcBindMaster) => kind == OmpClauseKind::ProcBind,
    }
}

fn acc_alias_matches_kind(alias: Option<AccClauseSourceAlias>, kind: AccClauseKind) -> bool {
    match alias {
        None => true,
        Some(AccClauseSourceAlias::PCopy | AccClauseSourceAlias::PresentOrCopy) => {
            kind == AccClauseKind::Copy
        }
        Some(AccClauseSourceAlias::PCopyIn | AccClauseSourceAlias::PresentOrCopyIn) => {
            kind == AccClauseKind::CopyIn
        }
        Some(AccClauseSourceAlias::PCopyOut | AccClauseSourceAlias::PresentOrCopyOut) => {
            kind == AccClauseKind::CopyOut
        }
        Some(AccClauseSourceAlias::PCreate | AccClauseSourceAlias::PresentOrCreate) => {
            kind == AccClauseKind::Create
        }
        Some(AccClauseSourceAlias::UpdateHost) => kind == AccClauseKind::SelfClause,
    }
}

fn preference_specification_has_required_contents(
    specification: &crate::ir::OmpPreferenceSpecification,
) -> bool {
    match specification {
        crate::ir::OmpPreferenceSpecification::ForeignRuntime(_) => true,
        crate::ir::OmpPreferenceSpecification::Selectors(selectors) => {
            !selectors.is_empty()
                && selectors.iter().all(|selector| match selector {
                    crate::ir::OmpPreferenceSelector::ForeignRuntime(_) => true,
                    crate::ir::OmpPreferenceSelector::Attributes(attributes) => {
                        !attributes.is_empty()
                    }
                })
                && selectors
                    .iter()
                    .filter(|selector| {
                        matches!(
                            selector,
                            crate::ir::OmpPreferenceSelector::ForeignRuntime(_)
                        )
                    })
                    .count()
                    <= 1
        }
    }
}

fn omp_payload_has_required_contents(payload: &OmpClausePayload) -> bool {
    use crate::ir::{ClauseData, DoacrossType, OmpDependence, OmpDoacrossIteration};

    match payload {
        ClauseData::ItemList(items)
        | ClauseData::Private { items }
        | ClauseData::Shared { items }
        | ClauseData::UseDevicePtr { items }
        | ClauseData::UseDeviceAddr { items }
        | ClauseData::IsDevicePtr { items }
        | ClauseData::HasDeviceAddr { items }
        | ClauseData::Copyin { items }
        | ClauseData::Copyprivate { items } => !items.is_empty(),
        ClauseData::Sizes { sizes } => !sizes.is_empty(),
        ClauseData::Permutation { positions } => positions.len() >= 2,
        ClauseData::Counts { counts } => {
            !counts.is_empty()
                && counts
                    .iter()
                    .filter(|count| matches!(count, crate::ir::OmpCount::Fill))
                    .count()
                    == 1
        }
        ClauseData::Uniform { parameters } => !parameters.is_empty(),
        ClauseData::Enter { items, .. } => !items.is_empty(),
        ClauseData::To { locators, .. } | ClauseData::From { locators, .. } => !locators.is_empty(),
        ClauseData::Map { locators, .. } | ClauseData::Affinity { locators, .. } => {
            !locators.is_empty()
        }
        ClauseData::Scan { items, .. }
        | ClauseData::Firstprivate { items, .. }
        | ClauseData::Lastprivate { items, .. }
        | ClauseData::Reduction { items, .. }
        | ClauseData::Linear { items, .. }
        | ClauseData::Aligned { items, .. }
        | ClauseData::Allocate { items, .. } => !items.is_empty(),
        ClauseData::Absent { directives } | ClauseData::Contains { directives } => {
            !directives.is_empty()
        }
        ClauseData::AdjustArgs { parameters, .. } => !parameters.is_empty(),
        ClauseData::AppendArgs { operations } => !operations.is_empty(),
        ClauseData::Apply {
            applied_directives, ..
        } => !applied_directives.is_empty(),
        ClauseData::Induction { items, .. } => !items.is_empty(),
        ClauseData::InitInterop {
            interop_types,
            preferences,
            ..
        } => {
            !interop_types.is_empty()
                && interop_types
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == interop_types.len()
                && preferences
                    .iter()
                    .all(preference_specification_has_required_contents)
        }
        ClauseData::Depend { dependence, .. } => match dependence {
            OmpDependence::Locators { locators, .. } => !locators.is_empty(),
            OmpDependence::Depobjs { objects } => !objects.is_empty(),
        },
        ClauseData::Doacross { kind, iteration } => match (kind, iteration) {
            (DoacrossType::Source, OmpDoacrossIteration::Current)
            | (DoacrossType::Sink, OmpDoacrossIteration::PreviousCurrent) => true,
            (DoacrossType::Sink, OmpDoacrossIteration::Vector(vector)) => !vector.is_empty(),
            _ => false,
        },
        ClauseData::UsesAllocators { allocators } => !allocators.is_empty(),
        ClauseData::NumThreads { nthreads, .. } => !nthreads.is_empty(),
        ClauseData::Bare
        | ClauseData::Nowait { .. }
        | ClauseData::Nogroup { .. }
        | ClauseData::Align { .. }
        | ClauseData::Destroy { .. }
        | ClauseData::Final { .. }
        | ClauseData::GraphId { .. }
        | ClauseData::Hint { .. }
        | ClauseData::Holds { .. }
        | ClauseData::Message { .. }
        | ClauseData::Nocontext { .. }
        | ClauseData::Novariants { .. }
        | ClauseData::Use { .. }
        | ClauseData::InitComplete { .. }
        | ClauseData::Branch { .. }
        | ClauseData::Full { .. }
        | ClauseData::Partial { .. }
        | ClauseData::Mergeable { .. }
        | ClauseData::Untied { .. }
        | ClauseData::Simd { .. }
        | ClauseData::Threads { .. }
        | ClauseData::Assumption { .. }
        | ClauseData::Indirect { .. }
        | ClauseData::Replayable { .. }
        | ClauseData::Safesync { .. }
        | ClauseData::Transparent { .. }
        | ClauseData::Threadset(_)
        | ClauseData::Memscope(_)
        | ClauseData::Looprange { .. }
        | ClauseData::GraphReset { .. }
        | ClauseData::Collector { .. }
        | ClauseData::Inductor { .. }
        | ClauseData::Default { .. }
        | ClauseData::Defaultmap { .. }
        | ClauseData::Priority { .. }
        | ClauseData::Detach { .. }
        | ClauseData::Schedule { .. }
        | ClauseData::Collapse { .. }
        | ClauseData::Ordered { .. }
        | ClauseData::Safelen { .. }
        | ClauseData::Simdlen { .. }
        | ClauseData::If { .. }
        | ClauseData::ProcBind(_)
        | ClauseData::Bind(_)
        | ClauseData::Device { .. }
        | ClauseData::DeviceType(_)
        | ClauseData::At(_)
        | ClauseData::Severity(_)
        | ClauseData::InitDepobj { .. }
        | ClauseData::Fail { .. }
        | ClauseData::MemoryOrder { .. }
        | ClauseData::AtomicOperation { .. }
        | ClauseData::ExtendedAtomic { .. }
        | ClauseData::Order { .. }
        | ClauseData::NumTeams { .. }
        | ClauseData::ThreadLimit { .. }
        | ClauseData::Allocator { .. }
        | ClauseData::DistSchedule { .. }
        | ClauseData::Grainsize { .. }
        | ClauseData::NumTasks { .. }
        | ClauseData::Filter { .. }
        | ClauseData::DepobjUpdate { .. }
        | ClauseData::MetadirectiveSelector { .. } => true,
        ClauseData::Requirement {
            requirement:
                crate::ir::RequireModifier::AtomicDefaultMemOrder(_)
                | crate::ir::RequireModifier::ExtImplementationDefinedRequirement(_),
            required,
        } => required.is_none(),
        ClauseData::Requirement { .. } => true,
    }
}

fn acc_payload_has_required_contents(payload: &AccClausePayload) -> bool {
    match payload {
        AccClausePayload::NumGangs(expressions) => !expressions.is_empty(),
        AccClausePayload::Tile(sizes) => !sizes.is_empty(),
        AccClausePayload::ItemList(items) => !items.is_empty(),
        AccClausePayload::DeviceType(device_types) => !device_types.is_empty(),
        AccClausePayload::Bare
        | AccClausePayload::Expression(_)
        | AccClausePayload::Bind(_)
        | AccClausePayload::Collapse(_)
        | AccClausePayload::Default(_)
        | AccClausePayload::Copy(_)
        | AccClausePayload::Create(_)
        | AccClausePayload::Data(_)
        | AccClausePayload::Gang(_)
        | AccClausePayload::Worker(_)
        | AccClausePayload::Vector(_)
        | AccClausePayload::Wait(_)
        | AccClausePayload::Reduction(_) => true,
    }
}

fn omp_payload_matches_kind(kind: OmpClauseKind, payload: &OmpClausePayload) -> bool {
    use crate::ir::{AtomicOp, ClauseData, ExtendedAtomicKind, MemoryOrder, ScanClauseMode};
    use OmpClauseKind as K;

    match payload {
        ClauseData::Bare => matches!(
            kind,
            K::Parallel | K::Sections | K::For | K::Do | K::Taskgroup
        ),
        ClauseData::Nowait { .. } => kind == K::Nowait,
        ClauseData::Nogroup { .. } => kind == K::Nogroup,
        ClauseData::Branch { .. } => matches!(kind, K::Inbranch | K::Notinbranch),
        ClauseData::Full { .. } => kind == K::Full,
        ClauseData::Partial { .. } => kind == K::Partial,
        ClauseData::Mergeable { .. } => kind == K::Mergeable,
        ClauseData::Untied { .. } => kind == K::Untied,
        ClauseData::Simd { .. } => kind == K::Simd,
        ClauseData::Threads { .. } => kind == K::Threads,
        ClauseData::Assumption { .. } => matches!(
            kind,
            K::NoOpenmp | K::NoOpenmpConstructs | K::NoOpenmpRoutines | K::NoParallelism
        ),
        ClauseData::Indirect { .. } => kind == K::Indirect,
        ClauseData::Replayable { .. } => kind == K::Replayable,
        ClauseData::Safesync { .. } => kind == K::Safesync,
        ClauseData::Transparent { .. } => kind == K::Transparent,
        ClauseData::Align { .. } => kind == K::Align,
        ClauseData::Destroy { .. } => kind == K::Destroy,
        ClauseData::Final { .. } => kind == K::Final,
        ClauseData::GraphId { .. } => kind == K::GraphId,
        ClauseData::Hint { .. } => kind == K::Hint,
        ClauseData::Holds { .. } => kind == K::Holds,
        ClauseData::Message { .. } => kind == K::Message,
        ClauseData::Nocontext { .. } => kind == K::Nocontext,
        ClauseData::Novariants { .. } => kind == K::Novariants,
        ClauseData::Threadset(_) => kind == K::Threadset,
        ClauseData::Memscope(_) => kind == K::Memscope,
        ClauseData::Looprange { .. } => kind == K::Looprange,
        ClauseData::GraphReset { .. } => kind == K::GraphReset,
        ClauseData::ItemList(_) => matches!(kind, K::Interop | K::Link | K::Local | K::Nontemporal),
        ClauseData::Sizes { .. } => kind == K::Sizes,
        ClauseData::Permutation { .. } => kind == K::Permutation,
        ClauseData::Counts { .. } => kind == K::Counts,
        ClauseData::Uniform { .. } => kind == K::Uniform,
        ClauseData::Use { .. } => kind == K::Use,
        ClauseData::Enter { .. } => kind == K::Enter,
        ClauseData::To { .. } => kind == K::To,
        ClauseData::From { .. } => kind == K::From,
        ClauseData::Scan { mode, .. } => matches!(
            (kind, mode),
            (K::Inclusive, ScanClauseMode::Inclusive) | (K::Exclusive, ScanClauseMode::Exclusive)
        ),
        ClauseData::InitComplete { .. } => kind == K::InitComplete,
        ClauseData::Absent { .. } => kind == K::Absent,
        ClauseData::Contains { .. } => kind == K::Contains,
        ClauseData::AdjustArgs { .. } => kind == K::AdjustArgs,
        ClauseData::AppendArgs { .. } => kind == K::AppendArgs,
        ClauseData::Collector { .. } => kind == K::Collector,
        ClauseData::Inductor { .. } => kind == K::Inductor,
        ClauseData::Apply { .. } => kind == K::Apply,
        ClauseData::Induction { .. } => kind == K::Induction,
        ClauseData::Private { .. } => kind == K::Private,
        ClauseData::Firstprivate { .. } => kind == K::Firstprivate,
        ClauseData::Lastprivate { .. } => kind == K::Lastprivate,
        ClauseData::Shared { .. } => kind == K::Shared,
        ClauseData::Default { .. } => kind == K::Default,
        ClauseData::Defaultmap { .. } => kind == K::Defaultmap,
        ClauseData::Reduction { .. } => {
            matches!(kind, K::Reduction | K::InReduction | K::TaskReduction)
        }
        ClauseData::Map { .. } => kind == K::Map,
        ClauseData::UseDevicePtr { .. } => kind == K::UseDevicePtr,
        ClauseData::UseDeviceAddr { .. } => kind == K::UseDeviceAddr,
        ClauseData::IsDevicePtr { .. } => kind == K::IsDevicePtr,
        ClauseData::HasDeviceAddr { .. } => kind == K::HasDeviceAddr,
        ClauseData::Depend { .. } => kind == K::Depend,
        ClauseData::Doacross { .. } => kind == K::Doacross,
        ClauseData::Priority { .. } => kind == K::Priority,
        ClauseData::Detach { .. } => kind == K::Detach,
        ClauseData::Affinity { .. } => kind == K::Affinity,
        ClauseData::Schedule { .. } => kind == K::Schedule,
        ClauseData::Collapse { .. } => kind == K::Collapse,
        ClauseData::Ordered { .. } => kind == K::Ordered,
        ClauseData::Linear { .. } => kind == K::Linear,
        ClauseData::Aligned { .. } => kind == K::Aligned,
        ClauseData::Safelen { .. } => kind == K::Safelen,
        ClauseData::Simdlen { .. } => kind == K::Simdlen,
        ClauseData::If { .. } => kind == K::If,
        ClauseData::ProcBind(_) => kind == K::ProcBind,
        ClauseData::Bind(_) => kind == K::Bind,
        ClauseData::NumThreads { .. } => kind == K::NumThreads,
        ClauseData::Device { .. } => kind == K::Device,
        ClauseData::DeviceType(_) => kind == K::DeviceType,
        ClauseData::At(_) => kind == K::At,
        ClauseData::Severity(_) => kind == K::Severity,
        ClauseData::InitInterop { .. } | ClauseData::InitDepobj { .. } => kind == K::Init,
        ClauseData::Fail { .. } => kind == K::Fail,
        ClauseData::MemoryOrder { order, .. } => matches!(
            (kind, order),
            (K::SeqCst, MemoryOrder::SeqCst)
                | (K::AcqRel, MemoryOrder::AcqRel)
                | (K::Acquire, MemoryOrder::Acquire)
                | (K::Release, MemoryOrder::Release)
                | (K::Relaxed, MemoryOrder::Relaxed)
        ),
        ClauseData::AtomicOperation { op, .. } => matches!(
            (kind, op),
            (K::Read, AtomicOp::Read) | (K::Write, AtomicOp::Write) | (K::Update, AtomicOp::Update)
        ),
        ClauseData::ExtendedAtomic {
            kind: extended_kind,
            ..
        } => matches!(
            (kind, extended_kind),
            (K::Capture, ExtendedAtomicKind::Capture)
                | (K::Compare, ExtendedAtomicKind::Compare)
                | (K::Weak, ExtendedAtomicKind::Weak)
        ),
        ClauseData::Order { .. } => kind == K::Order,
        ClauseData::NumTeams { .. } => kind == K::NumTeams,
        ClauseData::ThreadLimit { .. } => kind == K::ThreadLimit,
        ClauseData::Allocate { .. } => kind == K::Allocate,
        ClauseData::Allocator { .. } => kind == K::Allocator,
        ClauseData::Copyin { .. } => kind == K::CopyIn,
        ClauseData::Copyprivate { .. } => kind == K::Copyprivate,
        ClauseData::DistSchedule { .. } => kind == K::DistSchedule,
        ClauseData::Grainsize { .. } => kind == K::Grainsize,
        ClauseData::NumTasks { .. } => kind == K::NumTasks,
        ClauseData::Filter { .. } => kind == K::Filter,
        ClauseData::UsesAllocators { .. } => kind == K::UsesAllocators,
        ClauseData::Requirement { requirement, .. } => matches!(
            (kind, requirement),
            (
                K::AtomicDefaultMemOrder,
                crate::ir::RequireModifier::AtomicDefaultMemOrder(_)
            ) | (
                K::ReverseOffload,
                crate::ir::RequireModifier::ReverseOffload
            ) | (
                K::UnifiedAddress,
                crate::ir::RequireModifier::UnifiedAddress
            ) | (
                K::UnifiedSharedMemory,
                crate::ir::RequireModifier::UnifiedSharedMemory
            ) | (
                K::DynamicAllocators,
                crate::ir::RequireModifier::DynamicAllocators
            ) | (K::SelfMaps, crate::ir::RequireModifier::SelfMaps)
                | (
                    K::DeviceSafesync,
                    crate::ir::RequireModifier::DeviceSafesync
                )
                | (
                    K::ExtImplementationDefinedRequirement,
                    crate::ir::RequireModifier::ExtImplementationDefinedRequirement(_)
                )
        ),
        ClauseData::DepobjUpdate { .. } => kind == K::DepobjUpdate,
        ClauseData::MetadirectiveSelector { .. } => {
            matches!(kind, K::When | K::Match | K::Otherwise)
        }
    }
}

fn acc_payload_matches_kind(kind: AccClauseKind, payload: &AccClausePayload) -> bool {
    use AccClauseKind as K;
    match payload {
        AccClausePayload::Bare => matches!(
            kind,
            K::Async
                | K::Auto
                | K::Capture
                | K::Finalize
                | K::IfPresent
                | K::Independent
                | K::NoHost
                | K::Read
                | K::Seq
                | K::Update
                | K::Write
                | K::SelfClause
        ),
        AccClausePayload::Expression(_) => matches!(
            kind,
            K::Async
                | K::DefaultAsync
                | K::DeviceNum
                | K::If
                | K::NumWorkers
                | K::VectorLength
                | K::SelfClause
        ),
        AccClausePayload::Bind(_) => kind == K::Bind,
        AccClausePayload::NumGangs(_) => kind == K::NumGangs,
        AccClausePayload::Tile(_) => kind == K::Tile,
        AccClausePayload::ItemList(_) => matches!(
            kind,
            K::DevicePtr | K::Firstprivate | K::NoCreate | K::Present | K::Private | K::SelfClause
        ),
        AccClausePayload::Collapse(_) => kind == K::Collapse,
        AccClausePayload::Default(_) => kind == K::Default,
        AccClausePayload::Copy(copy) => matches!(
            (kind, copy.kind),
            (K::Copy, AccCopyKind::Copy)
                | (K::CopyIn, AccCopyKind::CopyIn)
                | (K::CopyOut, AccCopyKind::CopyOut)
        ),
        AccClausePayload::Create(_) => kind == K::Create,
        AccClausePayload::Data(data) => matches!(
            (kind, data.kind),
            (K::Attach, AccDataKind::Attach)
                | (K::Detach, AccDataKind::Detach)
                | (K::UseDevice, AccDataKind::UseDevice)
                | (K::Link, AccDataKind::Link)
                | (K::DeviceResident, AccDataKind::DeviceResident)
                | (K::Device, AccDataKind::Device)
                | (K::Delete, AccDataKind::Delete)
        ),
        AccClausePayload::DeviceType(_) => kind == K::DeviceType,
        AccClausePayload::Gang(_) => kind == K::Gang,
        AccClausePayload::Worker(_) => kind == K::Worker,
        AccClausePayload::Vector(_) => kind == K::Vector,
        AccClausePayload::Wait(_) => kind == K::Wait,
        AccClausePayload::Reduction(_) => kind == K::Reduction,
    }
}

// -------------------------------------------------------------------------
// Enum generation helpers (data sourced from parser tables)
// -------------------------------------------------------------------------

macro_rules! define_omp_directive_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum OmpDirectiveKind {
            $( $variant, )+
        }

        impl OmpDirectiveKind {
            pub const ALL: &'static [OmpDirectiveKind] = &[ $( OmpDirectiveKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( OmpDirectiveKind::$variant => $name, )+
                }
            }
        }

        impl From<OmpDirectiveKind> for DirectiveName {
            fn from(kind: OmpDirectiveKind) -> Self {
                match kind {
                    $( OmpDirectiveKind::$variant => DirectiveName::$variant, )+
                }
            }
        }

        impl TryFrom<DirectiveName> for OmpDirectiveKind {
            type Error = DirectiveName;

            fn try_from(value: DirectiveName) -> Result<Self, DirectiveName> {
                match value {
                    DirectiveName::BeginDeclareTargetUnderscore => {
                        Ok(OmpDirectiveKind::BeginDeclareTarget)
                    }
                    DirectiveName::DeclareTargetUnderscore => Ok(OmpDirectiveKind::DeclareTarget),
                    DirectiveName::EndDeclareTargetUnderscore => {
                        Ok(OmpDirectiveKind::EndDeclareTarget)
                    }
                    DirectiveName::TargetDataUnderscore => Ok(OmpDirectiveKind::TargetData),
                    DirectiveName::ParallelDoCompact => Ok(OmpDirectiveKind::ParallelDo),
                    DirectiveName::EndDoCompact => Ok(OmpDirectiveKind::EndDo),
                    DirectiveName::EndDoSimdCompact => Ok(OmpDirectiveKind::EndDoSimd),
                    $( DirectiveName::$variant => Ok(OmpDirectiveKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

macro_rules! define_omp_clause_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum OmpClauseKind {
            $( $variant, )+
        }

        impl OmpClauseKind {
            pub const ALL: &'static [OmpClauseKind] = &[ $( OmpClauseKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( OmpClauseKind::$variant => $name, )+
                }
            }
        }

        impl From<OmpClauseKind> for ClauseName {
            fn from(kind: OmpClauseKind) -> Self {
                match kind {
                    $( OmpClauseKind::$variant => ClauseName::$variant, )+
                }
            }
        }

        impl TryFrom<ClauseName> for OmpClauseKind {
            type Error = ClauseName;

            fn try_from(value: ClauseName) -> Result<Self, ClauseName> {
                match value {
                    $( ClauseName::$variant => Ok(OmpClauseKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

macro_rules! define_acc_directive_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum AccDirectiveKind {
            $( $variant, )+
        }

        impl AccDirectiveKind {
            pub const ALL: &'static [AccDirectiveKind] = &[ $( AccDirectiveKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( AccDirectiveKind::$variant => $name, )+
                }
            }
        }

        impl From<AccDirectiveKind> for DirectiveName {
            fn from(kind: AccDirectiveKind) -> Self {
                match kind {
                    $( AccDirectiveKind::$variant => DirectiveName::$variant, )+
                }
            }
        }

        impl TryFrom<DirectiveName> for AccDirectiveKind {
            type Error = DirectiveName;

            fn try_from(value: DirectiveName) -> Result<Self, DirectiveName> {
                match value {
                    $( DirectiveName::$variant => Ok(AccDirectiveKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

macro_rules! define_acc_clause_kind {
    ($( $variant:ident => $name:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum AccClauseKind {
            $( $variant, )+
        }

        impl AccClauseKind {
            pub const ALL: &'static [AccClauseKind] = &[ $( AccClauseKind::$variant, )+ ];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $( AccClauseKind::$variant => $name, )+
                }
            }
        }

        impl From<AccClauseKind> for ClauseName {
            fn from(kind: AccClauseKind) -> Self {
                match kind {
                    $( AccClauseKind::$variant => ClauseName::$variant, )+
                }
            }
        }

        impl TryFrom<ClauseName> for AccClauseKind {
            type Error = ClauseName;

            fn try_from(value: ClauseName) -> Result<Self, ClauseName> {
                match value {
                    $( ClauseName::$variant => Ok(AccClauseKind::$variant), )+
                    other => Err(other),
                }
            }
        }
    };
}

// --- Data generated
define_omp_directive_kind! {
    Allocate => "allocate",
    Allocators => "allocators",
    Assume => "assume",
    Assumes => "assumes",
    Atomic => "atomic",
    Barrier => "barrier",
    BeginAssumes => "begin assumes",
    BeginDeclareTarget => "begin declare target",
    BeginDeclareVariant => "begin declare variant",
    Cancel => "cancel",
    CancellationPoint => "cancellation point",
    Critical => "critical",
    DeclareInduction => "declare induction",
    DeclareMapper => "declare mapper",
    DeclareReduction => "declare reduction",
    DeclareSimd => "declare simd",
    DeclareTarget => "declare target",
    DeclareVariant => "declare variant",
    Depobj => "depobj",
    Dispatch => "dispatch",
    Distribute => "distribute",
    DistributeParallelDo => "distribute parallel do",
    DistributeParallelDoSimd => "distribute parallel do simd",
    DistributeParallelFor => "distribute parallel for",
    DistributeParallelForSimd => "distribute parallel for simd",
    DistributeParallelLoop => "distribute parallel loop",
    DistributeParallelLoopSimd => "distribute parallel loop simd",
    DistributeSimd => "distribute simd",
    Do => "do",
    DoSimd => "do simd",
    EndAssume => "end assume",
    EndAssumes => "end assumes",
    EndAllocators => "end allocators",
    EndDeclareTarget => "end declare target",
    EndDeclareVariant => "end declare variant",
    EndDispatch => "end dispatch",
    EndParallel => "end parallel",
    EndDo => "end do",
    EndSimd => "end simd",
    EndSections => "end sections",
    EndSingle => "end single",
    EndWorkshare => "end workshare",
    EndOrdered => "end ordered",
    EndLoop => "end loop",
    EndDistribute => "end distribute",
    EndTeams => "end teams",
    EndTaskloop => "end taskloop",
    EndTask => "end task",
    EndTaskgroup => "end taskgroup",
    EndMaster => "end master",
    EndMasked => "end masked",
    EndUnroll => "end unroll",
    EndCritical => "end critical",
    EndAtomic => "end atomic",
    EndParallelDo => "end parallel do",
    EndParallelSections => "end parallel sections",
    EndParallelWorkshare => "end parallel workshare",
    EndParallelMaster => "end parallel master",
    EndParallelSingle => "end parallel single",
    EndParallelMasterTaskloop => "end parallel master taskloop",
    EndParallelMasterTaskloopSimd => "end parallel master taskloop simd",
    EndDoSimd => "end do simd",
    EndParallelDoSimd => "end parallel do simd",
    EndDistributeSimd => "end distribute simd",
    EndDistributeParallelDo => "end distribute parallel do",
    EndDistributeParallelDoSimd => "end distribute parallel do simd",
    EndTargetParallel => "end target parallel",
    EndTargetParallelDo => "end target parallel do",
    EndTargetParallelDoSimd => "end target parallel do simd",
    EndTargetParallelLoop => "end target parallel loop",
    EndTargetSimd => "end target simd",
    EndTargetTeams => "end target teams",
    EndTargetTeamsDistribute => "end target teams distribute",
    EndTargetTeamsDistributeParallelDo => "end target teams distribute parallel do",
    EndTargetTeamsDistributeParallelDoSimd => "end target teams distribute parallel do simd",
    EndTargetTeamsDistributeSimd => "end target teams distribute simd",
    EndTargetTeamsLoop => "end target teams loop",
    EndTargetTeamsWorkdistribute => "end target teams workdistribute",
    EndTeamsDistribute => "end teams distribute",
    EndTeamsDistributeParallelDo => "end teams distribute parallel do",
    EndTeamsDistributeParallelDoSimd => "end teams distribute parallel do simd",
    EndTeamsDistributeSimd => "end teams distribute simd",
    EndTeamsLoop => "end teams loop",
    EndTaskloopSimd => "end taskloop simd",
    EndMasterTaskloop => "end master taskloop",
    EndMasterTaskloopSimd => "end master taskloop simd",
    EndMaskedTaskloop => "end masked taskloop",
    EndMaskedTaskloopSimd => "end masked taskloop simd",
    EndParallelMasked => "end parallel masked",
    EndParallelMaskedTaskloop => "end parallel masked taskloop",
    EndParallelMaskedTaskloopSimd => "end parallel masked taskloop simd",
    EndParallelLoop => "end parallel loop",
    EndTargetLoop => "end target loop",
    EndTile => "end tile",
    Error => "error",
    Flush => "flush",
    Fuse => "fuse",
    Groupprivate => "groupprivate",
    For => "for",
    ForSimd => "for simd",
    Interchange => "interchange",
    Interop => "interop",
    Loop => "loop",
    Reverse => "reverse",
    Masked => "masked",
    MaskedTaskloop => "masked taskloop",
    MaskedTaskloopSimd => "masked taskloop simd",
    Master => "master",
    MasterTaskloop => "master taskloop",
    MasterTaskloopSimd => "master taskloop simd",
    Metadirective => "metadirective",
    BeginMetadirective => "begin metadirective",
    // Fortran block-ending form of metadirective
    EndMetadirective => "end metadirective",
    Nothing => "nothing",
    Ordered => "ordered",
    Parallel => "parallel",
    ParallelDo => "parallel do",
    ParallelDoSimd => "parallel do simd",
    ParallelFor => "parallel for",
    ParallelForSimd => "parallel for simd",
    ParallelLoop => "parallel loop",
    ParallelLoopSimd => "parallel loop simd",
    ParallelMasked => "parallel masked",
    ParallelMaskedTaskloop => "parallel masked taskloop",
    ParallelMaskedTaskloopSimd => "parallel masked taskloop simd",
    ParallelMaster => "parallel master",
    ParallelMasterTaskloop => "parallel master taskloop",
    ParallelMasterTaskloopSimd => "parallel master taskloop simd",
    ParallelSections => "parallel sections",
    ParallelSingle => "parallel single",
    ParallelWorkshare => "parallel workshare",
    Requires => "requires",
    Scope => "scope",
    EndScope => "end scope",
    Scan => "scan",
    Section => "section",
    Sections => "sections",
    Simd => "simd",
    Single => "single",
    Split => "split",
    Stripe => "stripe",
    Target => "target",
    TargetData => "target data",
    TargetEnterData => "target enter data",
    TargetExitData => "target exit data",
    EndTarget => "end target",
    EndTargetData => "end target data",
    TargetLoop => "target loop",
    TargetLoopSimd => "target loop simd",
    TargetParallel => "target parallel",
    TargetParallelDo => "target parallel do",
    TargetParallelDoSimd => "target parallel do simd",
    TargetParallelFor => "target parallel for",
    TargetParallelForSimd => "target parallel for simd",
    TargetParallelLoop => "target parallel loop",
    TargetParallelLoopSimd => "target parallel loop simd",
    TargetSimd => "target simd",
    TargetTeams => "target teams",
    TargetTeamsDistribute => "target teams distribute",
    TargetTeamsDistributeParallelDo => "target teams distribute parallel do",
    TargetTeamsDistributeParallelDoSimd => "target teams distribute parallel do simd",
    TargetTeamsDistributeParallelFor => "target teams distribute parallel for",
    TargetTeamsDistributeParallelForSimd => "target teams distribute parallel for simd",
    TargetTeamsDistributeParallelLoop => "target teams distribute parallel loop",
    TargetTeamsDistributeParallelLoopSimd => "target teams distribute parallel loop simd",
    TargetTeamsDistributeSimd => "target teams distribute simd",
    TargetTeamsLoop => "target teams loop",
    TargetTeamsLoopSimd => "target teams loop simd",
    TargetTeamsWorkdistribute => "target teams workdistribute",
    TargetUpdate => "target update",
    Task => "task",
    TaskIteration => "task iteration",
    Taskgroup => "taskgroup",
    Taskgraph => "taskgraph",
    Taskloop => "taskloop",
    TaskloopSimd => "taskloop simd",
    Taskwait => "taskwait",
    Taskyield => "taskyield",
    Teams => "teams",
    TeamsDistribute => "teams distribute",
    TeamsDistributeParallelDo => "teams distribute parallel do",
    TeamsDistributeParallelDoSimd => "teams distribute parallel do simd",
    TeamsDistributeParallelFor => "teams distribute parallel for",
    TeamsDistributeParallelForSimd => "teams distribute parallel for simd",
    TeamsDistributeParallelLoop => "teams distribute parallel loop",
    TeamsDistributeParallelLoopSimd => "teams distribute parallel loop simd",
    TeamsDistributeSimd => "teams distribute simd",
    TeamsLoop => "teams loop",
    TeamsLoopSimd => "teams loop simd",
    Threadprivate => "threadprivate",
    Tile => "tile",
    Unroll => "unroll",
    Workdistribute => "workdistribute",
    Workshare => "workshare",
}

define_omp_clause_kind! {
    Absent => "absent",
    AcqRel => "acq_rel",
    Acquire => "acquire",
    AdjustArgs => "adjust_args",
    Affinity => "affinity",
    Align => "align",
    Aligned => "aligned",
    Allocate => "allocate",
    Allocator => "allocator",
    AppendArgs => "append_args",
    Apply => "apply",
    At => "at",
    AtomicDefaultMemOrder => "atomic_default_mem_order",
    Bind => "bind",
    Capture => "capture",
    Collapse => "collapse",
    Collector => "collector",
    Combiner => "combiner",
    Compare => "compare",
    Contains => "contains",
    CopyIn => "copyin",
    Copyprivate => "copyprivate",
    Parallel => "parallel",
    Sections => "sections",
    For => "for",
    Do => "do",
    Taskgroup => "taskgroup",
    Counts => "counts",
    Default => "default",
    Defaultmap => "defaultmap",
    Depend => "depend",
    DepobjUpdate => "depobj_update",
    Destroy => "destroy",
    Detach => "detach",
    Device => "device",
    DeviceSafesync => "device_safesync",
    DeviceType => "device_type",
    DistSchedule => "dist_schedule",
    Doacross => "doacross",
    DynamicAllocators => "dynamic_allocators",
    ExtImplementationDefinedRequirement => "ext_implementation_defined_requirement",
    Enter => "enter",
    Exclusive => "exclusive",
    Fail => "fail",
    Final => "final",
    Filter => "filter",
    Firstprivate => "firstprivate",
    From => "from",
    Full => "full",
    Grainsize => "grainsize",
    GraphId => "graph_id",
    GraphReset => "graph_reset",
    HasDeviceAddr => "has_device_addr",
    Hint => "hint",
    Holds => "holds",
    If => "if",
    InReduction => "in_reduction",
    Induction => "induction",
    Inductor => "inductor",
    Inbranch => "inbranch",
    Inclusive => "inclusive",
    Init => "init",
    InitComplete => "init_complete",
    Initializer => "initializer",
    Indirect => "indirect",
    IsDevicePtr => "is_device_ptr",
    Lastprivate => "lastprivate",
    Linear => "linear",
    Link => "link",
    Local => "local",
    Looprange => "looprange",
    Map => "map",
    Match => "match",
    Message => "message",
    Memscope => "memscope",
    Mergeable => "mergeable",
    Nocontext => "nocontext",
    Nogroup => "nogroup",
    NoOpenmp => "no_openmp",
    NoOpenmpConstructs => "no_openmp_constructs",
    NoOpenmpRoutines => "no_openmp_routines",
    NoParallelism => "no_parallelism",
    Nontemporal => "nontemporal",
    Notinbranch => "notinbranch",
    Novariants => "novariants",
    Interop => "interop",
    Nowait => "nowait",
    NumTasks => "num_tasks",
    NumTeams => "num_teams",
    NumThreads => "num_threads",
    Order => "order",
    Ordered => "ordered",
    Otherwise => "otherwise",
    Partial => "partial",
    Permutation => "permutation",
    Priority => "priority",
    Private => "private",
    ProcBind => "proc_bind",
    Read => "read",
    Reduction => "reduction",
    Release => "release",
    Relaxed => "relaxed",
    Replayable => "replayable",
    ReverseOffload => "reverse_offload",
    Safelen => "safelen",
    Safesync => "safesync",
    Schedule => "schedule",
    SelfMaps => "self_maps",
    SeqCst => "seq_cst",
    Severity => "severity",
    Shared => "shared",
    Simd => "simd",
    Simdlen => "simdlen",
    Sizes => "sizes",
    TaskReduction => "task_reduction",
    ThreadLimit => "thread_limit",
    Threads => "threads",
    Threadset => "threadset",
    To => "to",
    Transparent => "transparent",
    UnifiedAddress => "unified_address",
    UnifiedSharedMemory => "unified_shared_memory",
    Uniform => "uniform",
    Untied => "untied",
    Update => "update",
    Use => "use",
    UseDeviceAddr => "use_device_addr",
    UseDevicePtr => "use_device_ptr",
    UsesAllocators => "uses_allocators",
    Weak => "weak",
    When => "when",
    Write => "write",
}

define_acc_directive_kind! {
    Atomic => "atomic",
    Cache => "cache",
    Data => "data",
    Declare => "declare",
    End => "end",
    EnterData => "enter data",
    ExitData => "exit data",
    HostData => "host_data",
    Init => "init",
    Kernels => "kernels",
    KernelsLoop => "kernels loop",
    Loop => "loop",
    Parallel => "parallel",
    ParallelLoop => "parallel loop",
    Routine => "routine",
    Serial => "serial",
    SerialLoop => "serial loop",
    Set => "set",
    Shutdown => "shutdown",
    Update => "update",
    Wait => "wait",
}

define_acc_clause_kind! {
    Async => "async",
    Attach => "attach",
    Auto => "auto",
    Bind => "bind",
    Capture => "capture",
    Collapse => "collapse",
    Copy => "copy",
    CopyIn => "copyin",
    CopyOut => "copyout",
    Create => "create",
    Default => "default",
    DefaultAsync => "default_async",
    Delete => "delete",
    Detach => "detach",
    Device => "device",
    DeviceNum => "device_num",
    DeviceResident => "device_resident",
    DeviceType => "device_type",
    DevicePtr => "deviceptr",
    Finalize => "finalize",
    Firstprivate => "firstprivate",
    Gang => "gang",
    If => "if",
    IfPresent => "if_present",
    Independent => "independent",
    Link => "link",
    NoCreate => "no_create",
    NoHost => "nohost",
    NumGangs => "num_gangs",
    NumWorkers => "num_workers",
    Present => "present",
    Private => "private",
    Reduction => "reduction",
    Read => "read",
    SelfClause => "self",
    Seq => "seq",
    Tile => "tile",
    Update => "update",
    UseDevice => "use_device",
    Vector => "vector",
    VectorLength => "vector_length",
    Wait => "wait",
    Worker => "worker",
    Write => "write",
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omp_directive_conversion_round_trip() {
        for kind in OmpDirectiveKind::ALL {
            let dn: DirectiveName = (*kind).into();
            let back = OmpDirectiveKind::try_from(dn.clone()).expect("should convert");
            assert_eq!(*kind, back);
        }
    }

    #[test]
    fn acc_clause_conversion_round_trip() {
        for kind in AccClauseKind::ALL {
            let cn: ClauseName = (*kind).into();
            let back = AccClauseKind::try_from(cn.clone()).expect("should convert");
            assert_eq!(*kind, back);
        }
    }

    #[test]
    fn clause_constructors_reject_empty_required_payloads() {
        assert!(
            OmpClause::new(
                OmpClauseKind::Nontemporal,
                ClauseData::ItemList(Vec::new()),
                None,
                None,
                Span::entire("nontemporal"),
            )
            .is_err()
        );
        assert!(
            OmpClause::new(
                OmpClauseKind::Apply,
                ClauseData::Apply {
                    loop_modifier: None,
                    applied_directives: Vec::new(),
                },
                None,
                None,
                Span::entire("apply"),
            )
            .is_err()
        );
        assert!(
            AccClause::new(
                AccClauseKind::Private,
                AccClausePayload::ItemList(Vec::new()),
                None,
                Span::entire("private"),
            )
            .is_err()
        );
    }
}
