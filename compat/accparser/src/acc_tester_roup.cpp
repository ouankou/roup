#include <OpenACCParser.h>

#include <cctype>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>

extern "C" void setLang(OpenACCBaseLang lang);

namespace {

bool starts_with_case_insensitive(const std::string &value,
                                  const std::string &prefix) {
  if (value.size() < prefix.size())
    return false;
  for (std::size_t index = 0; index < prefix.size(); ++index) {
    if (std::tolower(static_cast<unsigned char>(value[index])) !=
        std::tolower(static_cast<unsigned char>(prefix[index]))) {
      return false;
    }
  }
  return true;
}

OpenACCBaseLang source_language(const std::string &line) {
  const std::size_t first = line.find_first_not_of(" \t\r\n");
  if (first == std::string::npos)
    throw std::runtime_error("fixture line must not be blank");
  const std::string directive = line.substr(first);
  if (starts_with_case_insensitive(directive, "!$acc") ||
      starts_with_case_insensitive(directive, "c$acc") ||
      starts_with_case_insensitive(directive, "*$acc")) {
    return ACC_Lang_Fortran;
  }
  if (starts_with_case_insensitive(directive, "#pragma"))
    return ACC_Lang_C;
  throw std::runtime_error(
      "fixture line has no explicit OpenACC pragma or Fortran sentinel");
}

} // namespace

int main(int argc, char *argv[]) {
  try {
    if (argc != 2)
      throw std::runtime_error("usage: acc_tester.out <input_file>");

    std::ifstream input(argv[1]);
    if (!input.is_open())
      throw std::runtime_error(std::string("could not open fixture: ") +
                               argv[1]);

    std::string filename(argv[1]);
    const std::size_t slash = filename.find_last_of('/');
    if (slash != std::string::npos)
      filename = filename.substr(slash + 1);
    std::ofstream output(filename + ".output", std::ofstream::trunc);
    if (!output.is_open())
      throw std::runtime_error("could not create fixture output");

    std::string line;
    while (std::getline(input, line)) {
      if (line.find_first_not_of(" \t\r\n") == std::string::npos)
        continue;
      setLang(source_language(line));
      std::unique_ptr<OpenACCDirective> directive(parseOpenACC(line));
      if (!directive)
        throw std::runtime_error("strict parser returned null");
      output << directive->generatePragmaString() << '\n';
    }
    if (!output)
      throw std::runtime_error("failed to write fixture output");
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
