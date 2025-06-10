use pest::Parser;
use clean_language_compiler::parser::CleanParser;
use clean_language_compiler::parser::Rule;

fn main() {
    let source = "integer add()\n\tinput\n\t\tinteger a\n\t\tinteger b\n\t\n\treturn a + b";
    
    println!("Testing function_in_block parsing:");
    println!("Source bytes: {:?}", source.bytes().collect::<Vec<_>>());
    println!("Source chars:");
    for (i, c) in source.chars().enumerate() {
        match c {
            '\n' => println!("  {}: NEWLINE", i),
            '\t' => println!("  {}: TAB", i),
            ' ' => println!("  {}: SPACE", i),
            _ => println!("  {}: {}", i, c),
        }
    }
    println!();
    
    match CleanParser::parse(Rule::function_in_block, source) {
        Ok(pairs) => {
            for pair in pairs {
                print_parse_tree(pair, 0);
            }
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}

fn print_parse_tree(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}Rule::{:?} -> {:?}", indent_str, pair.as_rule(), pair.as_str());
    
    for inner_pair in pair.into_inner() {
        print_parse_tree(inner_pair, indent + 1);
    }
} 