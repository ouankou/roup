#include <OpenACCParser.h>

#include <iostream>
#include <memory>
#include <stdexcept>

extern "C" void setLang(OpenACCBaseLang);

int main() {
  try {
    setLang(ACC_Lang_C);
    std::unique_ptr<OpenACCDirective> directive(
        parseOpenACC("#pragma acc parallel"));
    if (!directive)
      throw std::runtime_error("parseOpenACC returned null");
    if (directive->getKind() != ACCD_parallel)
      throw std::runtime_error("unexpected directive kind");
    if (directive->getBaseLang() != ACC_Lang_C)
      throw std::runtime_error("unexpected base language");
    std::cout << "strict compatibility caller: OK\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
