# Cursor AI Collaboration: Best Practices for Clean Language Compiler Project

This document outlines the agreed-upon best practices for collaborating with the Cursor AI assistant on the Clean Language compiler project. Adhering to these guidelines ensures a consistent, efficient, and predictable development workflow.

Our goal is to create a compiler that is easy to understand, easy to maintain, and easy to extend. with a clean structure, and a wasm output. The strategy is to use the cursor ai to implement the critical fixes, and then to use the cursor ai to implement the next steps. The tasks should be created and updated based in this this goal.

## Project Management & Structure

1.  **Confirm Creations:** Always explicitly request confirmation *before* creating any new files, folders, Docker containers, `.env` files, `docker-compose.yml`, or `Dockerfile`s. Prioritize modifying existing artifacts whenever possible.
2.  **No Redundant Root Folder:** Work directly within the existing `clean-language-compiler/` workspace. Do not create another nested folder with the same name.
3.  **Single Docker Compose:** Maintain only one `docker-compose.yml` file, located at the project root (`clean-language-compiler/`). Do not place it within subdirectories.
4.  **Targeted File Structure:** Avoid creating deeply nested folders unless absolutely necessary and explicitly requested. Aim for a relatively flat structure. tests should be in the tests folder, scripts should be in the scripts folder, and implementation should be in the src folder, and the docs should be in the docs folder, and the .cursor folder should be in the root of the project, and the .github folder should be in the root of the project.
5.  **Environment Files:** Treat `.env` files with care. Request confirmation before creation and never overwrite existing ones unless specifically instructed. Store API keys and sensitive configurations here.
6.  **Documentation Files:** Always update the documentation files when creating new features or fixing bugs. We are going to be using the `docs/` folder to store all of our documentation. The docs are the source of truth for the project. They are the first place to look for information about the project.  
we have 3 types of documentation files:
- inside .cursor folder: cursor-best-practices.md it is the file you are reading now that contains the rules for the cursor ai to follow. 
- inside docs/language folder: clean-language-origin.txt, this file is the source of truth for the cursor ai, and it is not modified by the ai. (it is the original language specs that is the base for clean-language-wasm.md), clean-language-wasm.md (the wasm spec, the specification of the language for the compiler based on the clean-language-origin.txt and that can be modified by the ai based on the clean-language-origin.txt), and compiler-specs.md (the compiler spec, the specification for the compiler).
- inside docs/tasks folder: tasks.md (the next tasks to be done and the status of the tasks, including tests), and implementation-plan.md (the implementation plan, detailing the strategy for the next steps), critical-fixes-summary.md (the summary of the current critical fixes). 
- inside docs/tools folder: implementation-tools.md provides additional tooling guidance for the implementation process.


## Code Development & Modification

6.  **Iterate on Existing Code:** Before generating new code, analyze existing patterns in the codebase (e.g., in `src/`, `tests/`). Follow established styles and structures. Request confirmation before introducing significantly different patterns or architectural changes.
7.  **No Placeholders or Fallbacks:** All code generated must be functional. Avoid placeholders (`// TODO`, `unimplemented!()`), dummy data, or fallback logic (e.g., hardcoded data when an API call fails). If functionality cannot be fully implemented, state it clearly.
8.  **Focused Changes:** Ensure requests are specific to the task. Avoid making unrelated changes to files or functions not directly involved in the current request.
9.  **Small, Manageable Files:** Strive to keep Rust files reasonably sized (aiming for under ~300 lines where practical). Consider refactoring larger components if needed.
10. **Use Standard Library:** Leverage the defined standard library functions (as detailed in `docs/compiler-specs.md`) where appropriate.
11. **ABI Compliance:** When dealing with host interaction or generated WebAssembly, ensure adherence to the ABI defined in `docs/compiler-specs.md` (passing strings/arrays as `(ptr, len)`, using correct numeric types, etc.).

## Workflow & Testing

12. **Use Docker for Execution:** All compiler execution or testing involving running the compiled output should utilize Docker and the project's `docker-compose.yml` (once created).
13. **Test Thoroughly:** Add relevant tests to the `tests/` directory for new features or bug fixes. Tests should ideally run within the appropriate Docker container context.
14. **Verify After Changes:** After applying significant code changes, attempt to run `cargo build` and `cargo test` (or suggest doing so) to immediately check for errors or regressions.

## Communication

15. **Provide Context:** When making requests, ensure sufficient context is provided. Attaching relevant files is often helpful.
16. **Review Edits:** Carefully review the diffs of any proposed file edits before applying them to ensure they match the intent. 