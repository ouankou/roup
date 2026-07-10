if(NOT DEFINED ADAPTER_SOURCE)
    message(FATAL_ERROR "ADAPTER_SOURCE is required")
endif()

file(READ "${ADAPTER_SOURCE}" ADAPTER_TEXT)

set(FORBIDDEN_ENUM_STRING_PATHS
    "directive_name("
    "clause_name("
    "required_string(ROUP_FIELD_KIND)"
    "required_string(ROUP_FIELD_MODIFIER)"
    "optional_string(ROUP_FIELD_MODIFIER)"
    "required_strings(ROUP_FIELD_MODIFIERS)"
    "required_string(ROUP_FIELD_OPERATOR)")

foreach(FORBIDDEN IN LISTS FORBIDDEN_ENUM_STRING_PATHS)
    string(FIND "${ADAPTER_TEXT}" "${FORBIDDEN}" POSITION)
    if(NOT POSITION EQUAL -1)
        message(FATAL_ERROR
            "OpenACC adapter reintroduced string-based enum classification: ${FORBIDDEN}")
    endif()
endforeach()

foreach(REQUIRED IN ITEMS
        "roup_acc_contract::directive_kind"
        "roup_acc_contract::clause_kind"
        "required_u32(ROUP_FIELD_KIND)"
        "required_u32s(ROUP_FIELD_MODIFIERS)"
        "read_acc_device_types"
        "read_acc_reduction_operator")
    string(FIND "${ADAPTER_TEXT}" "${REQUIRED}" POSITION)
    if(POSITION EQUAL -1)
        message(FATAL_ERROR
            "OpenACC adapter is missing required typed enum conversion: ${REQUIRED}")
    endif()
endforeach()
