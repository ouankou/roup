#include <OpenMPIR.h>

#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

bool rejects(const char *source) {
  try {
    std::unique_ptr<OpenMPDirective> directive(
        parseOpenMP(source, nullptr, nullptr));
    return directive == nullptr;
  } catch (const std::exception &) {
    return true;
  }
}

void expect_scalar_clause(const char *source, OpenMPClauseKind kind,
                          const char *expected_expression) {
  std::unique_ptr<OpenMPDirective> directive(
      parseOpenMP(source, nullptr, nullptr));
  if (!directive)
    throw std::runtime_error(std::string("valid input returned null: ") +
                             source);

  std::vector<OpenMPClause *> *clauses = directive->getClauses(kind);
  if (clauses == nullptr || clauses->size() != 1 || clauses->front() == nullptr)
    throw std::runtime_error(std::string("missing unique typed clause for: ") +
                             source);

  std::vector<const char *> *expressions = clauses->front()->getExpressions();
  if (expected_expression == nullptr) {
    if (expressions != nullptr && !expressions->empty())
      throw std::runtime_error(std::string("unexpected clause expression for: ") +
                               source);
    return;
  }
  if (expressions == nullptr || expressions->size() != 1 ||
      expressions->front() == nullptr ||
      std::string(expressions->front()) != expected_expression) {
    throw std::runtime_error(std::string("typed clause expression mismatch for: ") +
                             source);
  }
}

void expect_named_fortran_end_critical() {
  setLang(Lang_Fortran);
  std::unique_ptr<OpenMPDirective> directive(
      parseOpenMP("!$omp end critical(region_name)", nullptr, nullptr));
  OpenMPEndDirective *end =
      directive ? dynamic_cast<OpenMPEndDirective *>(directive.get()) : nullptr;
  OpenMPDirective *paired = end ? end->getPairedDirective() : nullptr;
  OpenMPCriticalDirective *critical =
      paired ? dynamic_cast<OpenMPCriticalDirective *>(paired) : nullptr;
  const std::string name = critical ? critical->getCriticalName() : "";
  setLang(Lang_C);

  if (end == nullptr || paired == nullptr ||
      paired->getKind() != OMPD_critical || critical == nullptr ||
      name != "region_name") {
    throw std::runtime_error(
        "named Fortran end critical lost its paired critical parameter");
  }
}

void expect_fortran_end_pair(const char *source,
                             OpenMPDirectiveKind paired_kind) {
  setLang(Lang_Fortran);
  std::unique_ptr<OpenMPDirective> directive(
      parseOpenMP(source, nullptr, nullptr));
  OpenMPEndDirective *end =
      directive ? dynamic_cast<OpenMPEndDirective *>(directive.get()) : nullptr;
  OpenMPDirective *paired = end ? end->getPairedDirective() : nullptr;
  setLang(Lang_C);
  if (end == nullptr || paired == nullptr || paired->getKind() != paired_kind) {
    throw std::runtime_error(std::string("typed end-pair mapping failed for: ") +
                             source);
  }
}

void expect_typed_directive_parameters() {
  setLang(Lang_Cplusplus);
  std::unique_ptr<OpenMPDirective> variant(parseOpenMP(
      "#pragma omp declare variant(base:fast<int>) "
      "match(construct={parallel})",
      nullptr, nullptr));
  OpenMPDeclareVariantDirective *declare_variant =
      variant ? dynamic_cast<OpenMPDeclareVariantDirective *>(variant.get())
              : nullptr;
  if (declare_variant == nullptr ||
      declare_variant->getVariantFuncID() != "base:fast<int>") {
    setLang(Lang_C);
    throw std::runtime_error(
        "typed declare-variant base/template-id was lost by the adapter");
  }

  std::unique_ptr<OpenMPDirective> allocate(parseOpenMP(
      "#pragma omp allocate(::ns::value, Type::storage)", nullptr,
      nullptr));
  OpenMPAllocateDirective *typed_allocate =
      allocate ? dynamic_cast<OpenMPAllocateDirective *>(allocate.get())
               : nullptr;
  if (typed_allocate == nullptr || typed_allocate->getAllocateList().size() != 2 ||
      typed_allocate->getAllocateList()[0] != "::ns::value" ||
      typed_allocate->getAllocateList()[1] != "Type::storage") {
    setLang(Lang_C);
    throw std::runtime_error(
        "directive-specific allocate list was lost by the adapter");
  }

  setLang(Lang_Fortran);
  std::unique_ptr<OpenMPDirective> simd(parseOpenMP(
      "!$omp declare simd(PROC) simdlen(4)", nullptr, nullptr));
  OpenMPDeclareSimdDirective *declare_simd =
      simd ? dynamic_cast<OpenMPDeclareSimdDirective *>(simd.get()) : nullptr;
  const std::string proc_name = declare_simd ? declare_simd->getProcName() : "";
  setLang(Lang_C);
  if (declare_simd == nullptr || proc_name != "proc")
    throw std::runtime_error(
        "required Fortran declare-simd target was lost by the adapter");
}

