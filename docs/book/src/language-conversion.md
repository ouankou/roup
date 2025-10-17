# Directive Language Conversion

ROUP can now translate parsed OpenMP pragmas between C/C++ and Fortran syntax.
This is useful when a project needs to consume pragmas from one language front
end (for example, Clang) and emit equivalent directives for another tool chain
(such as Flang).

> **Note:** Expression text is preserved verbatim. The converter rewrites the
> directive keyword sequence and sentinel, but it does not attempt to transform
> array subscripts or other language-specific syntax in clause expressions.

## Rust API

Use [`ir::convert_directive_language`](../api-reference.html#language-conversion)
to convert a directive string. The helper parses the input using the requested
source language and pretty-prints it with the target language's canonical
spelling.

```rust
use roup::ir::{convert_directive_language, Language};

let c_pragma = "#pragma omp target teams distribute parallel for simd schedule(static, 4)";
let fortran = convert_directive_language(c_pragma, Language::C, Language::Fortran)?;
assert_eq!(
    fortran,
    "!$omp target teams distribute parallel do simd schedule(static, 4)"
);
```

The function returns a `Result<String, LanguageConversionError>`. Parsing
failures (missing sentinel, malformed clause, etc.) are reported as errors and
no string allocation occurs.

## C API

The C interface exposes `roup_convert_language`. The function allocates a new
C string containing the converted pragma; callers must release it with
`roup_string_free`.

```c
#include <roup_constants.h>
#include <roup_compat.h>

const char* pragma = "#pragma omp parallel for private(i, j)";
char* fortran = roup_convert_language(pragma, ROUP_LANG_C, ROUP_LANG_FORTRAN_FREE);
if (fortran) {
    printf("%s\n", fortran);  // !$omp parallel do private(i, j)
    roup_string_free(fortran);
}
```

The language constants mirror the parser entry points:

- `ROUP_LANG_C` – C/C++ (`#pragma omp`)
- `ROUP_LANG_FORTRAN_FREE` – Fortran free-form (`!$OMP`)
- `ROUP_LANG_FORTRAN_FIXED` – Fortran fixed-form (treated identically for
  output; both map to `!$omp`).

Invalid parameters, UTF-8 errors, or syntactically incorrect directives produce
`NULL`.

## ompparser Compatibility Layer

Projects using the ompparser drop-in replacement can call the same function via
`roup_compat.h`. A small RAII helper keeps memory management simple.

```cpp
#include <memory>
#include <roup_constants.h>
#include "roup_compat.h"

std::unique_ptr<char, decltype(&roup_string_free)> converted(
    roup_convert_language("!$OMP DO SCHEDULE(DYNAMIC)",
                          ROUP_LANG_FORTRAN_FREE,
                          ROUP_LANG_C),
    &roup_string_free);

if (converted) {
    std::string result{converted.get()};
    // result == "#pragma omp for schedule(DYNAMIC)"
}
```

This workflow enables tools that parse C OpenMP pragmas (for example through
Clang) to automatically generate the equivalent Fortran directives ready for
Flang or other Fortran-centric consumers.
