#include <OpenACCIR.h>
#include <iostream>
#include <fstream>
#include <string>

extern "C" {
    OpenACCDirective* parseOpenACC(const char* input, void* exprParse);
    void setLang(OpenACCBaseLang lang);
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <input_file>" << std::endl;
        return 1;
    }

    std::ifstream infile(argv[1]);
    if (!infile.is_open()) {
        std::cerr << "Could not open file: " << argv[1] << std::endl;
        return 1;
    }

    // Extract filename from path for output file
    std::string filename_string = std::string(argv[1]);
    size_t pos = filename_string.rfind("/");
    if (pos != std::string::npos) {
        filename_string = filename_string.substr(pos + 1);
    }

    // Open output file (filename.output)
    std::string output_filename = filename_string + ".output";
    std::ofstream output_file(output_filename.c_str(), std::ofstream::trunc);
    if (!output_file.is_open()) {
        std::cerr << "Could not create output file: " << output_filename << std::endl;
        return 1;
    }

    std::string line;
    while (std::getline(infile, line)) {
        if (line.empty()) continue;

        // Detect language from input
        if (line.find("!$acc") == 0 || line.find("!$ACC") == 0) {
            setLang(ACC_Lang_Fortran);
        } else {
            setLang(ACC_Lang_C);
        }

        OpenACCDirective* dir = parseOpenACC(line.c_str(), nullptr);
        if (dir) {
            std::string output = dir->generatePragmaString();
            output_file << output << std::endl;
            delete dir;
        }
    }

    output_file.close();
    return 0;
}
