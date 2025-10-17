#!/usr/bin/env python3
"""
Extract OpenMP 6.0 keywords from documentation and validate against parser implementation.

This script parses the OpenMP 6.0 documentation to build canonical lists of directives
and clauses, then compares them to the current parser implementation to identify gaps.
"""

import re
import json
from pathlib import Path
from typing import Dict, List, Set, Tuple

# Paths
REPO_ROOT = Path(__file__).parent.parent
DIRECTIVES_CLAUSES_DOC = REPO_ROOT / "docs/book/src/openmp60-directives-clauses.md"
PARSER_FILE = REPO_ROOT / "src/parser/openmp.rs"

def extract_directives_from_doc() -> Dict[str, Dict]:
    """Extract all directives from the OpenMP 6.0 documentation."""
    with open(DIRECTIVES_CLAUSES_DOC, 'r') as f:
        content = f.read()

    # Extract directive section (lines between "## Directives and Constructs" and "## Clauses")
    dir_match = re.search(r'## Directives and Constructs\n\n(.*?)\n## Clauses', content, re.DOTALL)
    if not dir_match:
        raise ValueError("Could not find directives section in documentation")

    dir_section = dir_match.group(1)
    directives = {}

    # Parse each directive line: - `name` (Section X.Y; ...)
    for line in dir_section.split('\n'):
        if not line.startswith('- `'):
            continue

        # Extract directive name
        name_match = re.search(r'- `([^`]+)`', line)
        if not name_match:
            continue

        name = name_match.group(1)

        # Extract section reference
        section_match = re.search(r'\(Section ([^;]+);', line)
        section = section_match.group(1) if section_match else "unknown"

        # Extract category
        category_match = re.search(r'category: ([^;]+);', line)
        category = category_match.group(1) if category_match else "unknown"

        directives[name] = {
            'section': section,
            'category': category,
            'line': line
        }

    return directives

def extract_clauses_from_doc() -> Dict[str, Dict]:
    """Extract all clauses from the OpenMP 6.0 documentation."""
    with open(DIRECTIVES_CLAUSES_DOC, 'r') as f:
        content = f.read()

    # Extract clause section (lines between "## Clauses" and "## Modifiers")
    clause_match = re.search(r'## Clauses\n\n(.*?)\n## Modifiers', content, re.DOTALL)
    if not clause_match:
        raise ValueError("Could not find clauses section in documentation")

    clause_section = clause_match.group(1)
    clauses = {}

    # Parse each clause line: - `name` (Section X.Y; ...)
    for line in clause_section.split('\n'):
        if not line.startswith('- `'):
            continue

        # Extract clause name
        name_match = re.search(r'- `([^`]+)`', line)
        if not name_match:
            continue

        name = name_match.group(1)

        # Extract section reference
        section_match = re.search(r'\(Section ([^;)]+)', line)
        section = section_match.group(1) if section_match else "unknown"

        clauses[name] = {
            'section': section,
            'line': line
        }

    return clauses

def extract_parser_clauses() -> Dict[str, str]:
    """Extract clauses registered in the parser."""
    with open(PARSER_FILE, 'r') as f:
        content = f.read()

    # Extract openmp_clauses! macro block
    clause_match = re.search(r'openmp_clauses!\s*{(.*?)}\s*\nmacro_rules!', content, re.DOTALL)
    if not clause_match:
        raise ValueError("Could not find openmp_clauses macro in parser")

    clause_block = clause_match.group(1)
    clauses = {}

    # Parse each line: Name => { name: "clause_name", rule: ClauseRule::Type },
    for line in clause_block.split('\n'):
        name_match = re.search(r'name: "([^"]+)"', line)
        rule_match = re.search(r'rule: ClauseRule::([A-Za-z_]+)', line)

        if name_match and rule_match:
            clauses[name_match.group(1)] = rule_match.group(1)

    return clauses

