//! Directive and host-language version configuration.

use std::error::Error;
use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, Sub};
use std::str::FromStr;

mod sealed {
    pub trait Sealed {}
}

/// A directive specification family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Dialect {
    OpenMp,
    OpenAcc,
}

impl fmt::Display for Dialect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OpenMp => "OpenMP",
            Self::OpenAcc => "OpenACC",
        })
    }
}

/// Behavior for selecting directive specification versions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VersionPolicy<V> {
    /// Accept syntax standardized by any supported version.
    Any,
    /// Accept only syntax standardized by one exact version.
    Exact(V),
}

impl<V: DirectiveVersion> VersionPolicy<V> {
    /// Returns the versions permitted by this policy.
    #[must_use]
    pub fn allowed_versions(self) -> VersionSet<V> {
        match self {
            Self::Any => VersionSet::all(),
            Self::Exact(version) => VersionSet::singleton(version),
        }
    }
}

/// A version that can be represented in a typed [`VersionSet`].
///
/// This trait is sealed because version ordinals form part of the set's stable
/// representation.
pub trait DirectiveVersion:
    sealed::Sealed + Copy + Eq + Ord + fmt::Debug + fmt::Display + 'static
{
    /// All supported versions in chronological order.
    const ALL: &'static [Self];

    /// The specification family to which this version belongs.
    const DIALECT: Dialect;

    /// Returns the zero-based chronological ordinal.
    fn ordinal(self) -> u8;
}

/// A supported OpenMP specification version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum OpenMpVersion {
    V1_0 = 0,
    V1_1 = 1,
    V2_0 = 2,
    V2_5 = 3,
    V3_0 = 4,
    V3_1 = 5,
    V4_0 = 6,
    V4_5 = 7,
    V5_0 = 8,
    V5_1 = 9,
    V5_2 = 10,
    V6_0 = 11,
}

impl OpenMpVersion {
    pub const ALL: &'static [Self] = &[
        Self::V1_0,
        Self::V1_1,
        Self::V2_0,
        Self::V2_5,
        Self::V3_0,
        Self::V3_1,
        Self::V4_0,
        Self::V4_5,
        Self::V5_0,
        Self::V5_1,
        Self::V5_2,
        Self::V6_0,
    ];

    /// Returns the canonical specification version string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1_0 => "1.0",
            Self::V1_1 => "1.1",
            Self::V2_0 => "2.0",
            Self::V2_5 => "2.5",
            Self::V3_0 => "3.0",
            Self::V3_1 => "3.1",
            Self::V4_0 => "4.0",
            Self::V4_5 => "4.5",
            Self::V5_0 => "5.0",
            Self::V5_1 => "5.1",
            Self::V5_2 => "5.2",
            Self::V6_0 => "6.0",
        }
    }
}

impl sealed::Sealed for OpenMpVersion {}

impl DirectiveVersion for OpenMpVersion {
    const ALL: &'static [Self] = Self::ALL;
    const DIALECT: Dialect = Dialect::OpenMp;

    fn ordinal(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for OpenMpVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OpenMpVersion {
    type Err = VersionParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "1.0" => Ok(Self::V1_0),
            "1.1" => Ok(Self::V1_1),
            "2.0" => Ok(Self::V2_0),
            "2.5" => Ok(Self::V2_5),
            "3.0" => Ok(Self::V3_0),
            "3.1" => Ok(Self::V3_1),
            "4.0" => Ok(Self::V4_0),
            "4.5" => Ok(Self::V4_5),
            "5.0" => Ok(Self::V5_0),
            "5.1" => Ok(Self::V5_1),
            "5.2" => Ok(Self::V5_2),
            "6.0" => Ok(Self::V6_0),
            _ => Err(VersionParseError::new(Dialect::OpenMp, input)),
        }
    }
}

/// A supported OpenACC specification version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum OpenAccVersion {
    V1_0 = 0,
    V2_0 = 1,
    V2_5 = 2,
    V2_6 = 3,
    V2_7 = 4,
    V3_0 = 5,
    V3_1 = 6,
    V3_2 = 7,
    V3_3 = 8,
    V3_4 = 9,
}

impl OpenAccVersion {
    pub const ALL: &'static [Self] = &[
        Self::V1_0,
        Self::V2_0,
        Self::V2_5,
        Self::V2_6,
        Self::V2_7,
        Self::V3_0,
        Self::V3_1,
        Self::V3_2,
        Self::V3_3,
        Self::V3_4,
    ];

    /// Returns the canonical specification version string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1_0 => "1.0",
            Self::V2_0 => "2.0",
            Self::V2_5 => "2.5",
            Self::V2_6 => "2.6",
            Self::V2_7 => "2.7",
            Self::V3_0 => "3.0",
            Self::V3_1 => "3.1",
            Self::V3_2 => "3.2",
            Self::V3_3 => "3.3",
            Self::V3_4 => "3.4",
        }
    }
}