void expect_typed_clause_item_variants() {
  setLang(Lang_C);
  std::unique_ptr<OpenMPDirective> private_items(parseOpenMP(
      "#pragma omp parallel private(value, array[0:length])", nullptr,
      nullptr));
  std::vector<OpenMPClause *> *private_clauses =
      private_items ? private_items->getClauses(OMPC_private) : nullptr;
  std::vector<const char *> *private_values =
      private_clauses && private_clauses->size() == 1 &&
              private_clauses->front()
          ? private_clauses->front()->getExpressions()
          : nullptr;
  if (private_values == nullptr || private_values->size() != 2 ||
      std::string(private_values->at(0)) != "value" ||
      std::string(private_values->at(1)) != "array[0 : length]") {
    throw std::runtime_error(
        "identifier/variable clause-item nodes were lost by the adapter: " +
        (private_items ? private_items->generatePragmaString()
                       : std::string("<null directive>")));
  }

  std::unique_ptr<OpenMPDirective> doacross(parseOpenMP(
      "#pragma omp ordered depend(sink: i - 1)", nullptr, nullptr));
  if (doacross == nullptr ||
      doacross->generatePragmaString() !=
          "#pragma omp ordered depend (sink : i - 1)") {
    throw std::runtime_error(
        "historical depend spelling or expression node was lost: " +
        (doacross ? doacross->generatePragmaString()
                  : std::string("<null directive>")));
  }

  setLang(Lang_Fortran);
  std::unique_ptr<OpenMPDirective> common_blocks(parseOpenMP(
      "!$omp parallel private(/WORK/) copyin(/STATE/)", nullptr, nullptr));
  std::vector<OpenMPClause *> *private_common =
      common_blocks ? common_blocks->getClauses(OMPC_private) : nullptr;
  std::vector<OpenMPClause *> *copyin_common =
      common_blocks ? common_blocks->getClauses(OMPC_copyin) : nullptr;
  const bool common_blocks_preserved =
      private_common && private_common->size() == 1 &&
      private_common->front() &&
      private_common->front()->getExpressions()->size() == 1 &&
      std::string(private_common->front()->getExpressions()->front()) ==
          "/work/" &&
      copyin_common && copyin_common->size() == 1 && copyin_common->front() &&
      copyin_common->front()->getExpressions()->size() == 1 &&
      std::string(copyin_common->front()->getExpressions()->front()) ==
          "/state/";
  setLang(Lang_C);
  if (!common_blocks_preserved) {
    throw std::runtime_error(
        "Fortran common-block clause-item nodes were lost by the adapter");
  }
}

