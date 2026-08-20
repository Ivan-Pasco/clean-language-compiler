//! Grammar-seeded program generator (M9). Expands `SourceFile` from the
//! vendored EBNF with a deterministic PRNG, so every generated program is a
//! sentence of the DOC-15 grammar (modulo the pinned gap table below) and
//! every failure reproduces from its seed alone.

// Shared by several test binaries; not every binary uses every item.
#![allow(dead_code)]

use super::ebnf::{Expr, Grammar};
use indexmap::IndexMap;

/// Cost of a derivation, in emitted tokens. `INF` marks productions that
/// cannot be generated (their only bodies point outside the vendored
/// grammar); alternatives priced `INF` are never chosen.
const INF: u32 = u32::MAX / 4;

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

/// Grammar gaps pinned to a local interpretation while foundation decides.
/// Empty since the 2026-08-20 errata defined every M9 gap in the grammar
/// itself (ConstantBody in 05 §2, TestsBody in 11 §1, CallExpression in
/// 18); repopulate — with the DISCOVERIES entry — if new gaps appear. The
/// trip-wire in `Generator::new` fires when a pinned name gets a real
/// definition.
pub fn pinned_gap_rules() -> Vec<(&'static str, Expr)> {
    Vec::new()
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
}

impl Generator {
    pub fn new(mut grammar: Grammar) -> Generator {
        for (name, expr) in pinned_gap_rules() {
            assert!(
                !grammar.rules.contains_key(name),
                "gap rule {name} is now defined by the grammar — drop it \
                 from the pinned gap table and update DISCOVERIES-M9"
            );
            grammar.rules.insert(
                name.to_string(),
                super::ebnf::Rule {
                    expr,
                    file: "pinned-gap-table".to_string(),
                },
            );
        }
        let min_cost = compute_min_costs(&grammar);
        Generator { grammar, min_cost }
    }

    /// Every non-terminal reachable from `SourceFile` must have a finite
    /// derivation; anything infinite is either a new grammar gap or a
    /// generator bug, and the fuzzer refuses to run rather than silently
    /// under-covering.
    pub fn assert_root_generatable(&self) {
        let cost = self.min_cost.get("SourceFile").copied().unwrap_or(INF);
        assert!(
            cost < INF,
            "SourceFile has no finite derivation — vendored grammar changed?"
        );
    }

