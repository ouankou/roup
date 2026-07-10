/*
 * Strict fixture runner for the pinned ompparser text corpus.
 *
 * Unlike the upstream runner, this treats a thrown parse exception as the
 * required result for `EXPECT: invalid` cases. A hard parse error therefore
 * remains observable and cannot abort the remainder of the corpus.
 */

#include <OpenMPIR.h>

#include <algorithm>
#include <cctype>
#include <fstream>
#include <iostream>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>

namespace {

struct PendingValidation {
  std::string source;
  std::optional<std::string> output;
  std::optional<std::string> error;
  std::size_t line;
  bool expected_invalid;
};

std::string trim(std::string value) {
  const auto first = std::find_if_not(value.begin(), value.end(),
                                      [](unsigned char character) {
                                        return std::isspace(character) != 0;
                                      });
  const auto last = std::find_if_not(value.rbegin(), value.rend(),
                                     [](unsigned char character) {
                                       return std::isspace(character) != 0;
                                     })
                        .base();
  if (first >= last)
    return {};
  return std::string(first, last);
}

std::string lowercase(std::string value) {
  std::transform(value.begin(), value.end(), value.begin(),
                 [](unsigned char character) {
                   return static_cast<char>(std::tolower(character));
                 });
  return value;
}

bool is_fortran_directive(const std::string &line) {
  const std::string lowered = lowercase(line);
  return lowered.rfind("!$omp", 0) == 0 ||
         lowered.rfind("c$omp", 0) == 0 ||
         lowered.rfind("*$omp", 0) == 0;
}

bool is_directive(const std::string &line) {
  return line.rfind("#pragma", 0) == 0 || is_fortran_directive(line);
}

void finish_without_pass(const PendingValidation &pending, int &invalid_passes,
                         int &failures) {
  if (pending.expected_invalid) {
    if (pending.error.has_value()) {
      ++invalid_passes;
    } else {
      std::cerr << "line " << pending.line
                << ": expected a hard parse error for `" << pending.source
                << "`, got `" << *pending.output << "`\n";
      ++failures;
    }
    return;
  }
  std::cerr << "line " << pending.line << ": missing PASS validation for `"
            << pending.source << "`\n";
  if (pending.error.has_value())
    std::cerr << "  parse error: " << *pending.error << '\n';
  ++failures;
}

} // namespace

int main(int argc, const char *argv[]) {
  OpenMPBaseLang default_language = Lang_C;
  const char *filename = nullptr;
  for (int index = 1; index < argc; ++index) {
    const std::string argument = argv[index];
    if (argument == "--lang=c") {
      default_language = Lang_C;
    } else if (argument == "--lang=c++" || argument == "--lang=cpp" ||
               argument == "--lang=cxx") {
      default_language = Lang_Cplusplus;
    } else if (argument == "--lang=fortran") {
      default_language = Lang_Fortran;
    } else if (filename == nullptr) {
      filename = argv[index];
    } else {
      std::cerr << "unexpected argument: " << argument << '\n';
      return 2;
    }
  }
  if (filename == nullptr) {
    std::cerr << "fixture path is required\n";
    return 2;
  }

  std::ifstream input(filename);
  if (!input) {
    std::cerr << "cannot open fixture: " << filename << '\n';
    return 2;
  }

  constexpr const char *invalid_marker =
      "invalid test without paired validation.";
  bool expect_invalid = false;
  std::optional<PendingValidation> pending;
  int valid_passes = 0;
  int invalid_passes = 0;
  int failures = 0;
  std::size_t line_number = 0;
  std::string raw_line;

  while (std::getline(input, raw_line)) {
    ++line_number;
    std::string line = trim(raw_line);
    if (line.rfind("EXPECT:", 0) == 0) {
      const std::string expectation = lowercase(trim(line.substr(7)));
      if (expectation != "invalid" && expectation != "fail" &&
          expectation != "failure") {
        std::cerr << "line " << line_number
                  << ": unsupported expectation `" << expectation << "`\n";
        ++failures;
      }
      expect_invalid = true;
      continue;
    }

    const std::size_t invalid_comment = line.find(invalid_marker);
    if (invalid_comment != std::string::npos) {
      const std::size_t comment = line.rfind("//", invalid_comment);
      if (comment == std::string::npos) {
        std::cerr << "line " << line_number
                  << ": invalid marker is not in a comment\n";
        ++failures;
        continue;
      }
      const std::string before_comment = trim(line.substr(0, comment));
      expect_invalid = true;
      if (before_comment.empty())
        continue;
      line = before_comment;
    }

    if (is_directive(line)) {
      if (pending.has_value()) {
        finish_without_pass(*pending, invalid_passes, failures);
        pending.reset();
      }

      setLang(is_fortran_directive(line) ? Lang_Fortran : default_language);
      std::optional<std::string> output;
      std::optional<std::string> error;
      try {
        std::unique_ptr<OpenMPDirective> directive(
            parseOpenMP(line.c_str(), nullptr, nullptr));
        if (!directive) {
          throw std::runtime_error(
              "strict parser returned null without a diagnostic");
        }
        output = directive->generatePragmaString();
      } catch (const std::exception &exception) {
        error = exception.what();
      }

      pending = PendingValidation{line, output, error, line_number,
                                  expect_invalid};
      expect_invalid = false;
      continue;
    }

    if (line.rfind("PASS:", 0) == 0) {
      if (!pending.has_value()) {
        std::cerr << "line " << line_number
                  << ": PASS has no preceding valid directive\n";
        ++failures;
        continue;
      }
      const std::string expected = trim(line.substr(5));
      if (pending->error.has_value()) {
        std::cerr << "line " << pending->line << ": valid input `"
                  << pending->source << "` was rejected\n  parse error: "
                  << *pending->error << '\n';
        ++failures;
      } else if (*pending->output != expected) {
        std::cerr << "line " << pending->line << ": output mismatch for `"
                  << pending->source << "`\n  actual:   `" << *pending->output
                  << "`\n  expected: `" << expected << "`\n";
        ++failures;
      } else {
        ++valid_passes;
      }
      pending.reset();
    }
  }

  if (pending.has_value())
    finish_without_pass(*pending, invalid_passes, failures);
  if (expect_invalid) {
    std::cerr << "fixture ended before the expected-invalid directive\n";
    ++failures;
  }
  if (valid_passes + invalid_passes == 0) {
    std::cerr << "fixture contains no executable expectations\n";
    ++failures;
  }

  std::cout << "valid passes: " << valid_passes
            << ", hard-error passes: " << invalid_passes
            << ", failures: " << failures << '\n';
  return failures == 0 ? 0 : 1;
}
