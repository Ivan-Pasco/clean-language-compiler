//! Grammar-seeded program generator (M9). Expands `file` of the Lark
//! grammar in `grammar/` with a deterministic PRNG, so every generated
//! program is drawn from the grammar derived from the specification and
//! every failure reproduces from its seed alone.
//!
//! Rules expand structurally. A terminal becomes one lexeme: a terminal
//! spelled only with literals and other terminals is built from its
//! definition, its pieces concatenated with no space; a terminal whose
//! definition holds a regular expression takes its text from the
//! hand-written sample table below, and one with no sample fails loudly.
//! `_NL`, `_INDENT` and `_DEDENT` are the indenter's events (grammar
//! `lexical.md`): they render as a line end and as one tab more or less at
//! the start of the next line.

// Shared by several test binaries; not every binary uses every item.
#![allow(dead_code)]

use super::lark::{collect_names, has_regex, is_terminal_name, Expr, Grammar};
use indexmap::{IndexMap, IndexSet};

/// Cost of a derivation, in emitted tokens. `INF` marks rules that cannot
/// be generated; alternatives priced `INF` are never chosen.
const INF: u32 = u32::MAX / 4;

/// The start symbols of the grammar: a whole `.cln` file, and the
/// statement sequence the specification's fragments are.
pub const ROOTS: [&str; 2] = ["file", "fragment"];

/// The terminals the postlex indenter produces (grammar `lexical.md`).
/// They are events, not text: `_NL` ends a line; `_INDENT` / `_DEDENT`
/// move the tab level of the lines after it.
const NEWLINE: &str = "_NL";
const INDENT: &str = "_INDENT";
const DEDENT: &str = "_DEDENT";

/// The `//` comment the generator sometimes writes before a line end.
const LINE_COMMENT: &str = "LINE_COMMENT";

/// Sample text for every terminal whose definition holds a regular
/// expression, keyed by terminal name. Each sample is one whole lexeme the
/// terminal's pattern matches (lookaheads included: a keyword that needs
/// `:` or `(` after it gets them from the rule that uses it). A reachable
/// regex terminal missing here, or an entry naming no regex terminal of
/// the grammar, stops the generator: refreshing the grammar means
/// refreshing this table in the same change.
pub fn regex_terminal_samples() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        // lexical — comments (written by the generator at line ends).
        (LINE_COMMENT, &["// note", "// ünïcode 漢 🙂", "//"]),
        // lexical — names: never a reserved word.
        (
            "NAME",
            &[
                "x",
                "count",
                "myVariable",
                "value1",
                "MAX_SIZE",
                "Point",
                "T",
                "description",
                "step",
                "result",
            ],
        ),
        // lexical — literals.
        ("INT", &["0", "42", "7", "0xff", "0b1010", "0o777"]),
        ("FLOAT", &["3.14", ".5", "6.02e23", "0.0"]),
        (
            "STRING",
            &[
                "\"\"",
                "\"Hello, World!\"",
                "\"Line 1\\nLine 2\"",
                "\"Hello, {name}!\"",
                "\"\\{literal\\}\"",
                "\"ünïcode 漢 🙂\"",
            ],
        ),
        // statements — sized numeric types, one token.
        ("SIZE_SUFFIX", &[":8", ":16", ":32", ":64", ":8u", ":32u"]),
        // ai — a spec path, relative to the project root.
        ("SPEC_PATH", &["\"specs/payment.md\"", "\"docs/a.md\""]),
        // modules — an import path.
        ("IMPORT_PATH", &["app/utils.cln", "app/data/User.cln"]),
    ]
}

/// Deterministic PRNG (splitmix64) — no `rand`, no global state, so a seed
/// printed by a failing run reproduces the exact program anywhere.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// One rendered token of the generated program.
enum Tok {
    /// A lexeme; rendered with a single separating space from the previous
    /// lexeme on the same line (inline whitespace separates tokens and is
    /// never itself a token, per 03-lexical-structure).
    Text(String),
    Newline,
    Indent,
    Dedent,
}