def extract_parser_directives() -> Set[str]:
    """Extract directives registered in the parser."""
    with open(PARSER_FILE, 'r') as f:
        content = f.read()

    # Extract openmp_directives! macro block
    dir_match = re.search(r'openmp_directives!\s*{(.*?)}\s*\npub fn clause_registry', content, re.DOTALL)
    if not dir_match:
        raise ValueError("Could not find openmp_directives macro in parser")

    dir_block = dir_match.group(1)
    directives = set()

    # Parse each line: Name => "directive name",
    for line in dir_block.split('\n'):
        if '=>' in line and '"' in line:
            name_match = re.search(r'"([^"]+)"', line)
            if name_match:
                directives.add(name_match.group(1))

    return directives

def determine_clause_rule(clause_name: str, doc_line: str) -> str:
    """
    Determine the appropriate ClauseRule for a clause based on OpenMP 6.0 spec patterns.

    Returns: "Bare", "Parenthesized", or "Flexible"
    """
    # Bare clauses (no arguments)
    bare_clauses = {
        'acq_rel', 'acquire', 'dynamic_allocators', 'exclusive', 'inbranch',
        'inclusive', 'mergeable', 'nogroup', 'notinbranch', 'nowait',
        'relaxed', 'release', 'reproducible', 'seq_cst', 'untied',
        'full', 'init_complete', 'reverse_offload', 'safesync', 'self_maps',
        'simd', 'threads'
    }

    # Flexible clauses (optional arguments)
    flexible_clauses = {
        'capture', 'compare', 'destroy', 'fail', 'no_openmp', 'no_openmp_constructs',
        'no_openmp_routines', 'no_parallelism', 'novariants', 'ordered', 'partial',
        'public', 'read', 'replayable', 'reverse', 'transparent', 'unified_address',
        'unified_shared_memory', 'unroll', 'update', 'weak', 'write',
        'device_safesync', 'graph_reset', 'indirect', 'nocontext'
    }

    if clause_name in bare_clauses:
        return "Bare"
    elif clause_name in flexible_clauses:
        return "Flexible"
    else:
        return "Parenthesized"

