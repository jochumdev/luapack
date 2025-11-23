use std::collections::HashSet;
use std::path::Path;

use full_moon::ast::{self, Expression, FunctionArgs, FunctionCall, Prefix, Suffix};
use full_moon::tokenizer::{StringLiteralQuoteType, Token, TokenReference, TokenType};
use full_moon::visitors::VisitorMut;
use glob::Pattern;

use crate::normalize::normalize_module_name;
use crate::options::NameNormalizer;
use crate::replace::{ArgMode, MatchKind, ReplaceRule};
use crate::resolve::ModuleResolver;

fn rule_applies_to_file(rule: &ReplaceRule, file: &Path) -> bool {
    if rule.paths.is_empty() {
        return true;
    }
    let file_str = file.to_string_lossy().replace('\\', "/");
    for pat in &rule.paths {
        if let Ok(p) = Pattern::new(pat) {
            if p.matches(&file_str) {
                return true;
            }
        } else if file_str.contains(pat) {
            return true;
        }
    }
    false
}

fn apply_replace(
    module: &str,
    r: &ReplaceRule,
    resolver: Option<&ModuleResolver>,
    normalizer: &NameNormalizer,
) -> Option<(String, String)> {
    let module_n = normalize_module_name(module, normalizer);
    match r.match_kind {
        MatchKind::Exact => {
            if let Some(ref n) = r.name {
                if n == &module_n {
                    let arg = match r.arg {
                        ArgMode::Rest => module_n.to_string(),
                        ArgMode::Full => module_n.to_string(),
                    };
                    return Some((r.new.clone(), arg));
                }
            }
        }
        MatchKind::Prefix => {
            if let Some(ref p) = r.prefix {
                if module_n.starts_with(p) {
                    let rest = module_n[p.len()..].to_string();
                    let arg = match r.arg {
                        ArgMode::Rest => rest,
                        ArgMode::Full => module_n.to_string(),
                    };
                    return Some((r.new.clone(), arg));
                }
            }
        }
        MatchKind::Path => {
            if let Some(res) = resolver {
                if let Some(path) = res.resolve(&module_n) {
                    let path_str = path.to_string_lossy().replace('\\', "/");
                    for pat in &r.paths {
                        if let Ok(p) = Pattern::new(pat) {
                            if p.matches(&path_str) {
                                let arg = match r.arg {
                                    ArgMode::Rest => module_n.to_string(),
                                    ArgMode::Full => module_n.to_string(),
                                };
                                return Some((r.new.clone(), arg));
                            }
                        } else if path_str.contains(pat) {
                            let arg = match r.arg {
                                ArgMode::Rest => module_n.to_string(),
                                ArgMode::Full => module_n.to_string(),
                            };
                            return Some((r.new.clone(), arg));
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn transform_requires(
    code: &str,
    rules: &[ReplaceRule],
    file_path: Option<&Path>,
    resolver: Option<&ModuleResolver>,
    normalizer: &NameNormalizer,
) -> (String, usize) {
    if rules.is_empty() {
        return (code.to_string(), 0);
    }
    let ast = match full_moon::parse(code) {
        Ok(ast) => ast,
        Err(_) => {
            return (code.to_string(), 0);
        }
    };

    struct RequireRewriter<'a> {
        rules: &'a [ReplaceRule],
        file: Option<&'a Path>,
        scope_stack: Vec<HashSet<String>>,
        rewrites: usize,
        resolver: Option<&'a ModuleResolver>,
        normalizer: &'a NameNormalizer,
    }
    impl<'a> full_moon::visitors::VisitorMut for RequireRewriter<'a> {
        fn visit_block(&mut self, node: ast::Block) -> ast::Block {
            self.push_scope();
            node
        }
        fn visit_block_end(&mut self, node: ast::Block) -> ast::Block {
            self.pop_scope();
            node
        }
        fn visit_local_assignment(&mut self, node: ast::LocalAssignment) -> ast::LocalAssignment {
            for name in node.names().iter() {
                if let Some(s) = Self::token_ident_text(name) {
                    self.declare(&s);
                }
            }
            node
        }
        fn visit_local_function(&mut self, node: ast::LocalFunction) -> ast::LocalFunction {
            if let Some(s) = Self::token_ident_text(node.name()) {
                self.declare(&s);
            }
            node
        }
        fn visit_function_declaration(
            &mut self,
            node: ast::FunctionDeclaration,
        ) -> ast::FunctionDeclaration {
            if let Some(first) = node.name().names().iter().next() {
                if let Some(s) = Self::token_ident_text(first) {
                    self.declare(&s);
                }
            }
            node
        }
        fn visit_anonymous_function(
            &mut self,
            node: ast::AnonymousFunction,
        ) -> ast::AnonymousFunction {
            self.push_scope();
            for param in node.body().parameters().iter() {
                if let ast::Parameter::Name(tok) = param {
                    if let Some(s) = Self::token_ident_text(tok) {
                        self.declare(&s);
                    }
                }
            }
            node
        }
        fn visit_anonymous_function_end(
            &mut self,
            node: ast::AnonymousFunction,
        ) -> ast::AnonymousFunction {
            self.pop_scope();
            node
        }
        fn visit_function_call(&mut self, node: FunctionCall) -> FunctionCall {
            let mut new_node = node;
            let is_global_require = match new_node.prefix() {
                Prefix::Name(tok) => {
                    Self::token_is_ident(tok, "require") && !self.in_scope("require")
                }
                _ => false,
            };
            if !is_global_require {
                return new_node;
            }

            let mut suffixes = new_node.suffixes().cloned().collect::<Vec<_>>();
            let Some(first_call_idx) = suffixes.iter().position(|s| matches!(s, Suffix::Call(_)))
            else {
                return new_node;
            };
            let Suffix::Call(ast::Call::AnonymousCall(args_ref)) = &suffixes[first_call_idx] else {
                return new_node;
            };

            let mut arg_token_ref_opt: Option<TokenReference> = None;
            let mut build_args: Option<Box<dyn Fn(TokenReference) -> FunctionArgs>> = None;
            let args_owned = args_ref.clone();
            match args_owned {
                FunctionArgs::Parentheses {
                    arguments,
                    parentheses,
                } => {
                    let mut it = arguments.iter();
                    if let Some(Expression::String(tok)) = it.next() {
                        if it.next().is_none() {
                            arg_token_ref_opt = Some(tok.clone());
                            let par = parentheses.clone();
                            build_args = Some(Box::new(move |new_tok: TokenReference| {
                                let mut new_punct = ast::punctuated::Punctuated::new();
                                new_punct
                                    .push(ast::punctuated::Pair::End(Expression::String(new_tok)));
                                FunctionArgs::Parentheses {
                                    parentheses: par.clone(),
                                    arguments: new_punct,
                                }
                            }));
                        }
                    }
                }
                FunctionArgs::String(tok) => {
                    arg_token_ref_opt = Some(tok);
                    build_args = Some(Box::new(|new_tok: TokenReference| {
                        FunctionArgs::String(new_tok)
                    }));
                }
                FunctionArgs::TableConstructor(_) => {}
                _ => {}
            }

            let Some(arg_tok) = arg_token_ref_opt else {
                return new_node;
            };
            let Some((module_name, quote, depth)) = Self::string_literal_parts(&arg_tok) else {
                return new_node;
            };

            let mut replaced: Option<(String, String)> = None;
            for r in self.rules {
                if r.match_kind != MatchKind::Path && !self.path_rule_allows(r) {
                    continue;
                }
                if let Some((new_callee, new_arg)) =
                    apply_replace(&module_name, r, self.resolver, self.normalizer)
                {
                    replaced = Some((new_callee, new_arg));
                    break;
                }
            }
            let Some((new_callee, new_arg)) = replaced else {
                return new_node;
            };

            let new_ident = Self::make_ident(&new_callee);
            let new_arg_tok = Self::make_string(&new_arg, quote, depth, &arg_tok);

            let builder = match build_args {
                Some(b) => b,
                None => return new_node,
            };
            let new_args = builder(new_arg_tok);
            suffixes[first_call_idx] = Suffix::Call(ast::Call::AnonymousCall(new_args));
            new_node = new_node
                .with_prefix(Prefix::Name(new_ident))
                .with_suffixes(suffixes);
            self.rewrites += 1;
            new_node
        }
    }
    impl<'a> RequireRewriter<'a> {
        fn in_scope(&self, name: &str) -> bool {
            self.scope_stack.iter().any(|s| s.contains(name))
        }
        fn push_scope(&mut self) {
            self.scope_stack.push(Default::default());
        }
        fn pop_scope(&mut self) {
            self.scope_stack.pop();
        }
        fn declare(&mut self, name: &str) {
            if let Some(s) = self.scope_stack.last_mut() {
                s.insert(name.to_string());
            }
        }
        fn token_is_ident(token: &TokenReference, expected: &str) -> bool {
            matches!(token.token().token_type(), TokenType::Identifier { identifier } if identifier.as_str() == expected)
        }
        fn token_ident_text(token: &TokenReference) -> Option<String> {
            if let TokenType::Identifier { identifier } = token.token().token_type() {
                Some(identifier.to_string())
            } else {
                None
            }
        }
        fn string_literal_parts(
            token: &TokenReference,
        ) -> Option<(String, StringLiteralQuoteType, usize)> {
            if let TokenType::StringLiteral {
                literal,
                multi_line_depth,
                quote_type,
            } = token.token().token_type()
            {
                Some((literal.to_string(), *quote_type, *multi_line_depth))
            } else {
                None
            }
        }
        fn make_ident(name: &str) -> TokenReference {
            let t = Token::new(TokenType::Identifier {
                identifier: name.into(),
            });
            TokenReference::new(vec![], t, vec![])
        }
        fn make_string(
            lit: &str,
            quote: StringLiteralQuoteType,
            depth: usize,
            keep_trivia_from: &TokenReference,
        ) -> TokenReference {
            let t = Token::new(TokenType::StringLiteral {
                literal: lit.into(),
                multi_line_depth: depth,
                quote_type: quote,
            });
            keep_trivia_from.with_token(t)
        }
        fn path_rule_allows(&self, rule: &ReplaceRule) -> bool {
            if let Some(f) = self.file {
                rule_applies_to_file(rule, f)
            } else {
                true
            }
        }
    }

    let mut v = RequireRewriter {
        rules,
        file: file_path,
        scope_stack: vec![Default::default()],
        rewrites: 0,
        resolver,
        normalizer,
    };
    let new_ast = v.visit_ast(ast);
    (new_ast.to_string(), v.rewrites)
}