pub struct Generator {
    grammar: Grammar,
    min_cost: IndexMap<String, u32>,
    samples: IndexMap<&'static str, &'static [&'static str]>,
}

impl Generator {
    pub fn new(grammar: Grammar) -> Generator {
        let samples: IndexMap<_, _> = regex_terminal_samples().into_iter().collect();
        for name in samples.keys() {
            let def = grammar.terminals.get(*name).unwrap_or_else(|| {
                panic!("sample table names {name}, which the grammar does not define — drop it")
            });
            assert!(
                has_regex(&def.expr),
                "sample table names {name}, whose definition holds no regular \
                 expression — it is built from its definition; drop the entry"
            );
        }
        let min_cost = compute_min_costs(&grammar);
        let generator = Generator {
            grammar,
            min_cost,
            samples,
        };
        for name in generator.reachable_terminals() {
            let def = generator
                .grammar
                .terminals
                .get(&name)
                .unwrap_or_else(|| panic!("terminal {name} is referenced but not defined"));
            assert!(
                !has_regex(&def.expr) || generator.samples.contains_key(name.as_str()),
                "regex terminal {name} has no sample text — the grammar \
                 grew a terminal; add it to regex_terminal_samples()"
            );
        }
        generator
    }

    /// Terminals that generation can reach from the roots: through rules,
    /// and through the definitions of terminals built piece by piece
    /// (a sampled terminal is a leaf). The indenter's events are not
    /// lexemes; the line comment is written at line ends.
    fn reachable_terminals(&self) -> IndexSet<String> {
        let mut seen: IndexSet<String> = IndexSet::new();
        let mut terminals: IndexSet<String> = IndexSet::new();
        let mut stack: Vec<String> = ROOTS.iter().map(|r| r.to_string()).collect();
        stack.push(LINE_COMMENT.to_string());
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) || [NEWLINE, INDENT, DEDENT].contains(&name.as_str()) {
                continue;
            }
            if is_terminal_name(&name) {
                terminals.insert(name.clone());
                if self.samples.contains_key(name.as_str()) {
                    continue;
                }
            }
            if let Some(def) = self.grammar.definition(&name) {
                collect_names(&def.expr, &mut |n| stack.push(n.to_string()));
            }
        }
        terminals
    }

    /// Every root must have a finite derivation; anything infinite is
    /// either a grammar change or a generator bug, and the fuzzer refuses
    /// to run rather than silently under-covering.
    pub fn assert_root_generatable(&self) {
        for root in ROOTS {
            let cost = self.min_cost.get(root).copied().unwrap_or(INF);
            assert!(
                cost < INF,
                "{root} has no finite derivation — grammar changed?"
            );
        }
    }

    pub fn ungeneratable(&self) -> Vec<&str> {
        self.min_cost
            .iter()
            .filter(|(_, &c)| c >= INF)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Generates one program (a `source_file`) from the given seed.
    /// `budget` caps emitted tokens; once exceeded, expansion always takes
    /// the cheapest branch, so termination is structural, not
    /// probabilistic.
    pub fn program(&self, seed: u64, budget: u32) -> String {
        self.sentence(ROOTS[0], seed, budget)
    }

    /// Generates one sentence of the given start symbol.
    pub fn sentence(&self, root: &str, seed: u64, budget: u32) -> String {
        let mut rng = Rng::new(seed);
        let mut toks = Vec::new();
        let mut spent = 0u32;
        self.expand(
            &Expr::Name(root.to_string()),
            &mut rng,
            budget,
            &mut spent,
            &mut toks,
            0,
        );
        render(&toks)
    }

    fn cost(&self, expr: &Expr) -> u32 {
        expr_cost(expr, &self.min_cost, &self.grammar)
    }

    fn expand(
        &self,
        expr: &Expr,
        rng: &mut Rng,
        budget: u32,
        spent: &mut u32,
        out: &mut Vec<Tok>,
        depth: u32,
    ) {
        // Past either bound, expansion always takes the cheapest branch, so
        // recursion depth stays within what default 2 MiB thread stacks
        // handle on both the generator's and the compiler's side.
        let over_budget = *spent >= budget || depth >= 64;
        match expr {
            Expr::Seq(items) => {
                let mut i = 0;
                while i < items.len() {
                    if is_name(&items[i], INDENT) {
                        let close = items[i..]
                            .iter()
                            .position(|item| is_name(item, DEDENT))
                            .map(|k| i + k)
                            .expect("an _INDENT closes with a _DEDENT in the same sequence");
                        out.push(Tok::Indent);
                        self.expand_body(&items[i + 1..close], rng, budget, spent, out, depth);
                        i = close + 1;
                        continue;
                    }
                    self.expand(&items[i], rng, budget, spent, out, depth + 1);
                    i += 1;
                }
            }
            Expr::Alt(alts) => {
                let viable: Vec<&Expr> = alts.iter().filter(|a| self.cost(a) < INF).collect();
                assert!(!viable.is_empty(), "alternation with no finite branch");
                let chosen = if over_budget {
                    // Any of the cheapest branches, so a forced ending
                    // still varies (not always the first literal form).
                    let least = viable
                        .iter()
                        .map(|a| self.cost(a))
                        .min()
                        .expect("non-empty");
                    let cheapest: Vec<&&Expr> =
                        viable.iter().filter(|a| self.cost(a) == least).collect();
                    cheapest[rng.below(cheapest.len())]
                } else {
                    &viable[rng.below(viable.len())]
                };
                self.expand(chosen, rng, budget, spent, out, depth + 1);
            }
            Expr::Opt(inner) => {
                if !over_budget && self.cost(inner) < INF && rng.chance(40) {
                    self.expand(inner, rng, budget, spent, out, depth + 1);
                }
            }
            Expr::Star(inner) | Expr::Plus(inner) => {
                if matches!(expr, Expr::Plus(_)) {
                    self.expand(inner, rng, budget, spent, out, depth + 1);
                }
                if over_budget || self.cost(inner) >= INF {
                    return;
                }
                let mut reps = 0;
                while reps < 3 && rng.chance(50) && *spent < budget {
                    self.expand(inner, rng, budget, spent, out, depth + 1);
                    reps += 1;
                }
            }
            Expr::Literal(text) => {
                *spent += 1;
                out.push(Tok::Text(text.clone()));
            }
            Expr::Regex(pattern) => panic!(
                "anonymous regular expression /{pattern}/ in a rule has no sample \
                 text — name it as a terminal and add it to the sample table"
            ),
            Expr::Name(name) if name == NEWLINE => {
                if rng.chance(5) {
                    out.push(Tok::Text(self.lexeme(LINE_COMMENT, rng)));
                }
                out.push(Tok::Newline);
            }
            Expr::Name(name) if name == INDENT => out.push(Tok::Indent),
            Expr::Name(name) if name == DEDENT => out.push(Tok::Dedent),
            Expr::Name(name) if is_terminal_name(name) => {
                *spent += 1;
                out.push(Tok::Text(self.lexeme(name, rng)));
            }
            Expr::Name(name) => {
                let rule = self
                    .grammar
                    .rules
                    .get(name)
                    .unwrap_or_else(|| panic!("undefined rule {name}"));
                self.expand(&rule.expr, rng, budget, spent, out, depth + 1);
            }
        }
    }

    /// The items between an `_INDENT` and its `_DEDENT`. The lexer emits
    /// no `_INDENT` for a body without a line, so a body must still come
    /// out with a token:
    /// when the free draw leaves it empty, it is drawn again forced.
    fn expand_body(
        &self,
        items: &[Expr],
        rng: &mut Rng,
        budget: u32,
        spent: &mut u32,
        out: &mut Vec<Tok>,
        depth: u32,
    ) {
        let start = out.len();
        for item in items {
            self.expand(item, rng, budget, spent, out, depth + 1);
        }
        if !emits_text(&out[start..]) {
            out.truncate(start);
            let emitted = self.expand_forced_seq(items, rng, budget, spent, out, depth + 1);
            assert!(emitted, "an indented body can produce no token");
        }
        out.push(Tok::Dedent);
    }

    fn expand_forced_seq(
        &self,
        items: &[Expr],
        rng: &mut Rng,
        budget: u32,
        spent: &mut u32,
        out: &mut Vec<Tok>,
        depth: u32,
    ) -> bool {
        let mut emitted = false;
        for item in items {
            if emitted {
                self.expand(item, rng, budget, spent, out, depth + 1);
            } else {
                emitted = self.expand_forced(item, rng, budget, spent, out, depth + 1);
            }
        }
        emitted
    }

    /// Expands `expr` so that it emits at least one lexeme when it can:
    /// an optional or repeated part is taken once, and an alternation
    /// takes its cheapest branch that emits something. Only the path to
    /// the first lexeme is forced; the rest expands as usual.
    fn expand_forced(
        &self,
        expr: &Expr,
        rng: &mut Rng,
        budget: u32,
        spent: &mut u32,
        out: &mut Vec<Tok>,
        depth: u32,
    ) -> bool {
        match expr {
            Expr::Seq(items) => {
                let start = out.len();
                // A nested indented body is expanded (and forced) whole.
                if items.iter().any(|i| is_name(i, INDENT)) {
                    self.expand(expr, rng, budget, spent, out, depth);
                    return emits_text(&out[start..]);
                }
                self.expand_forced_seq(items, rng, budget, spent, out, depth)
            }
            Expr::Alt(alts) => {
                let emitting: Vec<&Expr> = alts
                    .iter()
                    .filter(|a| (1..INF).contains(&self.cost(a)))
                    .collect();
                let Some(least) = emitting.iter().map(|a| self.cost(a)).min() else {
                    self.expand(expr, rng, budget, spent, out, depth);
                    return false;
                };
                let cheapest: Vec<&&Expr> =
                    emitting.iter().filter(|a| self.cost(a) == least).collect();
                let chosen = cheapest[rng.below(cheapest.len())];
                self.expand_forced(chosen, rng, budget, spent, out, depth + 1)
            }
            Expr::Opt(inner) | Expr::Star(inner) | Expr::Plus(inner) => {
                if self.cost(inner) >= INF {
                    return false;
                }
                self.expand_forced(inner, rng, budget, spent, out, depth + 1)
            }
            Expr::Name(name) if !is_terminal_name(name) => {
                let rule = &self.grammar.rules[name.as_str()];
                self.expand_forced(&rule.expr, rng, budget, spent, out, depth + 1)
            }
            _ => {
                let start = out.len();
                self.expand(expr, rng, budget, spent, out, depth);
                emits_text(&out[start..])
            }
        }
    }

    /// One lexeme of the named terminal. A sampled terminal draws from its
    /// samples. A terminal built from its definition must not come out as
    /// a sample of a terminal of higher priority — the lexer would read it
    /// as that one (an IDENTIFIER spelled `if` is the keyword) — so the
    /// draw repeats until it does not.
    fn lexeme(&self, name: &str, rng: &mut Rng) -> String {
        if let Some(samples) = self.samples.get(name) {
            return samples[rng.below(samples.len())].to_string();
        }
        let def = self
            .grammar
            .terminals
            .get(name)
            .unwrap_or_else(|| panic!("undefined terminal {name}"));
        for _ in 0..64 {
            let mut text = String::new();
            self.spell(&def.expr, rng, &mut text, 0);
            let shadowed = self.samples.iter().any(|(other, samples)| {
                self.grammar.terminals[*other].priority > def.priority
                    && samples.contains(&text.as_str())
            });
            if !shadowed {
                return text;
            }
        }
        panic!("{name}: 64 straight draws spelled a higher-priority terminal");
    }

    /// Spells a terminal's definition: pieces concatenate with no space.
    fn spell(&self, expr: &Expr, rng: &mut Rng, out: &mut String, depth: u32) {
        assert!(depth < 64, "terminal definitions nest too deep");
        match expr {
            Expr::Seq(items) => {
                for item in items {
                    self.spell(item, rng, out, depth + 1);
                }
            }
            Expr::Alt(alts) => self.spell(&alts[rng.below(alts.len())], rng, out, depth + 1),
            Expr::Opt(inner) => {
                if rng.chance(40) {
                    self.spell(inner, rng, out, depth + 1);
                }
            }
            Expr::Star(inner) | Expr::Plus(inner) => {
                if matches!(expr, Expr::Plus(_)) {
                    self.spell(inner, rng, out, depth + 1);
                }
                let mut reps = 0;
                while reps < 3 && rng.chance(50) {
                    self.spell(inner, rng, out, depth + 1);
                    reps += 1;
                }
            }
            Expr::Literal(text) => out.push_str(text),
            Expr::Name(name) => out.push_str(&self.lexeme(name, rng)),
            Expr::Regex(pattern) => {
                panic!("regular expression /{pattern}/ reached while spelling a terminal")
            }
        }
    }
}

