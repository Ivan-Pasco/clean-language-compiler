# Platform 17. Text Files

Every file the Clean toolchain reads or writes — source files, manifests, lockfiles, request documents, diagnostic output — is UTF-8 text with a small set of rules that decide who validates the encoding, what happens when a byte-order mark appears, and how line terminators survive an in-place edit. Those facts had been asserted independently in five other chapters (the memory model, the type system, the compiler, the semantic rules, the server library) and owned nowhere; this chapter is their home. It draws a hard line against runtime data — an application processing a Latin-1 CSV or a binary upload is doing legitimate work and travels through the `bytes` type, not through the rules here.

---

## 17.1 Scope

This chapter governs the **files the Clean toolchain itself reads and writes**: Clean source (`.cln`), project and library manifests (`clean.toml`, `library.toml`), lockfiles, compilation request documents, WIT files, build manifests, and diagnostic output. It fixes how those files are encoded (§17.2–§17.5) and how their line terminators survive a tool touching them (§17.6).

It does **not** govern the data a Clean program handles while it runs. An application that reads a Latin-1 CSV, receives a binary upload, or proxies an arbitrary byte stream is doing legitimate work, and [12 — Server Extensions](./12-server-extensions.md) deliberately provides raw-byte paths for exactly that case. The runtime boundary is drawn by the type system, not by this chapter: `string` is UTF-8 by construction ([03 §Memory Model](./03-memory-model.md)), and anything that is not UTF-8 travels as `bytes`.

The distinction matters because the two halves fail differently. A mis-encoded toolchain file silently changes what a program *means*; a mis-encoded byte stream at runtime is just data the program has to handle.

---

## 17.2 The invariant

### TXT-01 — Every ecosystem text file is UTF-8

Every text file the Clean toolchain reads or writes MUST be encoded in UTF-8. A byte sequence that is not well-formed UTF-8 is not a Clean file of any kind: not a source file, not a manifest, not a lockfile, not a request document. There is no configurable encoding, no per-project override, and no fallback table.

This applies to file content only. It says nothing about the bytes an application processes at runtime (§17.1).

**Why:** Two developers with the same files must get the same build ([C-04](../01%20governance/05-concerns.md), [C-10](../01%20governance/05-concerns.md)). A file has no metadata declaring its encoding, so if the specification does not fix one, each tool picks its own, and the same bytes become two different programs — different string literals, different comments, and in the worst case a different parse.

**Check:** a file whose bytes violate UTF-8 well-formedness is rejected by every tool that reads it, and no tool in the ecosystem accepts an encoding option.

---

## 17.3 Who enforces it

### TXT-02 — The reader validates, at the moment it reads