def main():
    print("=" * 80)
    print("OpenMP 6.0 Keyword Extraction and Validation")
    print("=" * 80)
    print()

    # Extract from documentation
    print("📖 Extracting from documentation...")
    doc_directives = extract_directives_from_doc()
    doc_clauses = extract_clauses_from_doc()

    print(f"  ✓ Found {len(doc_directives)} directives in docs")
    print(f"  ✓ Found {len(doc_clauses)} clauses in docs")
    print()

    # Extract from parser
    print("🔍 Extracting from parser...")
    parser_clauses = extract_parser_clauses()
    parser_directives = extract_parser_directives()

    print(f"  ✓ Found {len(parser_clauses)} clauses in parser")
    print(f"  ✓ Found {len(parser_directives)} directives in parser")
    print()

    # Analyze clauses
    print("=" * 80)
    print("CLAUSE ANALYSIS")
    print("=" * 80)
    print()

    doc_clause_names = set(doc_clauses.keys())
    parser_clause_names = set(parser_clauses.keys())

    missing_clauses = sorted(doc_clause_names - parser_clause_names)
    extra_clauses = sorted(parser_clause_names - doc_clause_names)

    print(f"📊 Clause Coverage:")
    print(f"  Doc clauses:    {len(doc_clause_names)}")
    print(f"  Parser clauses: {len(parser_clause_names)}")
    print(f"  Missing:        {len(missing_clauses)}")
    print(f"  Extra:          {len(extra_clauses)}")
    print()

    if missing_clauses:
        print(f"❌ Missing {len(missing_clauses)} clauses in parser:")
        for clause in missing_clauses:
            rule = determine_clause_rule(clause, doc_clauses[clause]['line'])
            section = doc_clauses[clause]['section']
            print(f"  - {clause:<30} (Section {section}; rule: {rule})")
        print()

    if extra_clauses:
        print(f"⚠️  Extra {len(extra_clauses)} clauses not in OpenMP 6.0 docs:")
        for clause in extra_clauses:
            rule = parser_clauses[clause]
            print(f"  - {clause:<30} (rule: {rule})")
        print()

    # Analyze directives
    print("=" * 80)
    print("DIRECTIVE ANALYSIS")
    print("=" * 80)
    print()

    doc_directive_names = set(doc_directives.keys())

    # Normalize parser directives (remove combined forms for base comparison)
    # Base directives are those in the docs (64 total)
    # Combined directives are like "parallel for", "target teams", etc.
    base_parser_directives = set()
    combined_parser_directives = set()

    for directive in parser_directives:
        if directive in doc_directive_names:
            base_parser_directives.add(directive)
        else:
            combined_parser_directives.add(directive)

    missing_directives = sorted(doc_directive_names - base_parser_directives)

    print(f"📊 Directive Coverage:")
    print(f"  Doc base directives:      {len(doc_directive_names)}")
    print(f"  Parser base directives:   {len(base_parser_directives)}")
    print(f"  Parser combined forms:    {len(combined_parser_directives)}")
    print(f"  Total parser directives:  {len(parser_directives)}")
    print(f"  Missing base directives:  {len(missing_directives)}")
    print()

    if missing_directives:
        print(f"❌ Missing {len(missing_directives)} base directives in parser:")
        for directive in missing_directives:
            section = doc_directives[directive]['section']
            category = doc_directives[directive]['category']
            print(f"  - {directive:<30} (Section {section}; {category})")
        print()

    # Summary
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print()

    if not missing_clauses and not missing_directives:
        print("✅ COMPLETE: All OpenMP 6.0 keywords are registered in the parser!")
    else:
        print(f"❌ INCOMPLETE: {len(missing_clauses)} clauses and {len(missing_directives)} directives are missing")
        print()
        print("Next steps:")
        print("  1. Add missing clauses to openmp_clauses! macro in src/parser/openmp.rs")
        print("  2. Add missing directives to openmp_directives! macro")
        print("  3. Run this script again to verify")

    # Generate implementation snippets
    if missing_clauses:
        print()
        print("=" * 80)
        print("CLAUSE IMPLEMENTATION SNIPPET")
        print("=" * 80)
        print()
        print("Add these to openmp_clauses! macro (alphabetically):")
        print()
        for clause in missing_clauses:
            variant = ''.join(word.capitalize() for word in clause.split('_'))
            rule = determine_clause_rule(clause, doc_clauses[clause]['line'])
            print(f"    {variant} => {{ name: \"{clause}\", rule: ClauseRule::{rule} }},")

    if missing_directives:
        print()
        print("=" * 80)
        print("DIRECTIVE IMPLEMENTATION SNIPPET")
        print("=" * 80)
        print()
        print("Add these to openmp_directives! macro (alphabetically):")
        print()
        for directive in missing_directives:
            # Convert to PascalCase variant name
            words = directive.split()
            variant = ''.join(word.capitalize() for word in words)
            print(f"    {variant} => \"{directive}\",")

    # Save to JSON for programmatic use
    output = {
        'doc_clauses': {k: v for k, v in doc_clauses.items()},
        'doc_directives': {k: v for k, v in doc_directives.items()},
        'parser_clauses': parser_clauses,
        'parser_directives': list(parser_directives),
        'missing_clauses': missing_clauses,
        'missing_directives': missing_directives,
        'extra_clauses': extra_clauses,
        'combined_directives': list(combined_parser_directives),
    }

    output_file = REPO_ROOT / "keywords_analysis.json"
    with open(output_file, 'w') as f:
        json.dump(output, f, indent=2)

    print()
    print(f"📄 Full analysis saved to: {output_file}")

if __name__ == "__main__":
    main()
