//! Explicit directive-spelling availability by specification version.
//!
//! This is deliberately a spelling catalog rather than an inference engine.
//! Every spelling emitted by the current typed directive enums is either listed
//! in a standardized group below or in an explicit nonstandard list. Adding a
//! parser directive without updating this file therefore fails the coverage
//! tests and strict parsing rejects it rather than assuming availability.

use crate::version::{DirectiveVersion, HostLanguage, OpenAccVersion, OpenMpVersion, VersionSet};

/// Host languages in which a directive spelling is standardized.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LanguageAvailability {
    All,
    CAndCpp,
    Fortran,
}

impl LanguageAvailability {
    #[must_use]
    pub const fn supports(self, language: HostLanguage) -> bool {
        match self {
            Self::All => true,
            Self::CAndCpp => matches!(language, HostLanguage::C | HostLanguage::Cpp),
            Self::Fortran => matches!(language, HostLanguage::Fortran),
        }
    }
}

/// Availability metadata for one standardized directive spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectiveAvailability<V: DirectiveVersion> {
    spelling: &'static str,
    introduced: V,
    removed: Option<V>,
    languages: LanguageAvailability,
}

impl<V: DirectiveVersion> DirectiveAvailability<V> {
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        self.spelling
    }

    #[must_use]
    pub const fn introduced(self) -> V {
        self.introduced
    }

    /// Returns the first version in which this spelling is no longer
    /// standardized. `None` means it remains standardized in the latest
    /// catalogued version.
    #[must_use]
    pub const fn removed(self) -> Option<V> {
        self.removed
    }

    #[must_use]
    pub const fn languages(self) -> LanguageAvailability {
        self.languages
    }

    /// Returns versions whose parsers must accept this spelling.
    ///
    /// ROUP is intentionally cumulative: once standardized syntax is
    /// introduced, later exact-version modes continue to accept it even if a
    /// later specification deprecates or removes the spelling. The `removed`
    /// metadata remains available to consumers that want to report the
    /// specification status, but it never makes maintained source unparseable.
    #[must_use]
    pub fn version_interval(self) -> VersionSet<V> {
        V::ALL
            .iter()
            .copied()
            .filter(|version| *version >= self.introduced)
            .collect()
    }
}

#[derive(Clone, Copy)]
struct AvailabilityGroup<V: DirectiveVersion> {
    spellings: &'static [&'static str],
    introduced: V,
    removed: Option<V>,
    languages: LanguageAvailability,
}

impl<V: DirectiveVersion> AvailabilityGroup<V> {
    fn entry(self, spelling: &'static str) -> DirectiveAvailability<V> {
        DirectiveAvailability {
            spelling,
            introduced: self.introduced,
            removed: self.removed,
            languages: self.languages,
        }
    }
}

use LanguageAvailability::{All, CAndCpp, Fortran};
use OpenAccVersion as Acc;
use OpenMpVersion as Omp;