impl sealed::Sealed for OpenAccVersion {}

impl DirectiveVersion for OpenAccVersion {
    const ALL: &'static [Self] = Self::ALL;
    const DIALECT: Dialect = Dialect::OpenAcc;

    fn ordinal(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for OpenAccVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OpenAccVersion {
    type Err = VersionParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "1.0" => Ok(Self::V1_0),
            "2.0" => Ok(Self::V2_0),
            "2.5" => Ok(Self::V2_5),
            "2.6" => Ok(Self::V2_6),
            "2.7" => Ok(Self::V2_7),
            "3.0" => Ok(Self::V3_0),
            "3.1" => Ok(Self::V3_1),
            "3.2" => Ok(Self::V3_2),
            "3.3" => Ok(Self::V3_3),
            "3.4" => Ok(Self::V3_4),
            _ => Err(VersionParseError::new(Dialect::OpenAcc, input)),
        }
    }
}

/// A compact set containing versions from exactly one specification family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VersionSet<V: DirectiveVersion> {
    bits: u64,
    version: PhantomData<fn() -> V>,
}

impl<V: DirectiveVersion> VersionSet<V> {
    /// Returns the empty set.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            bits: 0,
            version: PhantomData,
        }
    }

    /// Returns the set of all supported versions in this family.
    #[must_use]
    pub fn all() -> Self {
        V::ALL.iter().copied().collect()
    }

    /// Returns a set containing only `version`.
    #[must_use]
    pub fn singleton(version: V) -> Self {
        Self {
            bits: bit_for(version),
            version: PhantomData,
        }
    }

    /// Reports whether the set has no members.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Returns the number of versions in the set.
    #[must_use]
    pub const fn len(self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Reports whether `version` belongs to this set.
    #[must_use]
    pub fn contains(self, version: V) -> bool {
        self.bits & bit_for(version) != 0
    }

    /// Inserts `version`, returning whether it was newly inserted.
    pub fn insert(&mut self, version: V) -> bool {
        let bit = bit_for(version);
        let was_absent = self.bits & bit == 0;
        self.bits |= bit;
        was_absent
    }

    /// Removes `version`, returning whether it was present.
    pub fn remove(&mut self, version: V) -> bool {
        let bit = bit_for(version);
        let was_present = self.bits & bit != 0;
        self.bits &= !bit;
        was_present
    }

    /// Returns the union of two sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
            version: PhantomData,
        }
    }

    /// Returns the intersection of two sets.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
            version: PhantomData,
        }
    }

    /// Returns versions in `self` that are absent from `other`.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self {
            bits: self.bits & !other.bits,
            version: PhantomData,
        }
    }

    /// Reports whether every version in `self` also belongs to `other`.
    #[must_use]
    pub const fn is_subset(self, other: Self) -> bool {
        self.bits & !other.bits == 0
    }

    /// Returns versions in chronological order.
    pub fn iter(self) -> VersionIter<V> {
        VersionIter {
            set: self,
            front: 0,
            back: V::ALL.len(),
        }
    }

    /// Returns the earliest version in the set.
    #[must_use]
    pub fn earliest(self) -> Option<V> {
        self.iter().next()
    }

    /// Returns the latest version in the set.
    #[must_use]
    pub fn latest(self) -> Option<V> {
        self.iter().next_back()
    }
}

impl<V: DirectiveVersion> Default for VersionSet<V> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<V: DirectiveVersion> FromIterator<V> for VersionSet<V> {
    fn from_iter<T: IntoIterator<Item = V>>(versions: T) -> Self {
        let mut set = Self::empty();
        for version in versions {
            set.insert(version);
        }
        set
    }
}

