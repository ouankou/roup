#include <OpenACCParser.h>
#include <roup.h>

#include "typed_contract.h"

#include <cstdint>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>

extern "C" void setLang(OpenACCBaseLang);

namespace {

template <typename Function>
void require_hard_error(Function operation, const char *subject) {
  try {
    operation();
  } catch (const std::exception &) {
    return;
  }
  throw std::runtime_error(std::string(subject) + " did not throw");
}

void require(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}

void test_language_selection_is_mandatory() {
  require_hard_error([] { parseOpenACC("#pragma acc parallel"); },
                     "parse before explicit language selection");
}

void test_numeric_contract_rejects_unknown_tags() {
  require(roup_acc_contract::directive_kind(
              {ROUP_DIALECT_OPENACC, ROUP_ACC_DIRECTIVE_UPDATE}) == ACCD_update,
          "typed directive ordinal did not map exhaustively");
  require(roup_acc_contract::clause_kind(
              {ROUP_DIALECT_OPENACC, ROUP_ACC_CLAUSE_SELF_CLAUSE}) == ACCC_self,
          "typed self-clause ordinal did not map exhaustively");
  require(roup_acc_contract::default_kind(ROUP_ACC_DEFAULT_PRESENT) ==
              ACCC_DEFAULT_present,
          "typed default-kind tag did not map numerically");
  require(roup_acc_contract::data_modifier(
              ROUP_ACC_DATA_MODIFIER_READONLY) == ACCC_DATA_MOD_readonly,
          "typed data-modifier tag did not map numerically");
  require(roup_acc_contract::builtin_device_type(
              ROUP_ACC_DEVICE_TYPE_MULTICORE) == ACCC_DEVICE_TYPE_multicore,
          "typed device-type tag did not map numerically");
  require(roup_acc_contract::builtin_reduction_operator(
              ROUP_ACC_REDUCTION_LOG_AND) == ACCC_REDUCTION_logand,
          "typed reduction-operator tag did not map numerically");

  constexpr std::uint32_t unknown = UINT32_MAX;
  require_hard_error(
      [=] {
        roup_acc_contract::directive_kind({ROUP_DIALECT_OPENACC, unknown});
      },
      "unknown directive ordinal");
  require_hard_error(
      [=] { roup_acc_contract::clause_kind({ROUP_DIALECT_OPENACC, unknown}); },
      "unknown clause ordinal");
  require_hard_error([=] { roup_acc_contract::default_kind(unknown); },
                     "unknown default-kind tag");
  require_hard_error([=] { roup_acc_contract::data_modifier(unknown); },
                     "unknown data-modifier tag");
  require_hard_error(
      [=] { roup_acc_contract::copy_variant(unknown, ACCC_copy); },
      "unknown copy-kind tag");
  require_hard_error([=] { roup_acc_contract::create_variant(unknown); },
                     "unknown create-kind tag");
  require_hard_error([=] { roup_acc_contract::data_clause_kind(unknown); },
                     "unknown data-kind tag");
  require_hard_error([=] { roup_acc_contract::worker_modifier(unknown); },
                     "unknown worker-modifier tag");
  require_hard_error([=] { roup_acc_contract::vector_modifier(unknown); },
                     "unknown vector-modifier tag");
  require_hard_error([=] { roup_acc_contract::builtin_device_type(unknown); },
                     "unknown device-type tag");
  require_hard_error(
      [=] { roup_acc_contract::builtin_reduction_operator(unknown); },
      "unknown reduction-operator tag");
  require_hard_error(
      [=] { roup_acc_contract::validate_bind_encoding(unknown, ACC_Lang_C); },
      "unknown bind encoding tag");
}

void test_locations_and_typed_data() {
  setLang(ACC_Lang_C);
  OpenACCDirective *raw = parseOpenACC(
      "#pragma acc parallel copyin(always, readonly: a, b[0:n])");
  require(raw != nullptr, "strict parser returned null");
  std::unique_ptr<OpenACCDirective> directive(raw);
  require(directive->getKind() == ACCD_parallel,
          "parallel directive kind was lost");
  require(directive->getBaseLang() == ACC_Lang_C,
          "C host language was lost");
  require(directive->getLine() == 1 && directive->getColumn() == 13,
          "directive source location is not exact");

  auto *clauses = directive->getClauses(ACCC_copyin);
  require(clauses->size() == 1, "copyin clause occurrence was lost");
  auto *copyin = static_cast<OpenACCCopyinClause *>(clauses->front());
  require(copyin->getLine() == 1 && copyin->getColumn() == 22,
          "clause source location is not exact");
  require(copyin->getVariant() == ACCC_DATA_COPYIN_copyin,
          "copyin canonical kind was lost");
  require(copyin->getModifiers().size() == 2 &&
              copyin->getModifiers()[0] == ACCC_DATA_MOD_always &&
              copyin->getModifiers()[1] == ACCC_DATA_MOD_readonly,
          "typed copyin modifiers were not preserved in order");
  require(copyin->getVars().size() == 2 &&
              copyin->getVars()[0].text == "a" &&
              copyin->getVars()[1].text == "b[0:n]",
          "typed copyin variables were not preserved in order");
}

void test_structured_payloads() {
  std::unique_ptr<OpenACCDirective> defaults(
      parseOpenACC("#pragma acc parallel default(none)"));
  auto *default_clause = static_cast<OpenACCDefaultClause *>(
      defaults->getClauses(ACCC_default)->at(0));
  require(default_clause->getKind() == ACCC_DEFAULT_none,
          "numeric OpenACC default kind was not preserved");

  std::unique_ptr<OpenACCDirective> enter_data(parseOpenACC(
      "#pragma acc enter data create(zero: temp) attach(ptr)"));
  auto *create = static_cast<OpenACCCreateClause *>(
      enter_data->getClauses(ACCC_create)->at(0));
  require(create->getVariant() == ACCC_DATA_CREATE_create &&
              create->getModifiers().size() == 1 &&
              create->getModifiers()[0] == ACCC_DATA_MOD_zero &&
              create->getVars().size() == 1 &&
              create->getVars()[0].text == "temp",
          "numeric OpenACC create kind/modifier was not preserved");
  auto *attach = static_cast<OpenACCAttachClause *>(
      enter_data->getClauses(ACCC_attach)->at(0));
  require(attach->getVars().size() == 1 &&
              attach->getVars()[0].text == "ptr",
          "numeric OpenACC data kind was not preserved");

  std::unique_ptr<OpenACCDirective> loop(parseOpenACC(
      "#pragma acc parallel loop collapse(force: 2) "
      "gang(4, dim: 2, static: *) "
      "worker(num: 8) vector(length: 32) tile(4, *)"));
  auto *collapse = static_cast<OpenACCCollapseClause *>(
      loop->getClauses(ACCC_collapse)->at(0));
  require(collapse->isForce() && collapse->getCounts().size() == 1 &&
              collapse->getCounts()[0].text == "2",
          "collapse(force:) was not represented structurally");
  auto *gang = static_cast<OpenACCGangClause *>(
      loop->getClauses(ACCC_gang)->at(0));
  require(gang->getArgs().size() == 3 &&
              gang->getArgs()[0].kind == ACCC_GANG_ARG_num_no_keyword &&
              gang->getArgs()[0].value.text == "4" &&
              gang->getArgs()[1].kind == ACCC_GANG_ARG_dim &&
              gang->getArgs()[1].value.text == "2" &&
              gang->getArgs()[2].kind == ACCC_GANG_ARG_static &&
              gang->getArgs()[2].value.text == "*",
          "ordered gang arguments were not represented structurally");
  auto *worker = static_cast<OpenACCWorkerClause *>(
      loop->getClauses(ACCC_worker)->at(0));
  require(worker->getModifier() == ACCC_WORKER_num &&
              worker->getNumExpr().text == "8",
          "worker modifier/value was not represented structurally");
  auto *vector = static_cast<OpenACCVectorClause *>(
      loop->getClauses(ACCC_vector)->at(0));
  require(vector->getModifier() == ACCC_VECTOR_length &&
              vector->getLengthExpr().text == "32",
          "vector modifier/value was not represented structurally");
  auto *tile = static_cast<OpenACCTileClause *>(
      loop->getClauses(ACCC_tile)->at(0));
  require(tile->getTileSizes().size() == 2 &&
              tile->getTileSizes()[0].text == "4" &&
              tile->getTileSizes()[1].text == "*",
          "typed tile expressions/automatic size were not preserved");

  std::unique_ptr<OpenACCDirective> typed_enums(parseOpenACC(
      "#pragma acc parallel loop reduction(+: total) "
      "device_type(host, gpu)"));
  auto *device_type = static_cast<OpenACCDeviceTypeClause *>(
      typed_enums->getClauses(ACCC_device_type)->at(0));
  require(device_type->getDeviceTypes().size() == 1 &&
              device_type->getDeviceTypes()[0] == ACCC_DEVICE_TYPE_host &&
              device_type->getUnknownDeviceTypes().size() == 1 &&
              device_type->getUnknownDeviceTypes()[0] == "gpu",
          "tagged OpenACC device types were not preserved");
  auto *reduction = static_cast<OpenACCReductionClause *>(
      typed_enums->getClauses(ACCC_reduction)->at(0));
  require(reduction->getOperator() == ACCC_REDUCTION_add,
          "tagged OpenACC reduction operator was not preserved");

  std::unique_ptr<OpenACCDirective> automatic_gang(
      parseOpenACC("#pragma acc loop gang(static: *)"));
  auto *static_gang = static_cast<OpenACCGangClause *>(
      automatic_gang->getClauses(ACCC_gang)->at(0));
  require(static_gang->getArgs().size() == 1 &&
              static_gang->getArgs()[0].kind == ACCC_GANG_ARG_static &&
              static_gang->getArgs()[0].value.text == "*",
          "typed automatic gang size was not preserved");

  std::unique_ptr<OpenACCDirective> wait(parseOpenACC(
      "#pragma acc wait(devnum: device: queues: first, second)"));
  auto *typed_wait = static_cast<OpenACCWaitDirective *>(wait.get());
  require(typed_wait->getDevnum().text == "device" &&
              typed_wait->getQueues() &&
              typed_wait->getAsyncIds().size() == 2 &&
              typed_wait->getAsyncIds()[0].text == "first" &&
              typed_wait->getAsyncIds()[1].text == "second",
          "wait directive parameter was not represented structurally");

  std::unique_ptr<OpenACCDirective> cache(parseOpenACC(
      "#pragma acc cache(readonly: values[index], tile[0:n])"));
  auto *typed_cache = static_cast<OpenACCCacheDirective *>(cache.get());
  require(typed_cache->getModifier() == ACCC_CACHE_readonly &&
              typed_cache->getVars().size() == 2 &&
              typed_cache->getVars()[0].text == "values[index]" &&
              typed_cache->getVars()[1].text == "tile[0:n]",
          "typed cache element/subarray distinction was not consumed");

  std::unique_ptr<OpenACCDirective> routine(
      parseOpenACC("#pragma acc routine(worker_fn)"));
  auto *typed_routine = static_cast<OpenACCRoutineDirective *>(routine.get());
  require(typed_routine->getName().text == "worker_fn",
          "typed routine name was not preserved");

  std::unique_ptr<OpenACCDirective> bare_routine(
      parseOpenACC("#pragma acc routine"));
  require(static_cast<OpenACCRoutineDirective *>(bare_routine.get())
              ->getName()
              .text.empty(),
          "bare routine fabricated a typed name parameter");

  std::unique_ptr<OpenACCDirective> named_binding(
      parseOpenACC("#pragma acc routine bind(device_entry)"));
  auto *typed_name_binding = static_cast<OpenACCBindClause *>(
      named_binding->getClauses(ACCC_bind)->at(0));
  require(typed_name_binding->getBinding().text == "device_entry" &&
              !typed_name_binding->isStringLiteral(),
          "typed bind name target was not preserved");

  std::unique_ptr<OpenACCDirective> string_binding(
      parseOpenACC("#pragma acc routine bind(\"device_entry\")"));
  auto *typed_string_binding = static_cast<OpenACCBindClause *>(
      string_binding->getClauses(ACCC_bind)->at(0));
  require(typed_string_binding->getBinding().text == "device_entry" &&
              typed_string_binding->isStringLiteral(),
          "typed bind string-literal target was not preserved");

  std::unique_ptr<OpenACCDirective> escaped_binding(
      parseOpenACC("#pragma acc routine bind(\"device\\nentry\")"));
  auto *typed_escaped_binding = static_cast<OpenACCBindClause *>(
      escaped_binding->getClauses(ACCC_bind)->at(0));
  require(typed_escaped_binding->getBinding().text == "device\\nentry" &&
              typed_escaped_binding->isStringLiteral(),
          "ordinary bind literal was not re-encoded losslessly");
}

void test_aliases_and_host_languages() {
  std::unique_ptr<OpenACCDirective> alias(
      parseOpenACC("#pragma acc parallel pcopy(a)"));
  require(alias->getClauses(ACCC_copy)->size() == 1,
          "historical pcopy alias was not canonicalized");
  require(alias->generatePragmaString() == "#pragma acc parallel copy(a)",
          "historical pcopy alias did not render canonically");

  setLang(ACC_Lang_Cplusplus);
  std::unique_ptr<OpenACCDirective> cpp(
      parseOpenACC("#pragma acc parallel if(ns::ready)"));
  require(cpp->getBaseLang() == ACC_Lang_Cplusplus,
          "C++ language selection was lost");

  require_hard_error([] { parseOpenACC("!$acc parallel if(flag)"); },
                     "Fortran source under a C++ profile");

  setLang(ACC_Lang_Fortran);

  std::unique_ptr<OpenACCDirective> fortran(
      parseOpenACC("!$acc parallel if(flag)"));
  require(fortran->getBaseLang() == ACC_Lang_Fortran,
          "explicit Fortran sentinel was not classified");

  std::unique_ptr<OpenACCDirective> fortran_binding(
      parseOpenACC("!$acc routine bind('device_entry')"));
  auto *typed_fortran_binding = static_cast<OpenACCBindClause *>(
      fortran_binding->getClauses(ACCC_bind)->at(0));
  require(typed_fortran_binding->getBinding().text == "device_entry" &&
              typed_fortran_binding->isStringLiteral(),
          "Fortran bind literal encoding was not preserved");

  std::unique_ptr<OpenACCDirective> common_block(
      parseOpenACC("!$acc declare copyin(/STATE/)"));
  auto *common_copyin = static_cast<OpenACCCopyinClause *>(
      common_block->getClauses(ACCC_copyin)->at(0));
  require(common_copyin->getVars().size() == 1 &&
              common_copyin->getVars()[0].text == "/state/",
          "typed Fortran common-block clause item was not preserved");

  std::unique_ptr<OpenACCDirective> end(
      parseOpenACC("!$acc end PARALLEL LOOP"));
  auto *typed_end = static_cast<OpenACCEndDirective *>(end.get());
  require(typed_end->getPairedDirective() != nullptr &&
              typed_end->getPairedDirective()->getKind() == ACCD_parallel_loop,
          "typed OpenACC end kind was not preserved");
  require_hard_error([] { parseOpenACC("!$acc end WAIT"); },
                     "non-pairable OpenACC end kind");
  require_hard_error([] { parseOpenACC("#pragma acc parallel"); },
                     "C pragma under a Fortran profile");

  setLang(ACC_Lang_C);

  std::unique_ptr<OpenACCDirective> host_alias(
      parseOpenACC("#pragma acc update host(a)"));
  require(host_alias->getClauses(ACCC_host)->empty() &&
              host_alias->getClauses(ACCC_self)->size() == 1,
          "update host did not arrive as the canonical self kind");
  auto *host_as_self = static_cast<OpenACCSelfClause *>(
      host_alias->getClauses(ACCC_self)->at(0));
  require(host_as_self->getVars().size() == 1 &&
              host_as_self->getVars()[0].text == "a" &&
              host_alias->generatePragmaString() ==
                  "#pragma acc update self(a)",
          "update host did not preserve the canonical self payload");

  std::unique_ptr<OpenACCDirective> canonical_self(
      parseOpenACC("#pragma acc update self(a)"));
  require(canonical_self->getClauses(ACCC_self)->size() == 1 &&
              canonical_self->getClauses(ACCC_host)->empty(),
          "update self did not retain its canonical semantic kind");
}

void test_failures_are_exceptions() {
  require_hard_error([] { parseOpenACC(""); }, "empty input");
  require_hard_error([] { parseOpenACC("parallel"); },
                     "prefix-free input");
  require_hard_error([] { parseOpenACC("#pragma acc parallel copy() "); },
                     "empty variable list");
  require_hard_error(
      [] { parseOpenACC("#pragma acc parallel typo_clause(value)"); },
      "unknown clause");
  require_hard_error([] { parseOpenACC("#pragma acc parallel bind()"); },
                     "empty expression");
  require_hard_error([] { parseOpenACC("#pragma acc cache(scalar)"); },
                     "scalar cache item");
  require_hard_error(
      [] { parseOpenACC("#pragma acc cache(values[0:n:2])"); },
      "strided cache subarray");
  require_hard_error([] { parseOpenACC("#pragma acc routine()"); },
                     "empty routine name");
  require_hard_error(
      [] { parseOpenACC("#pragma acc loop worker(first, second)"); },
      "multiple worker expressions");
  require_hard_error(
      [] { parseOpenACC("#pragma acc loop vector(length: first, second)"); },
      "multiple vector expressions");
  require_hard_error(
      [] { parseOpenACC("#pragma acc loop device_type(gpu, host)"); },
      "unrepresentable interleaved device-type order");
  require_hard_error(
      [] { parseOpenACC("#pragma acc loop device_type(host, host)"); },
      "device-type deduplication");
  require_hard_error([] { parseOpenACC("#pragma acc routine gang(4)"); },
                     "positional gang argument on routine");
  require_hard_error(
      [] { parseOpenACC("#pragma acc routine gang(num: 4)"); },
      "gang num argument on routine");
  require_hard_error(
      [] { parseOpenACC("#pragma acc routine gang(static: 4)"); },
      "gang static argument on routine");
  require_hard_error([] { parseOpenACC("#pragma acc routine worker(4)"); },
                     "worker argument on routine");
  require_hard_error([] { parseOpenACC("#pragma acc routine vector(4)"); },
                     "vector argument on routine");
  require_hard_error([] { parseOpenACC("#pragma acc routine bind(a + b)"); },
                     "arbitrary bind expression");
  require_hard_error(
      [] { parseOpenACC("#pragma acc routine bind(u8\"device_entry\")"); },
      "unrepresentable prefixed bind string literal");
  require_hard_error([] { setLang(ACC_Lang_unknown); },
                     "unknown base language");
}

} // namespace

int main() {
  try {
    test_language_selection_is_mandatory();
    test_numeric_contract_rejects_unknown_tags();
    test_locations_and_typed_data();
    test_structured_payloads();
    test_aliases_and_host_languages();
    test_failures_are_exceptions();
    std::cout << "strict OpenACC adapter contract: OK\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