// Version introductions are grouped only when the spelling and language
// availability are identical. Removal metadata is descriptive: parser
// acceptance remains cumulative so maintained historical source keeps working.
const OPENMP_GROUPS: &[AvailabilityGroup<OpenMpVersion>] = &[
    AvailabilityGroup {
        spellings: &[
            "atomic",
            "barrier",
            "critical",
            "flush",
            "ordered",
            "parallel",
            "parallel sections",
            "section",
            "sections",
            "single",
            "threadprivate",
        ],
        introduced: Omp::V1_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["master"],
        introduced: Omp::V1_0,
        removed: Some(Omp::V6_0),
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["for", "parallel for"],
        introduced: Omp::V1_0,
        removed: None,
        languages: CAndCpp,
    },
    AvailabilityGroup {
        spellings: &[
            "do",
            "end atomic",
            "end critical",
            "end do",
            "end ordered",
            "end parallel",
            "end parallel do",
            "end parallel sections",
            "end sections",
            "end single",
            "enddo",
            "parallel do",
            "paralleldo",
        ],
        introduced: Omp::V1_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &["end master"],
        introduced: Omp::V1_0,
        removed: Some(Omp::V6_0),
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &["parallel workshare", "workshare"],
        introduced: Omp::V2_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &["end parallel workshare", "end workshare"],
        introduced: Omp::V2_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &["task", "taskwait", "taskyield"],
        introduced: Omp::V3_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["end task"],
        introduced: Omp::V3_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "cancel",
            "cancellation point",
            "declare reduction",
            "declare simd",
            "declare target",
            "distribute",
            "distribute simd",
            "simd",
            "target",
            "target data",
            "target update",
            "taskgroup",
            "teams",
        ],
        introduced: Omp::V4_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &[
            "distribute parallel for",
            "distribute parallel for simd",
            "end declare target",
            "for simd",
            "parallel for simd",
        ],
        introduced: Omp::V4_0,
        removed: None,
        languages: CAndCpp,
    },
    AvailabilityGroup {
        spellings: &[
            "distribute parallel do",
            "distribute parallel do simd",
            "do simd",
            "end distribute",
            "end distribute parallel do",
            "end distribute parallel do simd",
            "end distribute simd",
            "end do simd",
            "end parallel do simd",
            "end simd",
            "end taskgroup",
            "end teams",
            "enddosimd",
            "parallel do simd",
        ],
        introduced: Omp::V4_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "target enter data",
            "target exit data",
            "target parallel",
            "target simd",
            "target teams",
            "target teams distribute",
            "target teams distribute simd",
            "taskloop",
            "taskloop simd",
            "teams distribute",
            "teams distribute simd",
        ],
        introduced: Omp::V4_5,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &[
            "target parallel for",
            "target parallel for simd",
            "target teams distribute parallel for",
            "target teams distribute parallel for simd",
            "teams distribute parallel for",
            "teams distribute parallel for simd",
        ],
        introduced: Omp::V4_5,
        removed: None,
        languages: CAndCpp,
    },
    AvailabilityGroup {
        spellings: &[
            "end target",
            "end target data",
            "end target parallel",
            "end target parallel do",
            "end target parallel do simd",
            "end target simd",
            "end target teams",
            "end target teams distribute",
            "end target teams distribute parallel do",
            "end target teams distribute parallel do simd",
            "end target teams distribute simd",
            "end taskloop",
            "end taskloop simd",
            "end teams distribute",
            "end teams distribute parallel do",
            "end teams distribute parallel do simd",
            "end teams distribute simd",
            "target parallel do",
            "target parallel do simd",
            "target teams distribute parallel do",
            "target teams distribute parallel do simd",
            "teams distribute parallel do",
            "teams distribute parallel do simd",
        ],
        introduced: Omp::V4_5,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "allocate",
            "begin metadirective",
            "declare mapper",
            "declare variant",
            "depobj",
            "end metadirective",
            "loop",
            "metadirective",
            "requires",
            "scan",
            "target loop",
            "target teams loop",
            "teams loop",
        ],
        introduced: Omp::V5_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["parallel loop"],
        introduced: Omp::V5_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &[
            "end loop",
            "end target loop",
            "end target teams loop",
            "end teams loop",
        ],
        introduced: Omp::V5_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "master taskloop",
            "master taskloop simd",
            "parallel master",
            "parallel master taskloop",
            "parallel master taskloop simd",
        ],
        introduced: Omp::V5_0,
        removed: Some(Omp::V6_0),
        languages: All,
    },
    AvailabilityGroup {
        spellings: &[
            "end master taskloop",
            "end master taskloop simd",
            "end parallel master",
            "end parallel master taskloop",
            "end parallel master taskloop simd",
        ],
        introduced: Omp::V5_0,
        removed: Some(Omp::V6_0),
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "assume",
            "assumes",
            "dispatch",
            "error",
            "interop",
            "masked",
            "masked taskloop",
            "masked taskloop simd",
            "nothing",
            "parallel masked",
            "parallel masked taskloop",
            "parallel masked taskloop simd",
            "scope",
            "tile",
            "unroll",
        ],
        introduced: Omp::V5_1,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &[
            "begin assumes",
            "begin declare target",
            "begin declare variant",
            "end assumes",
            "end declare variant",
        ],
        introduced: Omp::V5_1,
        removed: None,
        languages: CAndCpp,
    },
    AvailabilityGroup {
        spellings: &[
            "end assume",
            "end masked",
            "end masked taskloop",
            "end masked taskloop simd",
            "end parallel masked",
            "end parallel masked taskloop",
            "end parallel masked taskloop simd",
            "end scope",
            "end tile",
            "end unroll",
        ],
        introduced: Omp::V5_1,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &["allocators", "end allocators", "end dispatch"],
        introduced: Omp::V5_2,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "declare induction",
            "fuse",
            "groupprivate",
            "interchange",
            "reverse",
            "split",
            "stripe",
            "task iteration",
            "taskgraph",
        ],
        introduced: Omp::V6_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &[
            "distribute parallel loop",
            "distribute parallel loop simd",
            "parallel loop simd",
            "parallel single",
            "target loop simd",
            "target parallel loop",
            "target parallel loop simd",
            "target teams distribute parallel loop",
            "target teams distribute parallel loop simd",
            "target teams loop simd",
            "teams distribute parallel loop",
            "teams distribute parallel loop simd",
            "teams loop simd",
        ],
        introduced: Omp::V6_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["target teams workdistribute", "workdistribute"],
        introduced: Omp::V6_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &[
            "end parallel loop",
            "end parallel single",
            "end target parallel loop",
            "end target teams workdistribute",
        ],
        introduced: Omp::V6_0,
        removed: None,
        languages: Fortran,
    },
];

