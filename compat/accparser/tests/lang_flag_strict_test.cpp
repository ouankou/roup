#include <OpenACCParser.h>

#include <iostream>
#include <memory>
#include <stdexcept>

extern "C" void setLang(OpenACCBaseLang);

namespace {

void require(bool condition, const char *message) {
  if (!condition)
    throw std::runtime_error(message);
}

} // namespace

int main() {
  try {
    setLang(ACC_Lang_C);
    std::unique_ptr<OpenACCDirective> c(
        parseOpenACC("#pragma acc parallel"));
    require(c != nullptr, "C parse returned null");
    require(c->getBaseLang() == ACC_Lang_C, "C profile was not preserved");

    setLang(ACC_Lang_Cplusplus);
    std::unique_ptr<OpenACCDirective> cpp(
        parseOpenACC("#pragma acc parallel if(ns::ready)"));
    require(cpp != nullptr, "C++ parse returned null");
    require(cpp->getBaseLang() == ACC_Lang_Cplusplus,
            "C++ profile was not preserved");

    setLang(ACC_Lang_Fortran);
    std::unique_ptr<OpenACCDirective> fortran(
        parseOpenACC("!$acc parallel"));
    require(fortran != nullptr, "Fortran parse returned null");
    require(fortran->getBaseLang() == ACC_Lang_Fortran,
            "Fortran profile was not preserved");

    std::cout << "strict lang_flag behavior: OK\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