impl<V: DirectiveVersion> IntoIterator for VersionSet<V> {
    type Item = V;
    type IntoIter = VersionIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Chronological iterator over a [`VersionSet`].
#[derive(Clone, Debug)]
pub struct VersionIter<V: DirectiveVersion> {
    set: VersionSet<V>,
    front: usize,
    back: usize,
}

impl<V: DirectiveVersion> Iterator for VersionIter<V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        while self.front < self.back {
            let version = V::ALL[self.front];
            self.front += 1;
            if self.set.contains(version) {
                return Some(version);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl<V: DirectiveVersion> DoubleEndedIterator for VersionIter<V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.front < self.back {
            self.back -= 1;
            let version = V::ALL[self.back];
            if self.set.contains(version) {
                return Some(version);
            }
        }
        None
    }
}

impl<V: DirectiveVersion> ExactSizeIterator for VersionIter<V> {
    fn len(&self) -> usize {
        V::ALL[self.front..self.back]
            .iter()
            .filter(|version| self.set.contains(**version))
            .count()
    }
}

impl<V: DirectiveVersion> std::iter::FusedIterator for VersionIter<V> {}

impl<V: DirectiveVersion> BitOr for VersionSet<V> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl<V: DirectiveVersion> BitAnd for VersionSet<V> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}

impl<V: DirectiveVersion> Sub for VersionSet<V> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.difference(rhs)
    }
}

impl<V: DirectiveVersion> fmt::Display for VersionSet<V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("{")?;
        for (index, version) in self.iter().enumerate() {
            if index != 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{version}")?;
        }
        formatter.write_str("}")
    }
}

fn bit_for<V: DirectiveVersion>(version: V) -> u64 {
    // `DirectiveVersion` is sealed and every supported ordinal is below 64.
    1_u64 << version.ordinal()
}

/// Failure to parse an exact directive specification version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionParseError {
    dialect: Dialect,
    input: Box<str>,
}

impl VersionParseError {
    fn new(dialect: Dialect, input: &str) -> Self {
        Self {
            dialect,
            input: input.into(),
        }
    }

    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for VersionParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported {} specification version {:?}",
            self.dialect, self.input
        )
    }
}

impl Error for VersionParseError {}

/// A base host language.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HostLanguage {
    C,
    Cpp,
    Fortran,
}

impl fmt::Display for HostLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Fortran => "Fortran",
        })
    }
}

/// Supported C language standards.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CStandard {
    C89,
    C99,
    C11,
    C18,
    C23,
}

/// Supported C++ language standards.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CppStandard {
    Cpp98,
    Cpp11,
    Cpp14,
    Cpp17,
    Cpp20,
    Cpp23,
}

/// Supported Fortran language standards.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FortranStandard {
    Fortran77,
    Fortran90,
    Fortran95,
    Fortran2003,
    Fortran2008,
    Fortran2018,
    Fortran2023,
}

/// A host language together with the standard used to parse embedded syntax.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HostLanguageProfile {
    C(CStandard),
    Cpp(CppStandard),
    Fortran(FortranStandard),
}

impl HostLanguageProfile {
    /// Latest fully modeled standard for a base language.
    ///
    /// This is an explicit convenience for callers that select only a base
    /// language. Version-sensitive parser entry points retain the concrete
    /// profile supplied by their caller instead of invoking this function.
    #[must_use]
    pub const fn latest(language: HostLanguage) -> Self {
        match language {
            HostLanguage::C => Self::C(CStandard::C23),
            HostLanguage::Cpp => Self::Cpp(CppStandard::Cpp23),
            HostLanguage::Fortran => Self::Fortran(FortranStandard::Fortran2023),
        }
    }

    /// Returns the profile's base language.
    #[must_use]
    pub const fn language(self) -> HostLanguage {
        match self {
            Self::C(_) => HostLanguage::C,
            Self::Cpp(_) => HostLanguage::Cpp,
            Self::Fortran(_) => HostLanguage::Fortran,
        }
    }

    /// Checks that `source_form` is meaningful for this host language.
    pub fn validate_source_form(self, source_form: SourceForm) -> Result<(), HostProfileError> {
        let compatible = matches!(
            (self, source_form),
            (Self::C(_) | Self::Cpp(_), SourceForm::Pragma)
                | (
                    Self::Fortran(_),
                    SourceForm::FortranFree | SourceForm::FortranFixed
                )
        );

        if compatible {
            Ok(())
        } else {
            Err(HostProfileError::IncompatibleSourceForm {
                language: self.language(),
                source_form,
            })
        }
    }
}

/// Physical representation of a directive in source text.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceForm {
    Pragma,
    FortranFree,
    FortranFixed,
}

impl fmt::Display for SourceForm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pragma => "pragma",
            Self::FortranFree => "Fortran free form",
            Self::FortranFixed => "Fortran fixed form",
        })
    }
}

/// Invalid pairing of host language and source form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostProfileError {
    IncompatibleSourceForm {
        language: HostLanguage,
        source_form: SourceForm,
    },
}

impl fmt::Display for HostProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleSourceForm {
                language,
                source_form,
            } => write!(
                formatter,
                "{source_form} source is incompatible with host language {language}"
            ),
        }
    }
}

