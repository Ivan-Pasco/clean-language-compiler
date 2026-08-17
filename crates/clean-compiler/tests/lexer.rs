//! Milestone 1 step 5a checks: the lexer against the shapes of
//! `03-lexical-structure.ebnf.md` — tab layout, keywords vs identifiers,
//! literals, comments, and the SYN recovery paths.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::lexer::{lex, plain_text, Kw, StrPart, TokenKind};
use clean_compiler_types::codes;

fn kinds(source: &str) -> (Vec<TokenKind>, Vec<clean_compiler_types::Diagnostic>) {
    let mut sink = DiagnosticSink::new();
    let stream = lex("app/main.cln", source, &mut sink);
    (
        stream.tokens.into_iter().map(|t| t.kind).collect(),
        sink.into_diagnostics(),
    )
}

#[test]
fn layout_tokens_follow_tab_indentation() {
    use TokenKind::*;
    let (tokens, diagnostics) = kinds("functions:\n\tinit()\n");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(
        tokens,
        vec![
            Ident("functions".into()),
            Colon,
            Newline,
            Indent,
            Ident("init".into()),
            LParen,
            RParen,
            Newline,
            Dedent,
            Eof,
        ]
    );
}

#[test]
fn blank_and_comment_lines_do_not_change_layout() {
    use TokenKind::*;
    let source = "functions:\n\n\t// a comment line\n\tinit()\n";
    let (tokens, diagnostics) = kinds(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let indents = tokens.iter().filter(|t| matches!(t, Indent)).count();
    let dedents = tokens.iter().filter(|t| matches!(t, Dedent)).count();
    assert_eq!((indents, dedents), (1, 1));
}

#[test]
fn hard_keywords_lex_as_keywords_contextual_as_identifiers() {
    let (tokens, _) = kinds("if return functions tests integer start\n");
    use TokenKind::*;
    assert_eq!(
        tokens[..6],
        [
            Keyword(Kw::If),
            Keyword(Kw::Return),
            Ident("functions".into()),
            Ident("tests".into()),
            Ident("integer".into()),
            Keyword(Kw::Start),
        ]
    );
}

#[test]
fn keywords_are_case_sensitive() {
    let (tokens, _) = kinds("If IF if\n");
    use TokenKind::*;
    assert_eq!(
        tokens[..3],
        [Ident("If".into()), Ident("IF".into()), Keyword(Kw::If),]
    );
}

#[test]
fn integer_literals_in_every_base() {
    let (tokens, diagnostics) = kinds("42 0xff 0b101 0o17\n");
    assert!(diagnostics.is_empty());
    use TokenKind::*;
    assert_eq!(tokens[..4], [Int(42), Int(255), Int(5), Int(15)]);
}

#[test]
fn dot_boundary_rule_splits_member_access_from_number() {
    use TokenKind::*;
    // "3." is Int(3) then Dot; "3.5" is a NumberLiteral.
    let (tokens, _) = kinds("3.toString() 3.5\n");
    assert_eq!(tokens[0], Int(3));
    assert_eq!(tokens[1], Dot);
    assert_eq!(tokens[2], Ident("toString".into()));
    assert!(matches!(&tokens[5], Number(text) if text == "3.5"));
}

#[test]
fn string_escapes_decode() {
    let source = format!("{}\n", r#""a\tb\"c\{d""#);
    let (tokens, diagnostics) = kinds(&source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    match &tokens[0] {
        TokenKind::Str { parts } => {
            assert_eq!(plain_text(parts).as_deref(), Some("a\tb\"c{d"));
        }
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn interpolation_interior_is_tokenized() {
    let (tokens, diagnostics) = kinds("\"total: {a + b}!\"\n");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    match &tokens[0] {
        TokenKind::Str { parts } => {
            assert_eq!(parts.len(), 3, "{parts:#?}");
            assert_eq!(parts[0], StrPart::Text("total: ".into()));
            match &parts[1] {
                StrPart::Interp { tokens, .. } => {
                    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
                    assert!(
                        matches!(
                            kinds[..],
                            [
                                TokenKind::Ident(_),
                                TokenKind::Plus,
                                TokenKind::Ident(_),
                                TokenKind::Eof
                            ]
                        ),
                        "{kinds:?}"
                    );
                }
                other => panic!("expected interpolation, got {other:?}"),
            }
            assert_eq!(parts[2], StrPart::Text("!".into()));
        }
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn interpolation_admits_nested_string_with_brace() {
    // The nested literal contains "}" — the tokenizer must not mistake it
    // for the interpolation's close.
    let (tokens, diagnostics) = kinds("\"v: {f(\"}\")}\"\n");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    match &tokens[0] {
        TokenKind::Str { parts } => {
            assert_eq!(parts.len(), 2, "{parts:#?}");
            match &parts[1] {
                StrPart::Interp { tokens, .. } => {
                    assert!(
                        matches!(
                            tokens.iter().map(|t| &t.kind).collect::<Vec<_>>()[..],
                            [
                                TokenKind::Ident(_),
                                TokenKind::LParen,
                                TokenKind::Str { .. },
                                TokenKind::RParen,
                                TokenKind::Eof
                            ]
                        ),
                        "{tokens:#?}"
                    );
                }
                other => panic!("expected interpolation, got {other:?}"),
            }
        }
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn unclosed_interpolation_is_syn004_once() {
    let (_, diagnostics) = kinds("\"v: {a + b\n");
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, codes::SYN004);
}

#[test]
fn braces_outside_strings_lex_as_tokens() {
    // §8 lists LBrace/RBrace as tokens; the parser, not the lexer, rejects
    // them where the grammar has no production.
    let (tokens, diagnostics) = kinds("{ }\n");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    use TokenKind::*;
    assert_eq!(tokens[..2], [LBrace, RBrace]);
}

#[test]
fn reserved_unused_words_are_syn002_but_survive_as_identifiers() {
    let (tokens, diagnostics) = kinds("for from unit\n");
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|d| d.code == codes::SYN002));
    use TokenKind::*;
    assert_eq!(
        tokens[..3],
        [
            Ident("for".into()),
            Ident("from".into()),
            Ident("unit".into())
        ]
    );
}

#[test]
fn unterminated_string_is_syn004() {
    let (_, diagnostics) = kinds("\"never closed\n");
    assert_eq!(diagnostics[0].code, codes::SYN004);
}

#[test]
fn spaces_in_indentation_are_syn003() {
    let (_, diagnostics) = kinds("functions:\n    init()\n");
    assert_eq!(diagnostics[0].code, codes::SYN003);
}

#[test]
fn indentation_jump_is_syn006() {
    let (_, diagnostics) = kinds("functions:\n\t\t\tinit()\n");
    assert_eq!(diagnostics[0].code, codes::SYN006);
}

#[test]
fn invalid_character_is_syn001_and_recovers() {
    let (tokens, diagnostics) = kinds("a § b\n");
    assert_eq!(diagnostics[0].code, codes::SYN001);
    // Both identifiers survive around the bad character.
    use TokenKind::*;
    assert_eq!(tokens[0], Ident("a".into()));
    assert_eq!(tokens[1], Ident("b".into()));
}

#[test]
fn nested_block_comments_track_depth() {
    let (tokens, diagnostics) = kinds("a /* outer /* inner */ still */ b\n");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    use TokenKind::*;
    assert_eq!(tokens[0], Ident("a".into()));
    assert_eq!(tokens[1], Ident("b".into()));
}

#[test]
fn unclosed_block_comment_is_syn004() {
    let (_, diagnostics) = kinds("a /* never closed\n");
    assert_eq!(diagnostics[0].code, codes::SYN004);
}

#[test]
fn multiline_string_removes_close_margin() {
    let source = "\t\tstring s = \"\"\"\n\t\thello\n\t\t  world\n\t\t\"\"\"\n";
    // Note: the two leading tabs are indentation; run inside a block.
    let full = format!("functions:\n\tf()\n{source}");
    let mut sink = DiagnosticSink::new();
    let stream = lex("app/main.cln", &full, &mut sink);
    let string_token = stream
        .tokens
        .iter()
        .find_map(|t| match &t.kind {
            TokenKind::Str { parts } => plain_text(parts),
            _ => None,
        })
        .expect("a string token");
    assert_eq!(string_token, "hello\n  world");
}

#[test]
fn crlf_normalises_and_lone_cr_is_syn001() {
    let (tokens, diagnostics) = kinds("a\r\nb\rc\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, codes::SYN001);
    use TokenKind::*;
    assert_eq!(tokens[0], Ident("a".into()));
    assert_eq!(tokens[1], Newline);
}