void expect_if_condition_and_normalized_merges() {
  setLang(Lang_C);
  expect_scalar_clause("#pragma omp parallel if(flag)", OMPC_if, "flag");

  std::unique_ptr<OpenMPDirective> linear(parseOpenMP(
      "#pragma omp simd linear(a: 1) linear(b: 1)", nullptr, nullptr));
  std::vector<OpenMPClause *> *linear_clauses =
      linear ? linear->getClauses(OMPC_linear) : nullptr;
  if (linear_clauses == nullptr || linear_clauses->size() != 1 ||
      linear_clauses->front()->getExpressions()->size() != 2) {
    throw std::runtime_error(
        "normalized linear clauses did not merge into the first occurrence");
  }

  std::unique_ptr<OpenMPDirective> depend(parseOpenMP(
      "#pragma omp task depend(in: a) depend(in: b)", nullptr, nullptr));
  std::vector<OpenMPClause *> *depend_clauses =
      depend ? depend->getClauses(OMPC_depend) : nullptr;
  if (depend_clauses == nullptr || depend_clauses->size() != 1 ||
      depend_clauses->front()->getExpressions()->size() != 2) {
    throw std::runtime_error(
        "normalized depend clauses did not merge into the first occurrence");
  }
}

void expect_openmp6_clause_shapes_and_modifiers() {
  setLang(Lang_C);
  std::unique_ptr<OpenMPDirective> task(parseOpenMP(
      "#pragma omp task threadset(task: omp_pool)", nullptr, nullptr));
  std::vector<OpenMPClause *> *threadsets =
      task ? task->getClauses(OMPC_threadset) : nullptr;
  OpenMPClause *threadset =
      threadsets && threadsets->size() == 1 ? threadsets->front() : nullptr;
  if (threadset == nullptr || !threadset->hasDirectiveNameModifier() ||
      threadset->getDirectiveNameModifier() != OMPD_task ||
      threadset->getExpressions()->size() != 1 ||
      std::string(threadset->getExpressions()->front()) != "omp_pool") {
    throw std::runtime_error(
        "typed threadset kind or universal modifier was lost");
  }

  std::unique_ptr<OpenMPDirective> atomic(parseOpenMP(
      "#pragma omp atomic read memscope(atomic: cgroup)", nullptr, nullptr));
  std::vector<OpenMPClause *> *memscopes =
      atomic ? atomic->getClauses(OMPC_memscope) : nullptr;
  auto *memscope = memscopes && memscopes->size() == 1
                       ? dynamic_cast<OpenMPMemscopeClause *>(memscopes->front())
                       : nullptr;
  if (memscope == nullptr || memscope->getScope() != OMPC_MEMSCOPE_cgroup ||
      !memscope->hasDirectiveNameModifier() ||
      memscope->getDirectiveNameModifier() != OMPD_atomic) {
    throw std::runtime_error(
        "typed memscope kind or universal modifier was lost");
  }

  std::unique_ptr<OpenMPDirective> fuse(parseOpenMP(
      "#pragma omp fuse looprange(2, number_of_loops)", nullptr, nullptr));
  std::vector<OpenMPClause *> *loopranges =
      fuse ? fuse->getClauses(OMPC_looprange) : nullptr;
  OpenMPClause *looprange =
      loopranges && loopranges->size() == 1 ? loopranges->front() : nullptr;
  if (looprange == nullptr || looprange->getExpressions()->size() != 2 ||
      std::string(looprange->getExpressions()->at(0)) != "2" ||
      std::string(looprange->getExpressions()->at(1)) != "number_of_loops") {
    throw std::runtime_error("ordered looprange fields were lost");
  }

  std::unique_ptr<OpenMPDirective> graph(parseOpenMP(
      "#pragma omp taskgraph graph_reset", nullptr, nullptr));
  std::vector<OpenMPClause *> *resets =
      graph ? graph->getClauses(OMPC_graph_reset) : nullptr;
  if (resets == nullptr || resets->size() != 1 ||
      !resets->front()->getExpressions()->empty()) {
    throw std::runtime_error("bare graph_reset was not preserved");
  }

  for (const char *source : {
           "#pragma omp target map(mapper(default), to: x)",
           "#pragma omp target map(mapper(custom), to: x)",
       }) {
    std::unique_ptr<OpenMPDirective> mapped(
        parseOpenMP(source, nullptr, nullptr));
    std::vector<OpenMPClause *> *maps =
        mapped ? mapped->getClauses(OMPC_map) : nullptr;
    auto *map = maps && maps->size() == 1
                    ? dynamic_cast<OpenMPMapClause *>(maps->front())
                    : nullptr;
    const std::string expected =
        std::string(source).find("default") != std::string::npos ? "default"
                                                                  : "custom";
    if (map == nullptr || map->getMapperIdentifier() != expected) {
      throw std::runtime_error("typed mapper identifier was lost");
    }
  }

  std::unique_ptr<OpenMPDirective> metadirective(parseOpenMP(
      "#pragma omp metadirective when(user={condition(enabled)}: parallel private(parallel: x))",
      nullptr, nullptr));
  std::vector<OpenMPClause *> *whens =
      metadirective ? metadirective->getClauses(OMPC_when) : nullptr;
  auto *when = whens && whens->size() == 1
                   ? dynamic_cast<OpenMPWhenClause *>(whens->front())
                   : nullptr;
  OpenMPDirective *nested = when ? when->getVariantDirective() : nullptr;
  std::vector<OpenMPClause *> *nested_private =
      nested ? nested->getClauses(OMPC_private) : nullptr;
  OpenMPClause *private_clause =
      nested_private && nested_private->size() == 1
          ? nested_private->front()
          : nullptr;
  if (private_clause == nullptr ||
      !private_clause->hasDirectiveNameModifier() ||
      private_clause->getDirectiveNameModifier() != OMPD_parallel) {
    throw std::runtime_error(
        "nested universal directive-name modifier was lost");
  }

  std::unique_ptr<OpenMPDirective> encoded_selector(parseOpenMP(
      "#pragma omp metadirective when(device={isa(u8\"gpu\\001\")}: nothing)",
      nullptr, nullptr));
  std::vector<OpenMPClause *> *encoded_whens =
      encoded_selector ? encoded_selector->getClauses(OMPC_when) : nullptr;
  auto *encoded_when = encoded_whens && encoded_whens->size() == 1
                           ? dynamic_cast<OpenMPWhenClause *>(
                                 encoded_whens->front())
                           : nullptr;
  auto *isa = encoded_when ? encoded_when->getIsaExpression() : nullptr;
  if (isa == nullptr || isa->expression != "u8\"gpu\\001\"") {
    throw std::runtime_error(
        "selector literal encoding or control-byte escaping was lost");
  }
}

