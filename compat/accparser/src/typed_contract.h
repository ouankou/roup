#ifndef ROUP_ACCPARSER_TYPED_CONTRACT_H
#define ROUP_ACCPARSER_TYPED_CONTRACT_H

#include <OpenACCParser.h>
#include <roup.h>

#include <array>
#include <cstdint>
#include <stdexcept>
#include <string>

namespace roup_acc_contract {

inline constexpr std::array<OpenACCDirectiveKind, 21> DIRECTIVE_KINDS = {
    ACCD_atomic,        ACCD_cache,         ACCD_data,
    ACCD_declare,       ACCD_end,           ACCD_enter_data,
    ACCD_exit_data,     ACCD_host_data,     ACCD_init,
    ACCD_kernels,       ACCD_kernels_loop,  ACCD_loop,
    ACCD_parallel,      ACCD_parallel_loop, ACCD_routine,
    ACCD_serial,        ACCD_serial_loop,   ACCD_set,
    ACCD_shutdown,      ACCD_update,        ACCD_wait,
};

static_assert(ROUP_ACC_DIRECTIVE_ATOMIC == 0 &&
                  ROUP_ACC_DIRECTIVE_CACHE == 1 &&
                  ROUP_ACC_DIRECTIVE_DATA == 2 &&
                  ROUP_ACC_DIRECTIVE_DECLARE == 3 &&
                  ROUP_ACC_DIRECTIVE_END == 4 &&
                  ROUP_ACC_DIRECTIVE_ENTER_DATA == 5 &&
                  ROUP_ACC_DIRECTIVE_EXIT_DATA == 6 &&
                  ROUP_ACC_DIRECTIVE_HOST_DATA == 7 &&
                  ROUP_ACC_DIRECTIVE_INIT == 8 &&
                  ROUP_ACC_DIRECTIVE_KERNELS == 9 &&
                  ROUP_ACC_DIRECTIVE_KERNELS_LOOP == 10 &&
                  ROUP_ACC_DIRECTIVE_LOOP == 11 &&
                  ROUP_ACC_DIRECTIVE_PARALLEL == 12 &&
                  ROUP_ACC_DIRECTIVE_PARALLEL_LOOP == 13 &&
                  ROUP_ACC_DIRECTIVE_ROUTINE == 14 &&
                  ROUP_ACC_DIRECTIVE_SERIAL == 15 &&
                  ROUP_ACC_DIRECTIVE_SERIAL_LOOP == 16 &&
                  ROUP_ACC_DIRECTIVE_SET == 17 &&
                  ROUP_ACC_DIRECTIVE_SHUTDOWN == 18 &&
                  ROUP_ACC_DIRECTIVE_UPDATE == 19 &&
                  ROUP_ACC_DIRECTIVE_WAIT == 20,
              "OpenACC directive ordinal schema changed");

inline constexpr std::array<OpenACCClauseKind, 44> CLAUSE_KINDS = {
    ACCC_async,           ACCC_attach,       ACCC_auto,
    ACCC_bind,            ACCC_capture,      ACCC_collapse,
    ACCC_copy,            ACCC_copyin,       ACCC_copyout,
    ACCC_create,          ACCC_default,      ACCC_default_async,
    ACCC_delete,          ACCC_detach,       ACCC_device,
    ACCC_device_num,      ACCC_device_resident,
    ACCC_device_type,     ACCC_deviceptr,    ACCC_finalize,
    ACCC_firstprivate,    ACCC_gang,         ACCC_if,
    ACCC_if_present,      ACCC_independent,  ACCC_link,
    ACCC_no_create,       ACCC_nohost,       ACCC_num_gangs,
    ACCC_num_workers,     ACCC_present,      ACCC_private,
    ACCC_reduction,       ACCC_read,         ACCC_self,
    ACCC_seq,             ACCC_tile,         ACCC_update,
    ACCC_use_device,      ACCC_vector,       ACCC_vector_length,
    ACCC_wait,            ACCC_worker,       ACCC_write,
};

static_assert(ROUP_ACC_CLAUSE_ASYNC == 0 &&
                  ROUP_ACC_CLAUSE_ATTACH == 1 &&
                  ROUP_ACC_CLAUSE_AUTO == 2 &&
                  ROUP_ACC_CLAUSE_BIND == 3 &&
                  ROUP_ACC_CLAUSE_CAPTURE == 4 &&
                  ROUP_ACC_CLAUSE_COLLAPSE == 5 &&
                  ROUP_ACC_CLAUSE_COPY == 6 &&
                  ROUP_ACC_CLAUSE_COPY_IN == 7 &&
                  ROUP_ACC_CLAUSE_COPY_OUT == 8 &&
                  ROUP_ACC_CLAUSE_CREATE == 9 &&
                  ROUP_ACC_CLAUSE_DEFAULT == 10 &&
                  ROUP_ACC_CLAUSE_DEFAULT_ASYNC == 11 &&
                  ROUP_ACC_CLAUSE_DELETE == 12 &&
                  ROUP_ACC_CLAUSE_DETACH == 13 &&
                  ROUP_ACC_CLAUSE_DEVICE == 14 &&
                  ROUP_ACC_CLAUSE_DEVICE_NUM == 15 &&
                  ROUP_ACC_CLAUSE_DEVICE_RESIDENT == 16 &&
                  ROUP_ACC_CLAUSE_DEVICE_TYPE == 17 &&
                  ROUP_ACC_CLAUSE_DEVICE_PTR == 18 &&
                  ROUP_ACC_CLAUSE_FINALIZE == 19 &&
                  ROUP_ACC_CLAUSE_FIRSTPRIVATE == 20 &&
                  ROUP_ACC_CLAUSE_GANG == 21 &&
                  ROUP_ACC_CLAUSE_IF == 22 &&
                  ROUP_ACC_CLAUSE_IF_PRESENT == 23 &&
                  ROUP_ACC_CLAUSE_INDEPENDENT == 24 &&
                  ROUP_ACC_CLAUSE_LINK == 25 &&
                  ROUP_ACC_CLAUSE_NO_CREATE == 26 &&
                  ROUP_ACC_CLAUSE_NO_HOST == 27 &&
                  ROUP_ACC_CLAUSE_NUM_GANGS == 28 &&
                  ROUP_ACC_CLAUSE_NUM_WORKERS == 29 &&
                  ROUP_ACC_CLAUSE_PRESENT == 30 &&
                  ROUP_ACC_CLAUSE_PRIVATE == 31 &&
                  ROUP_ACC_CLAUSE_REDUCTION == 32 &&
                  ROUP_ACC_CLAUSE_READ == 33 &&
                  ROUP_ACC_CLAUSE_SELF_CLAUSE == 34 &&
                  ROUP_ACC_CLAUSE_SEQ == 35 &&
                  ROUP_ACC_CLAUSE_TILE == 36 &&
                  ROUP_ACC_CLAUSE_UPDATE == 37 &&
                  ROUP_ACC_CLAUSE_USE_DEVICE == 38 &&
                  ROUP_ACC_CLAUSE_VECTOR == 39 &&
                  ROUP_ACC_CLAUSE_VECTOR_LENGTH == 40 &&
                  ROUP_ACC_CLAUSE_WAIT == 41 &&
                  ROUP_ACC_CLAUSE_WORKER == 42 &&
                  ROUP_ACC_CLAUSE_WRITE == 43,
              "OpenACC clause ordinal schema changed");

inline OpenACCDirectiveKind directive_kind(RoupDirectiveKind kind) {
  if (kind.dialect != ROUP_DIALECT_OPENACC) {
    throw std::runtime_error("OpenACC adapter received a foreign directive kind");
  }
  if (kind.ordinal >= DIRECTIVE_KINDS.size()) {
    throw std::runtime_error("unknown typed OpenACC directive ordinal " +
                             std::to_string(kind.ordinal));
  }
  return DIRECTIVE_KINDS[kind.ordinal];
}

inline OpenACCClauseKind clause_kind(RoupClauseKind kind) {
  if (kind.dialect != ROUP_DIALECT_OPENACC) {
    throw std::runtime_error("OpenACC adapter received a foreign clause kind");
  }
  if (kind.ordinal >= CLAUSE_KINDS.size()) {
    throw std::runtime_error("unknown typed OpenACC clause ordinal " +
                             std::to_string(kind.ordinal));
  }
  return CLAUSE_KINDS[kind.ordinal];
}

inline OpenACCDefaultClauseKind default_kind(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_DEFAULT_NONE:
    return ACCC_DEFAULT_none;
  case ROUP_ACC_DEFAULT_PRESENT:
    return ACCC_DEFAULT_present;
  default:
    throw std::runtime_error("unknown typed OpenACC default-kind tag " +
                             std::to_string(value));
  }
}