fn is_name(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Name(n) if n == name)
}

fn emits_text(toks: &[Tok]) -> bool {
    toks.iter().any(|t| matches!(t, Tok::Text(_)))
}

/// Renders the token stream: NEWLINE emits `\n`; INDENT/DEDENT adjust the
/// tab level applied at the start of the next line (indentation is tabs);
/// lexemes on one line are separated by single spaces.
fn render(toks: &[Tok]) -> String {
    let mut output = String::new();
    let mut level: u32 = 0;
    let mut at_line_start = true;
    for tok in toks {
        match tok {
            Tok::Newline => {
                output.push('\n');
                at_line_start = true;
            }
            Tok::Indent => level += 1,
            Tok::Dedent => level = level.saturating_sub(1),
            Tok::Text(text) => {
                if at_line_start {
                    for _ in 0..level {
                        output.push('\t');
                    }
                    at_line_start = false;
                } else {
                    output.push(' ');
                }
                output.push_str(text);
            }
        }
    }
    output
}

fn expr_cost(expr: &Expr, costs: &IndexMap<String, u32>, grammar: &Grammar) -> u32 {
    match expr {
        Expr::Seq(items) => items
            .iter()
            .map(|i| expr_cost(i, costs, grammar))
            .fold(0u32, |a, b| a.saturating_add(b)),
        Expr::Alt(alts) => alts
            .iter()
            .map(|a| expr_cost(a, costs, grammar))
            .min()
            .unwrap_or(INF),
        Expr::Opt(_) | Expr::Star(_) => 0,
        Expr::Plus(inner) => expr_cost(inner, costs, grammar),
        Expr::Literal(_) => 1,
        // An anonymous regex in a rule has no sample text.
        Expr::Regex(_) => INF,
        Expr::Name(name) if is_terminal_name(name) => {
            if grammar.terminals.contains_key(name) || grammar.declared.contains(name) {
                1
            } else {
                INF
            }
        }
        Expr::Name(name) => costs.get(name).copied().unwrap_or(INF),
    }
}

/// Fixpoint of minimal derivation costs over all rules.
fn compute_min_costs(grammar: &Grammar) -> IndexMap<String, u32> {
    let mut costs: IndexMap<String, u32> = grammar.rules.keys().map(|k| (k.clone(), INF)).collect();
    loop {
        let mut changed = false;
        for (name, rule) in &grammar.rules {
            let cost = expr_cost(&rule.expr, &costs, grammar);
            if cost < costs[name] {
                costs[name] = cost;
                changed = true;
            }
        }
        if !changed {
            return costs;
        }
    }
}