void expect_allocation_expression_contract() {
  setLang(Lang_C);
  std::unique_ptr<OpenMPDirective> allocation(parseOpenMP(
      "#pragma omp parallel private(value) "
      "allocate(select_allocator(device): value)",
      nullptr, nullptr));
  std::vector<OpenMPClause *> *allocate_clauses =
      allocation ? allocation->getClauses(OMPC_allocate) : nullptr;
  auto *allocate = allocate_clauses && allocate_clauses->size() == 1
                       ? dynamic_cast<OpenMPAllocateClause *>(
                             allocate_clauses->front())
                       : nullptr;
  if (allocate == nullptr ||
      allocate->getAllocator() != OMPC_ALLOCATE_ALLOCATOR_user ||
      allocate->getUserDefinedAllocator() != "select_allocator(device)" ||
      allocate->getExpressions()->size() != 1 ||
      std::string(allocate->getExpressions()->front()) != "value") {
    throw std::runtime_error(
        "open allocator expression was reclassified or lost on allocate");
  }

  std::unique_ptr<OpenMPDirective> directive(parseOpenMP(
      "#pragma omp allocate(value) allocator(select_allocator(device))",
      nullptr, nullptr));
  std::vector<OpenMPClause *> *allocator_clauses =
      directive ? directive->getClauses(OMPC_allocator) : nullptr;
  auto *allocator = allocator_clauses && allocator_clauses->size() == 1
                        ? dynamic_cast<OpenMPAllocatorClause *>(
                              allocator_clauses->front())
                        : nullptr;
  if (allocator == nullptr ||
      allocator->getAllocator() != OMPC_ALLOCATOR_ALLOCATOR_user ||
      allocator->getUserDefinedAllocator() != "select_allocator(device)") {
    throw std::runtime_error(
        "open allocator-clause expression was reclassified or lost");
  }
}