impl Error for HostProfileError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openmp_versions_are_chronological_and_round_trip() {
        for (ordinal, version) in OpenMpVersion::ALL.iter().copied().enumerate() {
            assert_eq!(usize::from(version.ordinal()), ordinal);
            assert_eq!(version.as_str().parse(), Ok(version));
        }
        assert_eq!(OpenMpVersion::ALL.last(), Some(&OpenMpVersion::V6_0));
    }

    #[test]
    fn openacc_versions_are_chronological_and_round_trip() {
        for (ordinal, version) in OpenAccVersion::ALL.iter().copied().enumerate() {
            assert_eq!(usize::from(version.ordinal()), ordinal);
            assert_eq!(version.as_str().parse(), Ok(version));
        }
        assert_eq!(OpenAccVersion::ALL.last(), Some(&OpenAccVersion::V3_4));
    }

    #[test]
    fn invalid_version_reports_its_dialect_and_input() {
        let error = "7.0"
            .parse::<OpenMpVersion>()
            .expect_err("unsupported version must fail");
        assert_eq!(error.dialect(), Dialect::OpenMp);
        assert_eq!(error.input(), "7.0");
        assert!(error.to_string().contains("7.0"));
    }

    #[test]
    fn exact_and_any_policies_produce_typed_sets() {
        let exact = VersionPolicy::Exact(OpenMpVersion::V4_5).allowed_versions();
        let any = VersionPolicy::<OpenMpVersion>::Any.allowed_versions();

        assert_eq!(exact.len(), 1);
        assert!(exact.contains(OpenMpVersion::V4_5));
        assert_eq!(any.len(), OpenMpVersion::ALL.len());
    }

    #[test]
    fn version_sets_support_intersection_union_and_difference() {
        let early: VersionSet<OpenMpVersion> = [
            OpenMpVersion::V1_0,
            OpenMpVersion::V1_1,
            OpenMpVersion::V2_0,
        ]
        .into_iter()
        .collect();
        let overlapping: VersionSet<OpenMpVersion> = [OpenMpVersion::V2_0, OpenMpVersion::V2_5]
            .into_iter()
            .collect();

        assert_eq!(
            (early & overlapping).iter().collect::<Vec<_>>(),
            [OpenMpVersion::V2_0]
        );
        assert_eq!((early | overlapping).len(), 4);
        assert_eq!((early - overlapping).len(), 2);
        assert!(VersionSet::singleton(OpenMpVersion::V1_0).is_subset(early));
    }

    #[test]
    fn version_set_iteration_is_chronological() {
        let versions: VersionSet<OpenAccVersion> = [
            OpenAccVersion::V3_4,
            OpenAccVersion::V1_0,
            OpenAccVersion::V2_7,
        ]
        .into_iter()
        .collect();

        assert_eq!(
            versions.iter().collect::<Vec<_>>(),
            [
                OpenAccVersion::V1_0,
                OpenAccVersion::V2_7,
                OpenAccVersion::V3_4
            ]
        );
        assert_eq!(versions.earliest(), Some(OpenAccVersion::V1_0));
        assert_eq!(versions.latest(), Some(OpenAccVersion::V3_4));
        assert_eq!(versions.to_string(), "{1.0, 2.7, 3.4}");
    }

    #[test]
    fn version_set_insert_and_remove_report_membership_changes() {
        let mut versions = VersionSet::<OpenMpVersion>::empty();
        assert!(versions.insert(OpenMpVersion::V5_2));
        assert!(!versions.insert(OpenMpVersion::V5_2));
        assert!(versions.remove(OpenMpVersion::V5_2));
        assert!(!versions.remove(OpenMpVersion::V5_2));
        assert!(versions.is_empty());
    }

    #[test]
    fn source_forms_are_checked_against_host_language() {
        let c = HostLanguageProfile::C(CStandard::C23);
        let fortran = HostLanguageProfile::Fortran(FortranStandard::Fortran2023);

        assert_eq!(c.validate_source_form(SourceForm::Pragma), Ok(()));
        assert!(matches!(
            c.validate_source_form(SourceForm::FortranFree),
            Err(HostProfileError::IncompatibleSourceForm { .. })
        ));
        assert_eq!(
            fortran.validate_source_form(SourceForm::FortranFree),
            Ok(())
        );
        assert_eq!(
            fortran.validate_source_form(SourceForm::FortranFixed),
            Ok(())
        );
        assert!(fortran.validate_source_form(SourceForm::Pragma).is_err());
    }
}
