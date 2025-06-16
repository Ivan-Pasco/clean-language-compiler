#!/bin/bash

# Clean Language Workspace Cleanup Script
# Safely removes test files while preserving important project files

set -e  # Exit on any error

echo "🧹 Starting Clean Language workspace cleanup..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to safely remove files
safe_remove() {
    local file="$1"
    if [[ -f "$file" ]]; then
        echo -e "${YELLOW}Removing:${NC} $file"
        rm "$file"
        return 0
    else
        return 1
    fi
}

# Function to count files before removal
count_files() {
    local pattern="$1"
    local dir="$2"
    find "$dir" -maxdepth 1 -name "$pattern" -type f 2>/dev/null | wc -l
}

echo -e "${BLUE}📊 Analyzing workspace...${NC}"

# Count files to be removed
ROOT_CLEAN_COUNT=$(count_files "*.clean" ".")
ROOT_CLN_COUNT=$(count_files "*.cln" ".")
ROOT_WASM_COUNT=$(count_files "*.wasm" ".")
ROOT_WAT_COUNT=$(count_files "*.wat" ".")
ROOT_JS_TEST_COUNT=$(find . -maxdepth 1 -name "*test*.js" -o -name "run_web_test.js" -o -name "wasm_runner.js" | wc -l)
ROOT_RS_TEST_COUNT=$(find . -maxdepth 1 -name "*test*.rs" -type f | wc -l)

TESTS_CLEAN_COUNT=$(count_files "*.clean" "tests")
TESTS_CLN_COUNT=$(count_files "*.cln" "tests")
TESTS_WASM_COUNT=$(count_files "*.wasm" "tests")

TOTAL_FILES=$((ROOT_CLEAN_COUNT + ROOT_CLN_COUNT + ROOT_WASM_COUNT + ROOT_WAT_COUNT + ROOT_JS_TEST_COUNT + ROOT_RS_TEST_COUNT + TESTS_CLEAN_COUNT + TESTS_CLN_COUNT + TESTS_WASM_COUNT))

echo -e "${BLUE}Files to be removed:${NC}"
echo -e "  Root directory:"
echo -e "    - .clean files: ${ROOT_CLEAN_COUNT}"
echo -e "    - .cln files: ${ROOT_CLN_COUNT}"
echo -e "    - .wasm files: ${ROOT_WASM_COUNT}"
echo -e "    - .wat files: ${ROOT_WAT_COUNT}"
echo -e "    - Test .js files: ${ROOT_JS_TEST_COUNT}"
echo -e "    - Test .rs files: ${ROOT_RS_TEST_COUNT}"
echo -e "  Tests directory:"
echo -e "    - .clean files: ${TESTS_CLEAN_COUNT}"
echo -e "    - .cln files: ${TESTS_CLN_COUNT}"
echo -e "    - .wasm files: ${TESTS_WASM_COUNT}"
echo -e "${BLUE}Total files to remove: ${TOTAL_FILES}${NC}"

if [[ $TOTAL_FILES -eq 0 ]]; then
    echo -e "${GREEN}✅ Workspace is already clean!${NC}"
    exit 0
fi

# Ask for confirmation
echo -e "${YELLOW}⚠️  This will permanently delete ${TOTAL_FILES} test files.${NC}"
read -p "Do you want to continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}❌ Cleanup cancelled.${NC}"
    exit 1
fi

echo -e "${GREEN}🚀 Starting cleanup...${NC}"

# Remove files from root directory
echo -e "${BLUE}Cleaning root directory...${NC}"

# Remove .clean files (but preserve examples/*.clean)
for file in *.clean; do
    [[ -f "$file" ]] && safe_remove "$file"
done

# Remove .cln files
for file in *.cln; do
    [[ -f "$file" ]] && safe_remove "$file"
done

# Remove .wasm files
for file in *.wasm; do
    [[ -f "$file" ]] && safe_remove "$file"
done

# Remove .wat files
for file in *.wat; do
    [[ -f "$file" ]] && safe_remove "$file"
done

# Remove test JavaScript files
for file in *test*.js run_web_test.js wasm_runner.js; do
    [[ -f "$file" ]] && safe_remove "$file"
done

# Remove test Rust files from root
for file in *test*.rs; do
    [[ -f "$file" ]] && safe_remove "$file"
done

# Remove files from tests directory
if [[ -d "tests" ]]; then
    echo -e "${BLUE}Cleaning tests directory...${NC}"
    
    # Remove .clean files from tests
    for file in tests/*.clean; do
        [[ -f "$file" ]] && safe_remove "$file"
    done
    
    # Remove .cln files from tests
    for file in tests/*.cln; do
        [[ -f "$file" ]] && safe_remove "$file"
    done
    
    # Remove .wasm files from tests
    for file in tests/*.wasm; do
        [[ -f "$file" ]] && safe_remove "$file"
    done
fi

# Clean up any empty test directories (but preserve tests/ itself)
echo -e "${BLUE}Cleaning empty directories...${NC}"
if [[ -d "tests/test_inputs" ]]; then
    rmdir tests/test_inputs 2>/dev/null || true
fi

if [[ -d "tests/parser_tests" ]]; then
    # Only remove if empty
    if [[ -z "$(ls -A tests/parser_tests 2>/dev/null)" ]]; then
        rmdir tests/parser_tests 2>/dev/null || true
    fi
fi

echo -e "${GREEN}✅ Cleanup completed successfully!${NC}"

# Show what's left
echo -e "${BLUE}📋 Remaining important files:${NC}"
echo -e "  - Source code: ${GREEN}src/${NC}"
echo -e "  - Examples: ${GREEN}examples/${NC}"
echo -e "  - Modules: ${GREEN}modules/${NC}"
echo -e "  - Documentation: ${GREEN}*.md${NC}"
echo -e "  - Configuration: ${GREEN}Cargo.toml${NC}"
echo -e "  - Test framework: ${GREEN}tests/*.rs${NC}"

echo -e "${GREEN}🎉 Workspace cleanup complete!${NC}" 