void expect_locations(const char *source, OpenMPBaseLang language,
                      OpenMPDirectiveKind directive_kind,
                      int directive_column, OpenMPClauseKind first_kind,
                      int first_column, OpenMPClauseKind second_kind,
                      int second_column) {
  setLang(language);
  std::unique_ptr<OpenMPDirective> directive(
      parseOpenMP(source, nullptr, nullptr));
  if (!directive) {
    setLang(Lang_C);
    throw std::runtime_error(std::string("valid input returned null: ") +
                             source);
  }

  std::vector<OpenMPClause *> *first = directive->getClauses(first_kind);
  std::vector<OpenMPClause *> *second = directive->getClauses(second_kind);
  const bool matches =
      directive->getKind() == directive_kind && directive->getLine() == 1 &&
      directive->getColumn() == directive_column && first != nullptr &&
      first->size() == 1 && first->front() != nullptr &&
      first->front()->getLine() == 1 &&
      first->front()->getColumn() == first_column && second != nullptr &&
      second->size() == 1 && second->front() != nullptr &&
      second->front()->getLine() == 1 &&
      second->front()->getColumn() == second_column;
  setLang(Lang_C);
  if (!matches)
    throw std::runtime_error(std::string("source location mismatch for: ") +
                             source);
}

void expect_fortran_source_form_selection() {
  struct FortranCase {
    const char *source;
    OpenMPDirectiveKind kind;
  };
  const FortranCase cases[] = {
      {"!$omp parallel", OMPD_parallel},
      {"      C$OMP PARALLEL DO", OMPD_parallel_do},
      {"  *$omp parallel", OMPD_parallel},
      {"C$ parallel", OMPD_parallel},
      {"*$ PARALLEL", OMPD_parallel},
  };

  setLang(Lang_Fortran);
  for (const FortranCase &test : cases) {
    std::unique_ptr<OpenMPDirective> directive(
        parseOpenMP(test.source, nullptr, nullptr));
    if (directive == nullptr || directive->getKind() != test.kind ||
        directive->getBaseLang() != Lang_Fortran) {
      setLang(Lang_C);
      throw std::runtime_error(std::string("wrong Fortran source-form profile for: ") +
                               test.source);
    }
  }
  const bool pragma_conflict = rejects("#pragma omp parallel");

  setLang(Lang_C);
  const bool fixed_conflict = rejects("C$OMP PARALLEL");
  if (!pragma_conflict || !fixed_conflict) {
    throw std::runtime_error(
        "OpenMP source-form selection accepted a conflicting base language");
  }
}

} // namespace

