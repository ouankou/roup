use roup::ir::{
    convert_directive, BindModifier, ClauseData, DepobjUpdateDependence, DeviceModifier,
    DirectiveKind, GrainsizeModifier, Language, MemoryOrder, NumTasksModifier, OrderKind,
    OrderModifier, ParserConfig, SourceLocation, ValidationContext,
};
use roup::parser::{openmp, Parser};

fn parse_first_clause(source: &str) -> ClauseData {
    let parser: Parser = openmp::parser();
    let (_rest, directive) = parser.parse(source).expect("directive should parse");

    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::C,
        &ParserConfig::default(),
    )
    .expect("convert to IR");

    ir.clauses()
        .first()
        .cloned()
        .expect("expected at least one clause")
}

#[test]
fn device_modifier_allowed_on_target() {
    let clause = parse_first_clause("#pragma omp target device(ancestor: 1)");
    let ctx = ValidationContext::new(DirectiveKind::Target);
    assert!(ctx.is_clause_allowed(&clause).is_ok());
    if let ClauseData::Device { modifier, .. } = clause {
        assert_eq!(modifier, DeviceModifier::Ancestor);
    } else {
        panic!("expected device clause");
    }
}

#[test]
fn device_clause_rejected_off_target() {
    let clause = ClauseData::Device {
        modifier: DeviceModifier::Ancestor,
        device_num: roup::ir::Expression::unparsed("1"),
    };
    let ctx = ValidationContext::new(DirectiveKind::Parallel);
    assert!(ctx.is_clause_allowed(&clause).is_err());
}

#[test]
fn depobj_update_allowed_on_depobj() {
    let clause = ClauseData::DepobjUpdate {
        dependence: DepobjUpdateDependence::Inout,
    };
    let ctx = ValidationContext::new(DirectiveKind::Depobj);
    assert!(ctx.is_clause_allowed(&clause).is_ok());
}

#[test]
fn depobj_update_rejected_off_depobj() {
    let clause = ClauseData::DepobjUpdate {
        dependence: DepobjUpdateDependence::In,
    };
    let ctx = ValidationContext::new(DirectiveKind::Target);
    assert!(ctx.is_clause_allowed(&clause).is_err());
    if let ClauseData::DepobjUpdate { dependence } = clause {
        assert_eq!(dependence, DepobjUpdateDependence::In);
    }
}

#[test]
fn bind_clause_roundtrips_with_modifier() {
    let clause = parse_first_clause("#pragma omp parallel bind(parallel)");
    // Validation just needs to allow it on parallel constructs.
    let ctx = ValidationContext::new(DirectiveKind::Parallel);
    assert!(ctx.is_clause_allowed(&clause).is_ok());
    if let ClauseData::Bind(binding) = clause {
        assert_eq!(binding, BindModifier::Parallel);
    } else {
        panic!("expected bind clause");
    }
}

#[test]
fn grainsize_and_num_tasks_strict_modifiers_parse() {
    let g_clause = parse_first_clause("#pragma omp taskloop grainsize(strict: 4)");
    let nt_clause = parse_first_clause("#pragma omp taskloop num_tasks(strict: 2)");
    let ctx = ValidationContext::new(DirectiveKind::Taskloop);
    assert!(ctx.is_clause_allowed(&g_clause).is_ok());
    assert!(ctx.is_clause_allowed(&nt_clause).is_ok());
    if let ClauseData::Grainsize { modifier, grain } = g_clause {
        assert_eq!(modifier, GrainsizeModifier::Strict);
        assert_eq!(grain.to_string(), "4");
    } else {
        panic!("expected grainsize clause");
    }
    if let ClauseData::NumTasks { modifier, num } = nt_clause {
        assert_eq!(modifier, NumTasksModifier::Strict);
        assert_eq!(num.to_string(), "2");
    } else {
        panic!("expected num_tasks clause");
    }
}

#[test]
fn order_and_atomic_default_mem_order_parse() {
    let order_clause = parse_first_clause("#pragma omp loop order(reproducible: concurrent)");
    let atomic_clause =
        parse_first_clause("#pragma omp atomic read atomic_default_mem_order(acq_rel)");
    // order allowed on loop; atomic_default_mem_order allowed on atomic
    let loop_ctx = ValidationContext::new(DirectiveKind::Loop);
    assert!(loop_ctx.is_clause_allowed(&order_clause).is_ok());

    let atomic_ctx = ValidationContext::new(DirectiveKind::Atomic);
    assert!(atomic_ctx.is_clause_allowed(&atomic_clause).is_ok());

    if let ClauseData::Order { modifier, kind } = order_clause {
        assert_eq!(modifier, OrderModifier::Reproducible);
        assert_eq!(kind, OrderKind::Concurrent);
    } else {
        panic!("expected order clause");
    }

    if let ClauseData::AtomicDefaultMemOrder(mem) = atomic_clause {
        assert_eq!(mem, MemoryOrder::AcqRel);
    } else {
        panic!("expected atomic_default_mem_order clause");
    }
}