    pub fn ungeneratable(&self) -> Vec<&str> {
        self.min_cost
            .iter()
            .filter(|(_, &c)| c >= INF)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Generates one program from the given seed. `budget` caps emitted
    /// tokens; once exceeded, expansion always takes the cheapest branch,
    /// so termination is structural, not probabilistic.
    pub fn program(&self, seed: u64, budget: u32) -> String {
        let mut rng = Rng::new(seed);
        let mut toks = Vec::new();
        let mut spent = 0u32;
        self.expand(
            &Expr::NonTerm("SourceFile".to_string()),
            &mut rng,
            budget,
            &mut spent,
            &mut toks,
            0,
            0,
        );
        render(&toks)
    }

    fn cost(&self, expr: &Expr) -> u32 {
        expr_cost(expr, &self.min_cost)
    }

    #[allow(clippy::too_many_arguments)]
    fn expand(
        &self,
        expr: &Expr,
        rng: &mut Rng,
        budget: u32,
        spent: &mut u32,
        out: &mut Vec<Tok>,
        lexical_depth: u32,
        depth: u32,
    ) {
        // Past either bound, expansion always takes the cheapest branch, so
        // recursion depth stays within what default 2 MiB thread stacks
        // handle on both the generator's and the compiler's side.
        let over_budget = *spent >= budget || depth >= 64;
        match expr {
            Expr::Seq(items) => {
                for item in items {
                    self.expand(item, rng, budget, spent, out, lexical_depth, depth + 1);
                }
            }
            Expr::Alt(alts) => {
                let viable: Vec<&Expr> = alts.iter().filter(|a| self.cost(a) < INF).collect();
                assert!(!viable.is_empty(), "alternation with no finite branch");
                let chosen = if over_budget {
                    viable
                        .iter()
                        .min_by_key(|a| self.cost(a))
                        .expect("non-empty")
                } else {
                    &viable[rng.below(viable.len())]
                };
                self.expand(chosen, rng, budget, spent, out, lexical_depth, depth + 1);
            }
            Expr::Opt(inner) => {
                if !over_budget && self.cost(inner) < INF && rng.chance(40) {
                    self.expand(inner, rng, budget, spent, out, lexical_depth, depth + 1);
                }
            }
            Expr::Rep(inner) => {
                if over_budget || self.cost(inner) >= INF {
                    return;
                }
                let mut reps = 0;
                while reps < 3 && rng.chance(50) && *spent < budget {
                    self.expand(inner, rng, budget, spent, out, lexical_depth, depth + 1);
                    reps += 1;
                }
            }
            Expr::Terminal(text) => {
                *spent += 1;
                out.push(Tok::Text(text.clone()));
            }
            Expr::NonTerm(name) => {
                let rule = self
                    .grammar
                    .rules
                    .get(name)
                    .unwrap_or_else(|| panic!("undefined non-terminal {name}"));
                // Crossing from syntax into 03-lexical-structure builds one
                // lexeme: the whole expansion flattens so its pieces
                // concatenate with no separating space.
                let entering_lexeme =
                    lexical_depth == 0 && rule.file.starts_with("03-lexical-structure");
                if entering_lexeme {
                    let mut lexeme = Vec::new();
                    self.expand(&rule.expr, rng, budget, spent, &mut lexeme, 1, depth + 1);
                    out.extend(flatten_lexeme(lexeme));
                } else {
                    self.expand(
                        &rule.expr,
                        rng,
                        budget,
                        spent,
                        out,
                        lexical_depth,
                        depth + 1,
                    );
                }
            }
            Expr::Special(text) => {
                *spent += 1;
                self.expand_special(text, rng, out);
            }
            Expr::Except(base, exceptions) => {
                // Generate-and-retry: draw from the base until the drawn
                // lexeme is not one of the excepted terminals.
                for _ in 0..64 {
                    let mut attempt = Vec::new();
                    let mut sub_spent = 0;
                    self.expand(
                        base,
                        rng,
                        budget,
                        &mut sub_spent,
                        &mut attempt,
                        1,
                        depth + 1,
                    );
                    let text: String = attempt
                        .iter()
                        .map(|t| match t {
                            Tok::Text(s) => s.as_str(),
                            Tok::Newline => "\n",
                            Tok::Indent | Tok::Dedent => "",
                        })
                        .collect();
                    let excluded = exceptions.iter().any(|e| match e {
                        Expr::Terminal(t) => *t == text,
                        Expr::NonTerm(n) => match n.as_str() {
                            "LF" => text == "\n",
                            "CR" => text == "\r",
                            _ => false,
                        },
                        _ => false,
                    });
                    if !excluded {
                        *spent += 1;
                        out.push(Tok::Text(text));
                        return;
                    }
                }
                panic!("exception filter rejected 64 straight draws");
            }
        }
    }

    fn expand_special(&self, text: &str, rng: &mut Rng, out: &mut Vec<Tok>) {
        match text {
            "one line terminator event" => out.push(Tok::Newline),
            "one tab of additional indentation, per LEX-01" => out.push(Tok::Indent),
            "one tab of reduced indentation, per LEX-01" => out.push(Tok::Dedent),
            "EOF" => {}
            "one U+0009 character" => out.push(Tok::Text("\t".into())),
            "one U+000A character" => out.push(Tok::Text("\n".into())),
            "one U+000D character" => out.push(Tok::Text("\r".into())),
            "one U+0020 character" => out.push(Tok::Text(" ".into())),
            "one character in 0-9" => out.push(Tok::Text(pick_char(rng, "0123456789"))),
            "one character in 0-7" => out.push(Tok::Text(pick_char(rng, "01234567"))),
            "one character in A-Z or a-z" => out.push(Tok::Text(pick_char(
                rng,
                "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
            ))),
            "one character in a-f or A-F" => out.push(Tok::Text(pick_char(rng, "abcdefABCDEF"))),
            // TXT-01 permits any Unicode scalar; the generator draws from a
            // set that keeps line structure intact plus a few multi-byte
            // scalars so UTF-8 handling is exercised.
            "any Unicode scalar value permitted by TXT-01" => out.push(Tok::Text(pick_char(
                rng,
                "abcXYZ019 _.,;:!$%&=+-*/<>()[]{}#@~^|áé漢🙂",
            ))),
            // LEX-06 multi-line string content: one uninterpreted line that
            // is not the close delimiter, newline included.
            "any line whose content is not exactly the close delimiter — \
             text is uninterpreted inside" => {
                let mut line = String::new();
                for _ in 0..rng.below(8) {
                    line.push_str(&pick_char(rng, "abcXYZ019 _.,:!%&=+-*<>#áé漢"));
                }
                out.push(Tok::Text(line));
                out.push(Tok::Newline);
            }
            other => panic!(
                "special sequence with no generator mapping: ? {other} ? — \
                 the vendored grammar grew a new special; extend the table"
            ),
        }
    }
}

fn pick_char(rng: &mut Rng, set: &str) -> String {
    let chars: Vec<char> = set.chars().collect();
    chars[rng.below(chars.len())].to_string()
}

/// Flattens one lexical expansion into a single lexeme: consecutive text
/// pieces concatenate with no separating space; structural tokens
/// (NEWLINE events inside comment productions) pass through and start a
/// fresh piece.
fn flatten_lexeme(pieces: Vec<Tok>) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut current = String::new();
    for piece in pieces {
        match piece {
            Tok::Text(text) => current.push_str(&text),
            structural => {
                if !current.is_empty() {
                    out.push(Tok::Text(std::mem::take(&mut current)));
                }
                out.push(structural);
            }
        }
    }
    if !current.is_empty() {
        out.push(Tok::Text(current));
    }
    out
}