The component that turns bytes on disk into text MUST validate those bytes as well-formed UTF-8 before using them, and MUST refuse the file with [`CFG005`](./09-error-codes.md#316-configuration-codes-cfg) — naming the path and the byte offset of the first violation — rather than substituting, stripping, or replacing the offending bytes. Substitution characters (`U+FFFD`) MUST NOT be produced.

The readers are the components that touch the filesystem or an editor buffer: Clean Framework (assembling a compilation request), Clean Manager, the language server ([04](./04-ide-lsp-architecture.md)), and any CI or agent harness that builds a request document directly.

The compiler is **not** a reader and MUST NOT be assigned this duty. It obtains every input from the request document and touches nothing else ([CMP-01](./14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)); by the time source text reaches it, the bytes have already been decoded upstream. Its guarantee that `sources[].content` is UTF-8 text is inherited from this rule, not established by a check of its own.

**Why:** validation is only possible where the raw bytes are. Once a file has been decoded with the wrong table, the damage is well-formed UTF-8 — a source read as Latin-1 yields `Ã±` where `ñ` was written, and `Ã±` is two perfectly valid characters. No downstream component can detect it, ever. The check is therefore not merely best placed at the read boundary; it is only *possible* there.

**Check:** a file containing the single byte `0xF1` followed by an ASCII letter is rejected by the framework, the manager and the language server alike, each naming the same byte offset.

---

## 17.4 What the toolchain writes

### TXT-03 — Every generated text file is emitted as UTF-8

Every text artifact the ecosystem produces — scaffolded source, generated manifests, lockfiles, build manifests, request documents, diagnostic output in every rendering — MUST be written as UTF-8. A generator MUST NOT emit in the platform's default encoding, and MUST NOT vary its output encoding by host operating system.

**Why:** [TXT-01](#txt-01--every-ecosystem-text-file-is-utf-8) is only self-sustaining if both halves hold. With validation alone, one tool can generate a file that another tool is then obliged to reject — a failure the user did not cause and cannot fix. With both halves, every file the ecosystem produces is a file the ecosystem accepts.

**Check:** the byte stream of every generated artifact is well-formed UTF-8 on every supported platform, for identical inputs.

---

## 17.5 Byte-order marks

### TXT-04 — A leading byte-order mark is consumed and is not source text

A reader that finds the byte sequence `EF BB BF` as the **first three bytes** of a file MUST consume it and MUST NOT expose it to any later stage. The text begins after it, and every position the toolchain reports over that text — line, column, and the byte offsets of diagnostic spans — is measured from the first byte that follows, so the first character of a file with a mark and the first character of the same file without one occupy the same position.

The one exception is the offset carried by [`CFG005`](./09-error-codes.md#316-configuration-codes-cfg): an encoding failure happens before any text exists, so that offset is always counted from the true start of the file.

`U+FEFF` in any other position is not a mark and MUST NOT be treated as one. Inside a string literal it is ordinary content like any other character; in code position it is a non-ASCII character and therefore [`SYN001`](./09-error-codes.md#31-syntax-codes-syn) under [LEX-02](../04%20language/03-lexical-structure.md#lex-02--keywords-and-identifiers-are-ascii-text-is-unicode).

Generators MUST NOT write a byte-order mark. This holds for every artifact covered by [TXT-03](#txt-03--every-generated-text-file-is-emitted-as-utf-8), on every host platform.

**Why:** the three options are not symmetric. Treating the mark as content is the worst outcome available — it becomes an invisible character fused to the first token, and the resulting failure cannot be diagnosed by looking at the file, because the file looks correct. Rejecting it is honest but produces an error at line 1, column 1 about bytes the developer cannot see, for a file their editor considers valid ([C-01](../01%20governance/05-concerns.md), [C-02](../01%20governance/05-concerns.md)). Consuming it makes a problem the developer did not create disappear without a trace. Not emitting one keeps the ecosystem's own files free of a mark that would otherwise land mid-file whenever anything concatenates two of them.

**Check:** a source file, and a byte-identical copy of it prefixed with `EF BB BF`, compile to the same output and report every diagnostic at the same line and column; no artifact the toolchain generates begins with those three bytes.

---

## 17.6 Line terminators

A file's line terminators are a property of the file, not of the machine that opens it. Windows writes `\r\n`, macOS and Linux write `\n`, and git may convert between them on checkout without telling anyone — so the same repository is genuinely different bytes on two developers' disks, while looking identical in both their editors. Neither of the two rules below tries to prevent that. They make it stop mattering.

The compiler's side of this is separate and lives in the language spec: the lexer accepts both terminators and normalises before it does anything else ([LEX-07](../04%20language/03-lexical-structure.md#lex-07--line-terminators)). The compiler never writes a source file, so the two rules never meet.

### TXT-05 — A tool that rewrites a file preserves its line terminators

A tool that reads an existing file and writes it back — a formatter, an automated fix, a command that edits a manifest in place — MUST reproduce the file's existing line-terminator convention. It MUST NOT convert a file to a different convention as a side effect of an unrelated edit.

The convention of an existing file is **the first line terminator it contains**: `\r\n` if the first one is a carriage return followed by a line feed, `\n` otherwise. A whole-file scan and a majority vote are not used — the rule must be cheap, deterministic and free of ties.

**Why:** a tool that normalises terminators while changing one line produces a diff of the entire file. The change the developer made becomes invisible inside it, review becomes impossible, and the blame history for every line is destroyed. Editors have preserved conventions for decades for exactly this reason; a toolchain that does not is a toolchain developers will route around.

**Check:** reading a file and writing it back with no semantic change produces a byte-identical file, for a file of each convention.

### TXT-06 — A tool that creates a file uses the project's declared convention, and `\n` when there is none

When a tool creates a file that did not exist there is nothing to preserve, so the convention is chosen:

1. If an `.editorconfig` applying to the file's path declares `end_of_line`, that value is used.
2. Otherwise `\n`.

A tool MUST NOT consult the host operating system for this. `cln new` on Windows and `cln new` on macOS produce byte-identical files.

Project scaffolding writes an `.editorconfig` declaring `end_of_line = lf` ([Framework CLI §Scaffolding](../02%20components/framework/03-cli.md)), so the convention is a visible line in the repository that a team can change in one place, rather than a default buried in a tool. Changing it is safe at any time: editors honour it, [TXT-05](#txt-05--a-tool-that-rewrites-a-file-preserves-its-line-terminators) preserves whatever the files then hold, and the lexer accepts both regardless.

Artifacts that are regenerated whole on every run — the build manifest, diagnostic output, lockfiles, the compilation request document — are not covered by either rule. They have no previous version worth preserving and no developer edits them by hand: they are always written with `\n`.

**Why:** deriving the choice from the host platform would make one command produce different bytes on different machines, so a mixed-platform team's first commit would differ depending on who ran it ([C-04](../01%20governance/05-concerns.md)). `\n` is also what git stores, so it is the choice that surprises that team least.

**Check:** the same scaffolding command run on two platforms produces byte-identical files; with an `.editorconfig` declaring `crlf`, the created files carry `\r\n`.

---

## Changelog

- 2026-08-02 — §17.6 added with `TXT-05` (a tool that rewrites a file preserves its line terminators; the convention is the first terminator in the file) and `TXT-06` (a tool that creates one honours `.editorconfig`, and uses `\n` when there is none, never the host platform). Closes the toolchain half of question 4 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md); the lexer half is [LEX-07](../04%20language/03-lexical-structure.md#lex-07--line-terminators). **Chapter renamed** from *Text Encoding* to *Text Files*: line terminators are not an encoding, and the chapter's subject was always the shape of the files the toolchain reads and writes rather than character encoding alone. The file number and the `TXT-` prefix are unchanged, so no rule ID moves ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).
- 2026-08-02 — `TXT-04` added as §17.5, closing question 5 (byte-order marks) of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md) and replacing the *Open* placeholder this chapter shipped with hours earlier. A leading `EF BB BF` is consumed and is not source text; positions are measured from the byte after it, with `CFG005`'s offset the one declared exception since it precedes decoding; `U+FEFF` elsewhere is ordinary content or `SYN001`; generators never emit one.
- 2026-08-02 — Initial version, Accepted. Created as the home of a fact the specification asserted in five documents and owned in none: the memory model ("strings are always UTF-8"), the type system (`string` is UTF-8 text), the compiler ("every `sources[].content` is UTF-8 text"), the semantic rules (`RQD001`'s "decoded UTF-8 content") and the server library ("UTF-8 payload") each stated it independently, with no rule ID, no owner, no diagnostic and no check. `TXT-01` fixes the invariant, `TXT-02` assigns the duty to the components that hold the raw bytes and registers [`CFG005`](./09-error-codes.md#316-configuration-codes-cfg), `TXT-03` closes the generating half. §17.1 draws the boundary against runtime data, which [12 — Server Extensions](./12-server-extensions.md) explicitly serves with raw-byte paths. Resolves question 1 (source file encoding) of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md); the remaining lexical questions of that ADR are unaffected.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Audience:** Toolchain implementors (compiler, framework, manager, language server) handling files on disk
- **Rule prefix:** `TXT-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [09 — Error Codes](./09-error-codes.md) (`CFG005`), [14 — Compiler Architecture](./14-compiler-architecture.md) (`CMP-01`), [LEX-07](../04%20language/03-lexical-structure.md#lex-07--line-terminators)