/// Parser spellings that the strict facade must reject because they do not
/// identify a standardized OpenMP directive spelling.
pub const OPENMP_NONSTANDARD_SPELLINGS: &[&str] = &[
    "end",
    "end distribute parallel for",
    "end distribute parallel for simd",
    "end for simd",
    "end parallel for",
    "end parallel for simd",
    "end target enter data",
    "end target exit data",
    "end target parallel for",
    "end target parallel for simd",
    "end target teams distribute parallel for",
    "end target teams distribute parallel for simd",
    "end target update",
    "end teams distribute parallel for",
    "end teams distribute parallel for simd",
    "end section",
    "ompx",
    "target data composite",
];

const OPENACC_GROUPS: &[AvailabilityGroup<OpenAccVersion>] = &[
    AvailabilityGroup {
        spellings: &[
            "cache",
            "data",
            "declare",
            "host_data",
            "kernels",
            "kernels loop",
            "loop",
            "parallel",
            "parallel loop",
            "update",
            "wait",
        ],
        introduced: Acc::V1_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["end"],
        introduced: Acc::V1_0,
        removed: None,
        languages: Fortran,
    },
    AvailabilityGroup {
        spellings: &["atomic", "enter data", "exit data", "routine"],
        introduced: Acc::V2_0,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["init", "set", "shutdown"],
        introduced: Acc::V2_5,
        removed: None,
        languages: All,
    },
    AvailabilityGroup {
        spellings: &["serial", "serial loop"],
        introduced: Acc::V2_6,
        removed: None,
        languages: All,
    },
];

/// Parser spellings explicitly known not to be standardized OpenACC directive
/// names. The current typed OpenACC enum has no such entries.
pub const OPENACC_NONSTANDARD_SPELLINGS: &[&str] = &[];

#[must_use]
pub fn openmp_directive_availability(
    spelling: &str,
) -> Option<DirectiveAvailability<OpenMpVersion>> {
    find_availability(OPENMP_GROUPS, spelling)
}

#[must_use]
pub fn openacc_directive_availability(
    spelling: &str,
) -> Option<DirectiveAvailability<OpenAccVersion>> {
    find_availability(OPENACC_GROUPS, spelling)
}

/// Computes OpenMP versions compatible with a spelling and host language.
///
/// OpenMP 1.1 was a Fortran-only release, so it is removed from C and C++
/// result sets even for directives otherwise available since 1.0.
#[must_use]
pub fn openmp_compatible_versions(
    availability: DirectiveAvailability<OpenMpVersion>,
    language: HostLanguage,
) -> VersionSet<OpenMpVersion> {
    if !availability.languages().supports(language) {
        return VersionSet::empty();
    }

    let mut versions = availability.version_interval();
    if !matches!(language, HostLanguage::Fortran) {
        versions.remove(OpenMpVersion::V1_1);
    }
    versions
}

#[must_use]
pub fn openacc_compatible_versions(
    availability: DirectiveAvailability<OpenAccVersion>,
    language: HostLanguage,
) -> VersionSet<OpenAccVersion> {
    if availability.languages().supports(language) {
        availability.version_interval()
    } else {
        VersionSet::empty()
    }
}

