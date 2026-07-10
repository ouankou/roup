#include "roup.h"

#include <cstdint>
#include <cstdlib>
#include <exception>
#include <iostream>
#include <stdexcept>
#include <string>

class RoupFailure final : public std::runtime_error {
public:
    explicit RoupFailure(RoupStatus status)
        : std::runtime_error("ROUP operation failed with status " +
                             std::to_string(status)) {}
};

static void require_ok(const RoupCallResult &result) {
    if (result.status != ROUP_STATUS_OK) {
        if (result.error.generation != 0U) {
            const RoupCallResult released = roup_error_release(result.error);
            if (released.status != ROUP_STATUS_OK) {
                std::terminate();
            }
        }
        throw RoupFailure(result.status);
    }
}

int main() try {
    RoupParserOptions options{};
    options.abi_version = ROUP_ABI_VERSION;
    options.struct_size = static_cast<std::uint32_t>(sizeof(options));
    options.dialect = ROUP_DIALECT_OPENMP;
    options.version_policy = ROUP_VERSION_EXACT;
    options.version = ROUP_OMP_VERSION_6_0;
    options.host_language = ROUP_HOST_CPP;
    options.host_standard = ROUP_CPP_23;
    options.source_form = ROUP_SOURCE_PRAGMA;

    const RoupParserResult parser = roup_parser_create(options);
    require_ok(parser.result);

    constexpr std::uint8_t source[] = "#pragma omp master";
    const RoupDirectiveResult directive =
        roup_parse(parser.value, source, sizeof(source) - 1U);
    require_ok(directive.result);

    const RoupU64Result versions =
        roup_directive_compatible_versions(directive.value);
    require_ok(versions.result);
    if ((versions.value & ROUP_OMP_VERSION_BIT_6_0) == 0U) {
        throw std::runtime_error(
            "historical OpenMP syntax was not retained in 6.0 mode");
    }

    std::cout << "historical OpenMP syntax parsed into a typed directive\n";

    require_ok(roup_directive_release(directive.value));
    require_ok(roup_parser_release(parser.value));
    return EXIT_SUCCESS;
} catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return EXIT_FAILURE;
}