int main() {
  if (!rejects("#pragma omp parallel"))
    throw std::runtime_error("unknown base language did not fail explicitly");
  setLang(Lang_C);
  if (!rejects(nullptr))
    throw std::runtime_error("null input was not rejected");
  if (!rejects(""))
    throw std::runtime_error("empty input was not rejected");
  for (const char *source : {
           "#pragma omp unknown_directive",
           "#pragma omp parallel private(value",
           "#pragma omp parallel private(value@)",
           "pragma omp parallel",
       }) {
    if (!rejects(source))
      throw std::runtime_error(std::string("malformed input was accepted: ") +
                               source);
  }

  auto expect_accepts = [](const char *source) {
    std::unique_ptr<OpenMPDirective> directive(
        parseOpenMP(source, nullptr, nullptr));
    if (!directive)
      throw std::runtime_error(std::string("upstream-compatible syntax was rejected: ") +
                               source);
  };
  expect_accepts("#pragma omp parallel private(c/)");
  expect_accepts("#pragma omp parallel reduction(+: foo(x))");
  expect_accepts("#pragma omp parallel firstprivate(foo(x))");
  expect_accepts("#pragma omp atomic seq_cst, hint(abc), read,");
  expect_accepts("#pragma omp parallel private(value),");
  expect_accepts("#pragma omp task depend(depobj: *obj)");
  expect_accepts("#pragma omp for reduction(original(private),+: sum_v)");

  setLang(Lang_Cplusplus);
  expect_accepts("#pragma omp target map(this->values[0:count])");
  expect_accepts(
      "#pragma omp declare reduction(+ : std::vector<int>) "
      "combiner(std::plus<int>()(omp_out, omp_in))");

  setLang(Lang_Fortran);
  expect_accepts(
      "!$omp declare reduction(.add. : dt) "
      "combiner(omp_out=omp_out.add.omp_in) initializer(dt_init(omp_priv))");
  expect_accepts("!$omp omp teams num_teams(4)");
  expect_accepts("!$ompx vendor Name(\"GPU0\")");
  setLang(Lang_C);

  // The opaque ABI names every semantic component. The adapter must consume
  // those fields explicitly instead of assuming that these clauses are
  // fieldless because their pre-6.0 spellings were bare.
  expect_scalar_clause("#pragma omp atomic write seq_cst", OMPC_seq_cst,
                       nullptr);
  expect_scalar_clause("#pragma omp atomic read(use_it)", OMPC_read,
                       "use_it");
  expect_scalar_clause("#pragma omp atomic acquire(use_order)", OMPC_acquire,
                       "use_order");
  expect_scalar_clause("#pragma omp for nowait(skip_barrier)", OMPC_nowait,
                       "skip_barrier");
  expect_scalar_clause("#pragma omp taskloop nogroup(skip_group)", OMPC_nogroup,
                       "skip_group");
  expect_scalar_clause("#pragma omp declare simd inbranch(branch_only)",
                       OMPC_inbranch, "branch_only");
  expect_scalar_clause("#pragma omp unroll partial(8)", OMPC_partial, "8");
  expect_scalar_clause("#pragma omp unroll full(all_iterations)", OMPC_full,
                       "all_iterations");
  expect_scalar_clause("#pragma omp task mergeable(may_merge)", OMPC_mergeable,
                       "may_merge");
  expect_scalar_clause("#pragma omp task untied(may_move)", OMPC_untied,
                       "may_move");
  expect_scalar_clause("#pragma omp atomic compare(compare_it)", OMPC_compare,
                       "compare_it");
  expect_scalar_clause("#pragma omp ordered simd(use_lanes)", OMPC_simd,
                       "use_lanes");
  expect_scalar_clause("#pragma omp ordered threads(use_threads)", OMPC_threads,
                       "use_threads");
  expect_scalar_clause("#pragma omp dispatch nocontext(skip_context)",
                       OMPC_nocontext, "skip_context");
  expect_scalar_clause("#pragma omp dispatch novariants(skip_variants)",
                       OMPC_novariants, "skip_variants");
  expect_scalar_clause("#pragma omp assume no_parallelism(trust_me)",
                       OMPC_no_parallelism, "trust_me");
  expect_scalar_clause("#pragma omp declare target indirect(via_pointer)",
                       OMPC_indirect, "via_pointer");
  expect_scalar_clause("#pragma omp task replayable(replay_it)",
                       OMPC_replayable, "replay_it");
  expect_scalar_clause("#pragma omp parallel safesync(4)", OMPC_safesync, "4");
  expect_scalar_clause("#pragma omp task transparent(omp_import)",
                       OMPC_transparent, "omp_import");
  expect_scalar_clause("#pragma omp requires reverse_offload(required_flag)",
                       OMPC_reverse_offload, "required_flag");
  expect_scalar_clause("#pragma omp depobj(handle) update(inout)",
                       OMPC_depobj_update, nullptr);
  expect_typed_directive_parameters();
  expect_typed_clause_item_variants();
  expect_if_condition_and_normalized_merges();
  expect_openmp6_clause_shapes_and_modifiers();
  expect_allocation_expression_contract();
  expect_named_fortran_end_critical();
  expect_fortran_end_pair("!$omp end allocators", OMPD_allocators);
  expect_fortran_end_pair("!$omp end dispatch", OMPD_dispatch);
  expect_locations("#pragma omp parallel for private(i) collapse(2)", Lang_C,
                   OMPD_parallel_for, 13, OMPC_private, 26, OMPC_collapse,
                   37);
  expect_locations("!$omp parallel do private(i) collapse(2)", Lang_Fortran,
                   OMPD_parallel_do, 7, OMPC_private, 19, OMPC_collapse, 30);
  expect_fortran_source_form_selection();
  return 0;
}
