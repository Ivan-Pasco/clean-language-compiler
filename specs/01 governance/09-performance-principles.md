# Performance Principles — Clean Language

Performance in Clean Language is not a separate axis being traded against readability or safety — it is what keeps the Language and Security commitments felt in real code. Contracts that are secretly slow get switched off. A compiler that takes ten seconds trains developers to skip type-checking. A "natural way" to iterate that quietly goes quadratic teaches the developer that the natural way is a trap. The eight principles in this document each name a specific LANG or SEC commitment that would quietly be abandoned if performance did not preserve it, and state the class of budget the implementation must hold to keep the commitment honest.

---

## Part 0 — Why performance is a principle

Clean Language commits to being readable ([LANG-01](07-language-principles.md)), one-obvious-way ([LANG-04](07-language-principles.md)), safe by default ([LANG-13](07-language-principles.md)), and honest about the developer's time ([LANG-03](07-language-principles.md)). The security principles ([SEC-01…SEC-10](08-security-principles.md)) add sandboxing, checksum verification, contract enforcement, and constant-time comparisons on top of that.

**Those commitments generate performance obligations.** A slow compiler makes [LANG-03](07-language-principles.md) a lie. A standard library that goes quadratic on a naïve loop pushes the developer toward escape hatches, breaking [LANG-04](07-language-principles.md). Contracts with visible cost make [LANG-13](07-language-principles.md) feel like a tax the developer wants to opt out of. A framework that allocates on every hot-path call turns "the golden path" into "the slow path" and quietly retrains developers to distrust the idioms the language teaches them.

The principles in this document are not preferences and they are not a co-equal axis being traded against DX. **They are the cost of keeping the Language and Security principles honest.** Every PERF-NN below names a Language or Security commitment it exists to preserve, and states the class of implementation budget that preservation requires.

This reframes the two questions performance principles have to answer:

- *"How fast is fast enough?"* — Fast enough that the developer never notices the tax, and therefore never reaches for an escape hatch that would undo a LANG or SEC principle.
- *"When performance and something else conflict, what wins?"* — The question is malformed. If a LANG or SEC principle is right, and a PERF principle exists to preserve it, then the only place a conflict can appear is in the implementation. The fix goes there. See Part 2.

### What the language commits to

1. **A compilation target with a small, characterisable runtime.** The cost model of the emitted code is knowable from static inspection. There is no hidden JIT that might tier up or down, no adaptive optimiser whose behaviour depends on wall-clock warm-up. The concrete target is recorded in [ADR-0022 §1](decisions/0022-foundational-technology-stack.md).
2. **Reproducible builds** ([C-04, C-10](05-concerns.md)). Same inputs, same platform, same output — including generated code shape and size. Performance is a build output; a build that varies in speed for the same source is a defect.
3. **Bounded compile-time work.** Every compile-time handler runs under an explicit time and memory budget ([SEC-02](08-security-principles.md)); the compiler itself has a budget too — it MUST NOT become a language where a project of moderate size takes tens of seconds to type-check.
4. **A characterisable cost for every standard-library operation.** The stdlib is small on purpose ([C-09](05-concerns.md)) so that "what will this cost?" has an answer a developer can read, not one they have to benchmark.

### What the language does NOT commit to

- **Beating native code.** The chosen compilation target has overhead compared to hand-tuned native; Clean does not pretend otherwise. Where the developer needs bare-metal speed, they call out through the host boundary ([SEC-01](08-security-principles.md), [SEC-10](08-security-principles.md)).
- **Zero-cost abstractions in the C++ sense.** Clean prefers a slightly heavier abstraction with obvious cost over an abstraction that is theoretically free but requires the developer to reason about monomorphisation, inlining thresholds, and template instantiation depth. That reasoning is a DX cost ([LANG-03](07-language-principles.md)).
- **Live-image, per-function hot-swap.** Clean is not a live-image system where individual function bodies are edited inside a running process and hot-patched in place — that model requires runtime machinery Clean does not build. **Coarser hot reload during dev time is a different thing and is explicitly in scope**: swapping a whole component at a well-defined boundary while the host process and out-of-component state stay alive. The toolchain SHOULD leverage the composition model recorded in [ADR-0022 §3](decisions/0022-foundational-technology-stack.md) for fast-feedback dev loops.
- **Custom allocators, arena allocation, or manual memory layout control** as first-class user features. These belong in the host, not in the language. Adding them would violate [LANG-01](07-language-principles.md) far more than it would gain.