inline OpenACCDataClauseModifierKind data_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_DATA_MODIFIER_ALWAYS:
    return ACCC_DATA_MOD_always;
  case ROUP_ACC_DATA_MODIFIER_ALWAYS_IN:
    return ACCC_DATA_MOD_alwaysin;
  case ROUP_ACC_DATA_MODIFIER_ALWAYS_OUT:
    return ACCC_DATA_MOD_alwaysout;
  case ROUP_ACC_DATA_MODIFIER_CAPTURE:
    return ACCC_DATA_MOD_capture;
  case ROUP_ACC_DATA_MODIFIER_READONLY:
    return ACCC_DATA_MOD_readonly;
  case ROUP_ACC_DATA_MODIFIER_ZERO:
    return ACCC_DATA_MOD_zero;
  default:
    throw std::runtime_error("unknown typed OpenACC data-modifier tag " +
                             std::to_string(value));
  }
}

inline OpenACCDataClauseVariant copy_variant(std::uint32_t value,
                                             OpenACCClauseKind clause) {
  if (value == ROUP_ACC_COPY && clause == ACCC_copy)
    return ACCC_DATA_COPY_copy;
  if (value == ROUP_ACC_COPYIN && clause == ACCC_copyin)
    return ACCC_DATA_COPYIN_copyin;
  if (value == ROUP_ACC_COPYOUT && clause == ACCC_copyout)
    return ACCC_DATA_COPYOUT_copyout;
  throw std::runtime_error("typed OpenACC copy-kind tag disagrees with its clause");
}

