//! Enum-based AST scaffolding shared between the parser, IR, and compat layer.
//!
//! This module introduces language-specific directive and clause kind
//! wrappers so we can stop sharing a single enum between OpenMP and OpenACC.
//! The actual directive payload structures will be filled out in later steps;
//! for now this module focuses on the strongly typed identifiers and the
//! normalization configuration knobs described in `COMPAT_AST_SPEC.md`.

use crate::parser::directive_kind::DirectiveName;
use crate::parser::ClauseName;

/// Language identifier used throughout the enum-based AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoupLanguage {
    OpenMp,
    OpenAcc,
}

/// Clause normalization strategy (mirrors ompparser/accparser behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseNormalizationMode {
    /// Keep clauses exactly as written (no merging).
    Disabled,
    /// Merge compatible clauses by concatenating their variable lists.
    MergeVariableLists,
    /// Match the upstream ompparser/accparser defaults.
    ParserParity,
}

impl Default for ClauseNormalizationMode {
    fn default() -> Self {
        ClauseNormalizationMode::ParserParity
    }
}

/// Strongly typed OpenMP directive identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OmpDirectiveKind(pub DirectiveName);

impl OmpDirectiveKind {
    /// Attempt to construct an OpenMP directive kind from the parser enum.
    /// Fails if the name belongs to the OpenACC subset.
    pub fn new(name: DirectiveName) -> Result<Self, DirectiveName> {
        if is_openacc_directive(&name) {
            Err(name)
        } else {
            Ok(Self(name))
        }
    }

    /// Access the underlying parser enum variant.
    pub fn as_directive_name(&self) -> DirectiveName {
        self.0.clone()
    }
}

/// Strongly typed OpenACC directive identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccDirectiveKind(pub DirectiveName);

impl AccDirectiveKind {
    pub fn new(name: DirectiveName) -> Result<Self, DirectiveName> {
        if is_openacc_directive(&name) {
            Ok(Self(name))
        } else {
            Err(name)
        }
    }

    pub fn as_directive_name(&self) -> DirectiveName {
        self.0.clone()
    }
}

/// Strongly typed OpenMP clause identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OmpClauseKind(pub ClauseName);

impl OmpClauseKind {
    pub fn new(name: ClauseName) -> Result<Self, ClauseName> {
        if is_openacc_clause(&name) {
            Err(name)
        } else {
            Ok(Self(name))
        }
    }

    pub fn as_clause_name(self) -> ClauseName {
        self.0.clone()
    }
}

/// Strongly typed OpenACC clause identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccClauseKind(pub ClauseName);

impl AccClauseKind {
    pub fn new(name: ClauseName) -> Result<Self, ClauseName> {
        if is_openacc_clause(&name) {
            Ok(Self(name))
        } else {
            Err(name)
        }
    }

    pub fn as_clause_name(self) -> ClauseName {
        self.0.clone()
    }
}

/// OpenACC directives supported by the compatibility layer.
fn is_openacc_directive(name: &DirectiveName) -> bool {
    use DirectiveName::*;
    matches!(
        name,
        Atomic
            | Cache
            | Data
            | Declare
            | Do
            | DoSimd
            | End
            | EnterData
            | ExitData
            | For
            | ForSimd
            | HostData
            | Init
            | Kernels
            | KernelsLoop
            | Loop
            | Parallel
            | ParallelDo
            | ParallelDoSimd
            | ParallelFor
            | ParallelForSimd
            | ParallelLoop
            | Routine
            | Serial
            | SerialLoop
            | Set
            | Shutdown
            | Update
            | Wait
    )
}

/// Clause names that belong to the OpenACC subset.
fn is_openacc_clause(name: &ClauseName) -> bool {
    use ClauseName::*;
    matches!(
        name,
        Copy | CopyIn
            | CopyOut
            | Async
            | Wait
            | NumGangs
            | NumWorkers
            | VectorLength
            | Gang
            | Worker
            | Vector
            | Seq
            | Independent
            | Auto
            | DeviceType
            | Bind
            | DefaultAsync
            | Link
            | NoCreate
            | NoHost
            | Read
            | SelfClause
            | Tile
            | UseDevice
            | Attach
            | Detach
            | Finalize
            | IfPresent
            | Capture
            | Write
            | Update
            | Delete
            | Device
            | DevicePtr
            | DeviceNum
            | DeviceResident
            | Host
            | Present
            | Create
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::directive_kind::DirectiveName;

    #[test]
    fn rejects_openacc_directives_for_omp_kind() {
        let err = OmpDirectiveKind::new(DirectiveName::Parallel)
            .expect_err("parallel is OpenACC in this context");
        assert_eq!(err, DirectiveName::Parallel);
    }

    #[test]
    fn accepts_openacc_directive_wrapper() {
        let acc = AccDirectiveKind::new(DirectiveName::Parallel)
            .expect("parallel should map to OpenACC kind");
        assert_eq!(acc.as_directive_name(), DirectiveName::Parallel);
    }

    #[test]
    fn openmp_clause_wrapper_rejects_acc_clause() {
        let err = OmpClauseKind::new(ClauseName::Copy)
            .expect_err("copy belongs to the OpenACC namespace");
        assert_eq!(err, ClauseName::Copy);
    }

    #[test]
    fn acc_clause_wrapper_accepts_acc_clause() {
        let clause = AccClauseKind::new(ClauseName::CopyIn).unwrap();
        assert_eq!(clause.as_clause_name(), ClauseName::CopyIn);
    }
}