---

## Part 1 — The Principles

Each principle names the Language or Security commitment it preserves. That is the causal chain: LANG/SEC says the language behaves a certain way; PERF states what class of implementation cost must be honoured to keep that behaviour felt-and-not-just-declared. Specific numeric budgets, benchmark filenames, and target values live in the platform specification.

#### PERF-01 — Pay-as-you-go for safety features

*(Preserves: [LANG-13](07-language-principles.md), [LANG-09](07-language-principles.md); Addresses: C-02, C-08)*

Every safety feature that costs runtime work MUST cost zero when not used. A source file that does not declare a given safety feature MUST NOT pay for that feature's machinery in the emitted artifact.

Codegen MUST be structured so that unused features are absent from the emitted output, not merely present-but-dormant. Dead-code elimination is not enough — the feature MUST be off by default in the shape of the generated code, not on-and-elided.

**Why this principle exists.** [LANG-13](07-language-principles.md) puts contracts on by default and [LANG-09](07-language-principles.md) makes absence a typed value. Both are correct. Both would be quietly abandoned if using them made every unrelated function slower and larger. Pay-as-you-go is what keeps "on by default" honest.

The specific codegen shape for each safety feature, and the tests that verify absence when unused, live in [04 language / 10 — Contracts](../04%20language/10-contracts.md), [04 language / 04 — Type System](../04%20language/04-type-system.md), and the compiler benchmark suite.

Source: [04 language / 10 — Contracts](../04%20language/10-contracts.md); [04 language / 04 — Type System](../04%20language/04-type-system.md).

#### PERF-02 — Contracts have bounded, documented cost

*(Preserves: [LANG-13](07-language-principles.md); Addresses: C-02, C-08)*

A precondition on a typical function MUST evaluate in a tight, published instruction count and MUST reference only values already available at the call site. A contract that walks a data structure is legal but SHOULD be flagged by a compiler warning suggesting a computed-state alternative.

The build option that strips postconditions and invariants (see [LANG-13](07-language-principles.md)) MUST remove all their cost entirely; a stripped release binary MUST be no larger and no slower than an equivalent contract-free program.

**Why this principle exists.** [LANG-13](07-language-principles.md) is only tenable if preconditions are genuinely cheap — otherwise developers omit them on hot functions, exactly where the invariants matter most. A published, benchmarked instruction budget is what makes the LANG-13 default defensible in code review.

The specific instruction-count target, the build option name, and the benchmark that verifies both live in [04 language / 10 — Contracts](../04%20language/10-contracts.md).

Source: [04 language / 10 — Contracts](../04%20language/10-contracts.md).

#### PERF-03 — The standard library has a characterisable cost model

*(Preserves: [LANG-04](07-language-principles.md), [LANG-03](07-language-principles.md); Addresses: C-02, C-09)*

Every standard-library function MUST document its asymptotic cost in its signature or accompanying doc. A stdlib function that hides quadratic or worse behaviour behind a linear-looking API is a defect.

The stdlib is deliberately small ([C-09](05-concerns.md)) so this discipline is tractable. Growing the stdlib means growing the documented cost surface — an unmeasured, undocumented cost is not a valid stdlib addition.

**Why this principle exists.** [LANG-04](07-language-principles.md) says there is one obvious way to do things. The developer trusts that the one obvious way is not secretly the wrong way. Undocumented cost is a form of hidden behaviour, and hidden behaviour is what [LANG-03](07-language-principles.md) forbids.

The specific documentation format and the cost-annotation scheme live in [04 language / 15 — Standard Library](../04%20language/15-standard-library.md).

Source: [04 language / 15 — Standard Library](../04%20language/15-standard-library.md); [C-09](05-concerns.md).

#### PERF-04 — No silent quadratic behaviour on the golden path

*(Preserves: [LANG-01](07-language-principles.md), [LANG-04](07-language-principles.md); Addresses: C-02)*

Common patterns a beginner will reach for MUST NOT accidentally become quadratic or worse. The stdlib MUST provide amortised implementations of common operations where the naïve algorithm would degrade. The framework's static analysis MUST warn on known pathological compositions when the input sizes are detectable.