/// Renders the token stream: NEWLINE emits `\n`; INDENT/DEDENT adjust the
/// tab level applied at the start of the next line (LEX-01: indentation is
/// tabs); lexemes on one line are separated by single spaces.
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

fn expr_cost(expr: &Expr, costs: &IndexMap<String, u32>) -> u32 {
    match expr {
        Expr::Seq(items) => items
            .iter()
            .map(|i| expr_cost(i, costs))
            .fold(0u32, |a, b| a.saturating_add(b)),
        Expr::Alt(alts) => alts
            .iter()
            .map(|a| expr_cost(a, costs))
            .min()
            .unwrap_or(INF),
        Expr::Opt(_) | Expr::Rep(_) => 0,
        Expr::Terminal(_) => 1,
        Expr::NonTerm(name) => costs.get(name).copied().unwrap_or(INF),
        Expr::Special(text) => match text.as_str() {
            // Cross-repo references are not generatable from this grammar.
            t if t.starts_with("see ") => INF,
            "handler-defined body" => INF,
            // 21 §BlockArg payloads (ExpressionType / IdentifierType):
            // schema-tier compile-time types defined in schema/block-ast.md,
            // officially not source syntax (2026-08-20 erratum for 1g).
            t if t.contains("schema-tier") => INF,
            // Prefer a real line terminator over the EOF branch so comments
            // usually end their line; EOF still gets drawn at random.
            "EOF" => 3,
            _ => 1,
        },
        Expr::Except(base, _) => expr_cost(base, costs),
    }
}

/// Fixpoint of minimal derivation costs over all rules.
fn compute_min_costs(grammar: &Grammar) -> IndexMap<String, u32> {
    let mut costs: IndexMap<String, u32> = grammar.rules.keys().map(|k| (k.clone(), INF)).collect();
    loop {
        let mut changed = false;
        for (name, rule) in &grammar.rules {
            let cost = expr_cost(&rule.expr, &costs);
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
