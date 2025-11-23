use std::collections::HashSet;

use full_moon::ast::{self, Expression, FunctionArgs, FunctionCall, Prefix, Suffix};
use full_moon::tokenizer::TokenType;
use full_moon::visitors::VisitorMut;

#[derive(Debug, Clone)]
pub struct RequireMatch {
    pub module: String,
    pub line: usize,
    pub col: usize,
}

pub fn find_literal_requires(code: &str) -> Vec<RequireMatch> {
    let ast = match full_moon::parse(code) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    struct Collect<'a> {
        found: Vec<RequireMatch>,
        scope_stack: Vec<HashSet<String>>,
        _p: std::marker::PhantomData<&'a ()>,
    }
    impl<'a> Collect<'a> {
        fn in_scope(&self, name: &str) -> bool {
            self.scope_stack.iter().any(|s| s.contains(name))
        }
        fn push(&mut self) {
            self.scope_stack.push(Default::default());
        }
        fn pop(&mut self) {
            self.scope_stack.pop();
        }
        fn declare(&mut self, name: &str) {
            if let Some(s) = self.scope_stack.last_mut() {
                s.insert(name.to_string());
            }
        }
    }
    impl<'a> VisitorMut for Collect<'a> {
        fn visit_block(&mut self, node: ast::Block) -> ast::Block {
            self.push();
            node
        }
        fn visit_block_end(&mut self, node: ast::Block) -> ast::Block {
            self.pop();
            node
        }
        fn visit_local_assignment(&mut self, node: ast::LocalAssignment) -> ast::LocalAssignment {
            for n in node.names().iter() {
                if let TokenType::Identifier { identifier } = n.token().token_type() {
                    self.declare(identifier.as_str());
                }
            }
            node
        }
        fn visit_local_function(&mut self, node: ast::LocalFunction) -> ast::LocalFunction {
            if let TokenType::Identifier { identifier } = node.name().token().token_type() {
                self.declare(identifier.as_str());
            }
            node
        }
        fn visit_anonymous_function(
            &mut self,
            node: ast::AnonymousFunction,
        ) -> ast::AnonymousFunction {
            self.push();
            for p in node.body().parameters().iter() {
                if let ast::Parameter::Name(tok) = p {
                    if let TokenType::Identifier { identifier } = tok.token().token_type() {
                        self.declare(identifier.as_str());
                    }
                }
            }
            node
        }
        fn visit_anonymous_function_end(
            &mut self,
            node: ast::AnonymousFunction,
        ) -> ast::AnonymousFunction {
            self.pop();
            node
        }
        fn visit_function_call(&mut self, node: FunctionCall) -> FunctionCall {
            if let Prefix::Name(tok) = node.prefix() {
                if matches!(tok.token().token_type(), TokenType::Identifier { identifier } if identifier.as_str() == "require")
                    && !self.in_scope("require")
                {
                    if let Some(Suffix::Call(ast::Call::AnonymousCall(args))) =
                        node.suffixes().next()
                    {
                        match args {
                            FunctionArgs::Parentheses { arguments, .. } => {
                                let mut it = arguments.iter();
                                if let Some(Expression::String(_)) = it.next() {
                                    if it.next().is_none() {
                                        if let Some(Expression::String(tokref)) =
                                            arguments.iter().next()
                                        {
                                            if let TokenType::StringLiteral { literal, .. } =
                                                tokref.token().token_type()
                                            {
                                                self.found.push(RequireMatch {
                                                    module: literal.to_string(),
                                                    line: 0,
                                                    col: 0,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            FunctionArgs::String(tokref) => {
                                if let TokenType::StringLiteral { literal, .. } =
                                    tokref.token().token_type()
                                {
                                    self.found.push(RequireMatch {
                                        module: literal.to_string(),
                                        line: 0,
                                        col: 0,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            node
        }
    }

    let mut v = Collect {
        found: Vec::new(),
        scope_stack: vec![Default::default()],
        _p: std::marker::PhantomData,
    };
    let _ = v.visit_ast(ast);
    v.found
}