**Why this principle exists.** [LANG-01](07-language-principles.md) says the developer wrote what they meant and the language reads back what they wrote. If the natural way to combine a thousand elements is a thousand times slower than it should be, the developer learns "the natural way is wrong" — and starts reaching for an escape hatch that violates [LANG-04](07-language-principles.md). The language MUST NOT punish beginners for not knowing the shape of the hot path.

The specific set of "known pathological compositions" the analyzer must detect (naïve string accumulation, membership-check in a loop, N+1 query patterns) and the amortisation strategy for each are defined in [04 language / 15 — Standard Library](../04%20language/15-standard-library.md) and in the framework libraries that own the corresponding APIs.

Source: [04 language / 15 — Standard Library](../04%20language/15-standard-library.md).

#### PERF-05 — Compile time is a first-class budget

*(Preserves: [LANG-03](07-language-principles.md), [SEC-02](08-security-principles.md); Addresses: C-02, C-08)*

The compiler MUST hold itself to per-file and per-project time budgets, published in the platform specification. Exceeding a budget on typical hardware for typical project sizes is a compiler defect, not a "we'll optimise later" acceptable state.

The interactive editing experience MUST stay responsive at a keystroke: incremental type-check on a single-file edit MUST complete inside a sub-frame latency budget for typical file sizes.

Compile-time handlers ([SEC-02](08-security-principles.md)) run under their own hard budgets and MUST NOT be able to blow past them by construction.

**Why this principle exists.** [LANG-03](07-language-principles.md) says developer time is the scarce resource. A compiler that takes tens of seconds on a small project is telling the developer their time doesn't matter. [SEC-02](08-security-principles.md) requires handler budgets to be enforced; the compiler that enforces them cannot itself be the reason a build feels slow.

The specific numeric budgets (per-file cold, per-file warm, per-project clean-build, incremental type-check keystroke latency) live in [03 platform / 04 — IDE / Language Server](../03%20platform/04-ide-lsp-architecture.md) and the associated build-budget spec section.

Source: [03 platform / 04 — IDE / Language Server](../03%20platform/04-ide-lsp-architecture.md); [SEC-02](08-security-principles.md).

#### PERF-06 — Predictable memory over minimum memory

*(Preserves: [LANG-01](07-language-principles.md), [LANG-03](07-language-principles.md); Addresses: C-02, C-08)*

Given a choice between a scheme that uses less memory but has unpredictable allocation timing (long GC pauses, unpredictable heap growth) and a scheme that uses more memory but is predictable, Clean MUST choose predictable.

The language MUST NOT commit to any memory-management scheme whose latency is unbounded or whose timing depends on ambient runtime state. Mechanisms with bounded, statically-analysable behaviour (reference counting, arena resets at declared boundaries, pre-sized allocations) are the acceptable techniques.

"Minimum memory" is a valid goal for a specific application; the *language* optimises for predictability. Applications that need to squeeze the last byte reach for host functions.

**Why this principle exists.** [LANG-01](07-language-principles.md) says the developer reads the code and knows what it does. That extends to runtime behaviour: a developer looking at a Clean function should be able to predict its memory shape without running it. A pause that appears every few seconds because the runtime "felt like collecting" breaks that predictability and [LANG-03](07-language-principles.md) with it.

The specific memory model (linear growth, allocator strategy, per-scope resets) is defined in [03 platform / 03 — Memory Model](../03%20platform/03-memory-model.md) and [03 platform / 05 — Memory Policy](../03%20platform/05-memory-policy.md).

Source: [03 platform / 03 — Memory Model](../03%20platform/03-memory-model.md); [03 platform / 05 — Memory Policy](../03%20platform/05-memory-policy.md).

#### PERF-07 — The golden path is the fast path

*(Preserves: [LANG-01](07-language-principles.md), [LANG-04](07-language-principles.md); Addresses: C-01, C-02)*

The idiomatic way to do something in Clean MUST also be the performant way, within the language's cost model. If "the fast way" requires the developer to abandon the natural idiom and reach for an escape hatch, the natural idiom is a defect.

Concretely: iterating a collection with the language's iteration construct MUST codegen to the same tight loop a hand-written index loop would. DSL-block queries (ORM, HTTP, jobs) MUST codegen to the underlying protocol's most efficient shape by default (prepared statements with parameter binding, appropriate batching, connection reuse). Default handler shapes MUST NOT allocate per-call in the framework's own code.

Escape hatches exist for genuinely exotic cases. When a developer reaches for one on a common case, that is a signal to fix the common case, not to document the escape hatch.