fn find_availability<V: DirectiveVersion>(
    groups: &'static [AvailabilityGroup<V>],
    spelling: &str,
) -> Option<DirectiveAvailability<V>> {
    groups.iter().find_map(|group| {
        group
            .spellings
            .iter()
            .copied()
            .find(|candidate| *candidate == spelling)
            .map(|candidate| group.entry(candidate))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AccDirectiveKind, OmpDirectiveKind};
    use std::collections::HashSet;

    #[test]
    fn every_current_openmp_kind_is_classified_once() {
        let mut standardized = HashSet::new();
        for group in OPENMP_GROUPS {
            for spelling in group.spellings {
                assert!(
                    standardized.insert(*spelling),
                    "duplicate OpenMP availability entry for {spelling:?}"
                );
            }
        }

        let nonstandard: HashSet<_> = OPENMP_NONSTANDARD_SPELLINGS.iter().copied().collect();
        assert!(standardized.is_disjoint(&nonstandard));
        for kind in OmpDirectiveKind::ALL {
            let spelling = kind.as_str();
            assert!(
                standardized.contains(spelling) || nonstandard.contains(spelling),
                "OpenMP directive {spelling:?} is not classified"
            );
        }
        for spelling in standardized {
            let name = crate::parser::directive_kind::lookup_directive_name(spelling);
            assert!(
                OmpDirectiveKind::try_from(name).is_ok(),
                "standardized OpenMP spelling {spelling:?} has no canonical typed kind"
            );
        }
    }

    #[test]
    fn parser_only_openmp_names_cannot_become_public_typed_kinds() {
        for spelling in OPENMP_NONSTANDARD_SPELLINGS
            .iter()
            .copied()
            .filter(|spelling| !matches!(*spelling, "ompx" | "end section"))
            .chain(["parallel_for", "begin_declare_target"])
        {
            let raw = crate::parser::directive_kind::lookup_directive_name(spelling);
            assert!(
                OmpDirectiveKind::try_from(raw).is_err(),
                "nonstandard OpenMP spelling {spelling:?} reached OmpDirectiveKind"
            );
        }
    }

    #[test]
    fn every_current_openacc_kind_is_classified_once() {
        let mut standardized = HashSet::new();
        for group in OPENACC_GROUPS {
            for spelling in group.spellings {
                assert!(
                    standardized.insert(*spelling),
                    "duplicate OpenACC availability entry for {spelling:?}"
                );
            }
        }

        let nonstandard: HashSet<_> = OPENACC_NONSTANDARD_SPELLINGS.iter().copied().collect();
        assert!(standardized.is_disjoint(&nonstandard));
        for kind in AccDirectiveKind::ALL {
            let spelling = kind.as_str();
            assert!(
                standardized.contains(spelling) || nonstandard.contains(spelling),
                "OpenACC directive {spelling:?} is not classified"
            );
        }
        assert_eq!(
            standardized.len() + nonstandard.len(),
            AccDirectiveKind::ALL.len()
        );
    }

    #[test]
    fn historical_master_remains_accepted_in_openmp_six() {
        for (spelling, introduced) in [
            ("master", OpenMpVersion::V1_0),
            ("parallel master", OpenMpVersion::V5_0),
        ] {
            let availability = openmp_directive_availability(spelling)
                .unwrap_or_else(|| panic!("{spelling} must have standardized availability"));
            let versions = openmp_compatible_versions(availability, HostLanguage::C);

            assert!(versions.contains(introduced));
            assert!(versions.contains(OpenMpVersion::V5_2));
            assert!(versions.contains(OpenMpVersion::V6_0));
        }
    }

    #[test]
    fn openmp_one_point_one_is_fortran_only() {
        let availability = openmp_directive_availability("parallel")
            .expect("parallel must have standardized availability");
        let c_versions = openmp_compatible_versions(availability, HostLanguage::C);
        let fortran_versions = openmp_compatible_versions(availability, HostLanguage::Fortran);

        assert!(!c_versions.contains(OpenMpVersion::V1_1));
        assert!(fortran_versions.contains(OpenMpVersion::V1_1));
    }

    #[test]
    fn language_specific_spellings_are_filtered() {
        let omp_do = openmp_directive_availability("do").expect("do is standardized");
        assert!(openmp_compatible_versions(omp_do, HostLanguage::C).is_empty());
        assert!(
            openmp_compatible_versions(omp_do, HostLanguage::Fortran).contains(OpenMpVersion::V1_0)
        );

        let acc_end = openacc_directive_availability("end").expect("end is standardized");
        assert!(openacc_compatible_versions(acc_end, HostLanguage::Cpp).is_empty());
        assert!(
            openacc_compatible_versions(acc_end, HostLanguage::Fortran)
                .contains(OpenAccVersion::V1_0)
        );
    }

    #[test]
    fn openacc_serial_was_introduced_in_two_point_six() {
        let availability =
            openacc_directive_availability("serial").expect("serial is standardized");

        assert_eq!(availability.introduced(), OpenAccVersion::V2_6);
        assert!(
            !availability
                .version_interval()
                .contains(OpenAccVersion::V2_5)
        );
        assert!(
            availability
                .version_interval()
                .contains(OpenAccVersion::V2_6)
        );
    }
}