inline OpenACCDataClauseVariant create_variant(std::uint32_t value) {
  if (value != ROUP_ACC_CREATE) {
    throw std::runtime_error("unknown typed OpenACC create-kind tag " +
                             std::to_string(value));
  }
  return ACCC_DATA_CREATE_create;
}

inline OpenACCClauseKind data_clause_kind(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_DATA_ATTACH:
    return ACCC_attach;
  case ROUP_ACC_DATA_DETACH:
    return ACCC_detach;
  case ROUP_ACC_DATA_USE_DEVICE:
    return ACCC_use_device;
  case ROUP_ACC_DATA_LINK:
    return ACCC_link;
  case ROUP_ACC_DATA_DEVICE_RESIDENT:
    return ACCC_device_resident;
  case ROUP_ACC_DATA_DEVICE:
    return ACCC_device;
  case ROUP_ACC_DATA_DELETE:
    return ACCC_delete;
  default:
    throw std::runtime_error("unknown typed OpenACC data-kind tag " +
                             std::to_string(value));
  }
}

inline OpenACCWorkerClauseModifier worker_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_WORKER_NUM:
    return ACCC_WORKER_num;
  case ROUP_ACC_WORKER_EXPRESSION:
    return ACCC_WORKER_expr_only;
  default:
    throw std::runtime_error("unknown typed OpenACC worker-modifier tag " +
                             std::to_string(value));
  }
}

inline OpenACCVectorClauseModifier vector_modifier(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_VECTOR_LENGTH:
    return ACCC_VECTOR_length;
  case ROUP_ACC_VECTOR_EXPRESSION:
    return ACCC_VECTOR_expr_only;
  default:
    throw std::runtime_error("unknown typed OpenACC vector-modifier tag " +
                             std::to_string(value));
  }
}