**Why this principle exists.** This is the most direct expression of the "performance preserves DX" thesis. Every time an escape hatch outperforms the idiom, [LANG-04](07-language-principles.md) is quietly undone — because the developer now knows there are two ways to do things, and one of them is a trap. PERF-07 is what keeps LANG-04 true in practice, not just on paper.

The specific idioms whose codegen quality is measured, and the benchmark suites that verify each, live in [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md) and the framework libraries that own the corresponding DSL blocks.

Source: [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md); [LANG-01, LANG-04](07-language-principles.md).

#### PERF-08 — Measured, not asserted

*(Preserves: [LANG-03](07-language-principles.md), all other PERF principles; Addresses: C-02, C-10)*

Every performance claim in the specification MUST be backed by a benchmark in the compiler or framework repository, run in CI, with a documented target and a regression alarm. Prose assertions of the form "X is cheap" are not spec statements; "X evaluates in ≤ N operations on the target platform, verified by benchmark Y" is.

Performance regressions caught by these benchmarks MUST be treated the same as functional regressions: the change that caused them either meets its target or is reverted. Silent performance drift is a defect.

**Why this principle exists.** Every other PERF principle is a claim about behaviour. Without measurement, those claims decay: what was N operations today drifts to 10N next year, and by the time a developer notices, the LANG principle it was preserving has already been quietly abandoned. Measurement is what stops decay.

Source: [C-10](05-concerns.md).

---

## Part 2 — How performance principles relate to LANG and SEC principles

Performance principles are not a co-equal axis being traded against Language or Security. Each PERF-NN exists to preserve a specific LANG-NN or SEC-NN commitment in practice. This shapes conflict resolution:

1. **PERF and LANG (or SEC) cannot conflict at the principle level.** Every PERF principle names the LANG/SEC principle it preserves. A perceived conflict between them means one of the two things below is true, and rule 2 or 3 applies.
2. **When the implementation fails a PERF principle, the fix is in the implementation.** The LANG or SEC principle the PERF principle preserves is NOT relaxed to accommodate the failure. If contracts are slow ([LANG-13](07-language-principles.md) + PERF-01/PERF-02 failing), the answer is faster contract codegen, not "let developers opt out per-function." If the compiler is slow (PERF-05 failing), the answer is a faster compiler, not "let developers skip type-checking."
3. **When a proposed LANG or SEC change would violate a PERF principle at the implementation level, the change MUST arrive with an implementation strategy that meets the PERF budget.** A LANG change that requires the implementation to abandon PERF-01 is not "PERF vs LANG" — it is a LANG proposal without a viable implementation, and MUST be revised or deferred until one exists.
4. **A new PERF principle MUST name the LANG or SEC principle it preserves.** A PERF principle with no such trace is decoration — remove it, or elevate the underlying LANG/SEC commitment that would justify it.
5. **A new PERF principle MUST arrive with (or before) the CI benchmark that verifies it** (PERF-08). A principle with no measurement is aspiration, not law, and cannot be a basis for reviewing code changes.
6. When a spec chapter is amended in a way that changes a documented cost, the chapter MUST cite the PERF-NN principle it derives from, alongside the LANG-NN and concern citations ([DOC-14](00-documentation-principles.md#doc-14)).
7. Retiring a PERF principle MUST NOT reuse its ID. The retired principle keeps its number and is marked *Withdrawn (YYYY-MM-DD, ADR-NNNN)*, with the ADR explaining what changed about the platform, cost model, or the LANG/SEC principle it was preserving.
8. **Principles state policy; specs state numeric budgets; [ADR-0022](decisions/0022-foundational-technology-stack.md) records foundational technology choices.** A PERF principle that names a specific millisecond target, instruction count, filename, or algorithm is a defect — those belong in the spec chapter. Rewording forced by a spec or ADR change MUST NOT change the policy; if it does, an ADR that retires the affected principle is required.

---

## Metadata

- **Status:** Draft
- **Audience:** Compiler and framework maintainers, standard-library authors, library authors, and reviewers of any change that affects compile time, runtime cost, memory footprint, or the perceived responsiveness of the toolchain
- **Rule prefix:** `PERF-`
- **References:** [Language Principles](07-language-principles.md); [Security Principles](08-security-principles.md); [Architectural Concerns](05-concerns.md); [ADR-0022 — Foundational Technology Stack](decisions/0022-foundational-technology-stack.md)
