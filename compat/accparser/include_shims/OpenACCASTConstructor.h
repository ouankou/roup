#pragma once

#include "OpenACCIR.h"
#include <string>

class OpenACCIRConstructor {
public:
    virtual ~OpenACCIRConstructor() = default;
};

OpenACCDirective* parseOpenACC(std::string input);