inline OpenACCDeviceTypeKind builtin_device_type(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_DEVICE_TYPE_HOST:
    return ACCC_DEVICE_TYPE_host;
  case ROUP_ACC_DEVICE_TYPE_WILDCARD:
    return ACCC_DEVICE_TYPE_any;
  case ROUP_ACC_DEVICE_TYPE_MULTICORE:
    return ACCC_DEVICE_TYPE_multicore;
  case ROUP_ACC_DEVICE_TYPE_DEFAULT:
    return ACCC_DEVICE_TYPE_default;
  default:
    throw std::runtime_error("unknown typed OpenACC builtin device-type tag " +
                             std::to_string(value));
  }
}

inline OpenACCReductionClauseOperator
builtin_reduction_operator(std::uint32_t value) {
  switch (value) {
  case ROUP_ACC_REDUCTION_ADD:
    return ACCC_REDUCTION_add;
  case ROUP_ACC_REDUCTION_MUL:
    return ACCC_REDUCTION_mul;
  case ROUP_ACC_REDUCTION_MAX:
    return ACCC_REDUCTION_max;
  case ROUP_ACC_REDUCTION_MIN:
    return ACCC_REDUCTION_min;
  case ROUP_ACC_REDUCTION_BIT_AND:
    return ACCC_REDUCTION_bitand;
  case ROUP_ACC_REDUCTION_BIT_OR:
    return ACCC_REDUCTION_bitor;
  case ROUP_ACC_REDUCTION_BIT_XOR:
    return ACCC_REDUCTION_bitxor;
  case ROUP_ACC_REDUCTION_LOG_AND:
    return ACCC_REDUCTION_logand;
  case ROUP_ACC_REDUCTION_LOG_OR:
    return ACCC_REDUCTION_logor;
  case ROUP_ACC_REDUCTION_FORTRAN_AND:
    return ACCC_REDUCTION_fort_and;
  case ROUP_ACC_REDUCTION_FORTRAN_OR:
    return ACCC_REDUCTION_fort_or;
  case ROUP_ACC_REDUCTION_FORTRAN_EQV:
    return ACCC_REDUCTION_fort_eqv;
  case ROUP_ACC_REDUCTION_FORTRAN_NEQV:
    return ACCC_REDUCTION_fort_neqv;
  case ROUP_ACC_REDUCTION_FORTRAN_IAND:
    return ACCC_REDUCTION_fort_iand;
  case ROUP_ACC_REDUCTION_FORTRAN_IOR:
    return ACCC_REDUCTION_fort_ior;
  case ROUP_ACC_REDUCTION_FORTRAN_IEOR:
    return ACCC_REDUCTION_fort_ieor;
  default:
    throw std::runtime_error("unknown typed OpenACC reduction-operator tag " +
                             std::to_string(value));
  }
}

inline void validate_bind_encoding(std::uint32_t encoding,
                                   OpenACCBaseLang language) {
  if ((language == ACC_Lang_C || language == ACC_Lang_Cplusplus) &&
      encoding == ROUP_CHARACTER_ENCODING_ORDINARY) {
    return;
  }
  if (language == ACC_Lang_Fortran &&
      encoding == ROUP_CHARACTER_ENCODING_FORTRAN) {
    return;
  }
  switch (encoding) {
  case ROUP_CHARACTER_ENCODING_ORDINARY:
  case ROUP_CHARACTER_ENCODING_UTF8:
  case ROUP_CHARACTER_ENCODING_UTF16:
  case ROUP_CHARACTER_ENCODING_UTF32:
  case ROUP_CHARACTER_ENCODING_WIDE:
  case ROUP_CHARACTER_ENCODING_FORTRAN:
    throw std::runtime_error(
        "accparser cannot preserve this OpenACC bind string-literal encoding");
  default:
    throw std::runtime_error("unknown typed character-encoding tag " +
                             std::to_string(encoding));
  }
}

} // namespace roup_acc_contract

#endif // ROUP_ACCPARSER_TYPED_CONTRACT_H
