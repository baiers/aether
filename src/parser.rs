use pest_derive::Parser;
use pest::Parser;
use crate::ast::*;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "aether.pest"]
pub struct AetherParser;

pub fn parse_aether(input: &str) -> Result<AetherProgram, Box<dyn std::error::Error + Send + Sync>> {
    let mut roots = Vec::new();
    let pairs = AetherParser::parse(Rule::program, input)
        .map_err(|e| format!("Parse error: {}", e))?;

    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                for inner_pair in pair.into_inner() {
                    if let Rule::root_block = inner_pair.as_rule() {
                        roots.push(parse_root(inner_pair)?);
                    }
                }
            }
            Rule::EOI => (),
            _ => (),
        }
    }

    Ok(AetherProgram { roots })
}

fn parse_root(pair: pest::iterators::Pair<Rule>) -> Result<RootBlock, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let mut blocks = Vec::new();

    for b_pair in inner {
        if let Rule::block = b_pair.as_rule() {
            let inner_block = b_pair.into_inner().next().unwrap();
            blocks.push(parse_block(inner_block)?);
        }
    }

    Ok(RootBlock { id, blocks })
}

fn parse_block(pair: pest::iterators::Pair<Rule>) -> Result<Block, Box<dyn std::error::Error + Send + Sync>> {
    match pair.as_rule() {
        Rule::context_block => Ok(Block::Context(parse_context(pair)?)),
        Rule::action_node => Ok(Block::Action(parse_action(pair)?)),
        Rule::request_block => Ok(Block::Request(parse_request(pair)?)),
        Rule::failure_block => Ok(Block::Failure(parse_failure(pair)?)),
        Rule::parallel_block => Ok(Block::Parallel(parse_parallel(pair)?)),
        _ => Err(format!("Unknown block type: {:?}", pair.as_rule()).into()),
    }
}

fn parse_context(pair: pest::iterators::Pair<Rule>) -> Result<ContextBlock, Box<dyn std::error::Error + Send + Sync>> {
    let mut id = None;
    let mut data = HashMap::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::hash => { id = Some(p.as_str().to_string()); }
            Rule::ctx_pair => {
                let mut inner = p.into_inner();
                let first = inner.next().unwrap();
                let k = first.as_str().to_string();
                let v = parse_value(inner.next().unwrap())?;
                data.insert(k, v);
            }
            _ => (),
        }
    }

    Ok(ContextBlock { id, data })
}

fn parse_action(pair: pest::iterators::Pair<Rule>) -> Result<ActionNode, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let content = inner.next().unwrap();

    let mut meta = None;
    let mut inputs = None;
    let mut condition = None;
    let mut language = GuestLang::Python;
    let mut code = String::new();
    let mut outputs = None;
    let mut validation = None;
    let mut depends_on = Vec::new();

    for part in content.into_inner() {
        match part.as_rule() {
            Rule::meta_block => { meta = Some(parse_meta(part)?); }
            Rule::in_block => {
                let (bindings, deps) = parse_in_block(part)?;
                inputs = Some(bindings);
                depends_on = deps;
            }
            Rule::condition_block => {
                let expr_pair = part.into_inner().next().unwrap();
                condition = Some(parse_expression(expr_pair)?);
            }
            Rule::exec_block => {
                let mut exec_inner = part.into_inner();
                let lang_str = exec_inner.next().unwrap().as_str();
                language = GuestLang::from_str(lang_str)
                    .ok_or_else(|| format!("Unknown language: {}", lang_str))?;
                code = exec_inner.next().unwrap().as_str().trim().to_string();
            }
            Rule::out_block => { outputs = Some(parse_out_block(part)?); }
            Rule::val_block => { validation = Some(parse_val_block(part)?); }
            _ => (),
        }
    }

    Ok(ActionNode {
        id, meta, inputs, condition, language, code, outputs, validation, depends_on,
    })
}

fn parse_meta(pair: pest::iterators::Pair<Rule>) -> Result<MetaBlock, Box<dyn std::error::Error + Send + Sync>> {
    let mut intent = None;
    let mut safety = None;
    let mut extra = HashMap::new();

    for p in pair.into_inner() {
        if let Rule::pair = p.as_rule() {
            let (k, v) = parse_pair(p)?;
            match k.as_str() {
                "_intent" => {
                    intent = v.as_str().map(|s| s.trim_matches('"').to_string());
                }
                "_safety" => {
                    if let Some(s) = v.as_str() {
                        let cleaned = s.trim_matches('"');
                        safety = SafetyLevel::from_str(cleaned);
                    }
                }
                _ => { extra.insert(k, v); }
            }
        }
    }

    Ok(MetaBlock { intent, safety, extra })
}

fn parse_in_block(pair: pest::iterators::Pair<Rule>) -> Result<(Vec<InputBinding>, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    let mut bindings = Vec::new();
    let mut deps = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ref_pair => {
                let mut inner = p.into_inner();
                let address = inner.next().unwrap().as_str().to_string();
                // Check for optional alias
                let mut alias = None;
                let mut ref_target = String::new();
                for sub in inner {
                    match sub.as_rule() {
                        Rule::alias => {
                            alias = Some(sub.into_inner().next().unwrap().as_str().to_string());
                        }
                        Rule::address => {
                            ref_target = sub.as_str().to_string();
                        }
                        _ => (),
                    }
                }
                // Track dependency: the ref target's hex prefix is the source node
                deps.push(ref_target.clone());
                bindings.push(InputBinding {
                    address,
                    alias,
                    source: InputSource::Ref(ref_target),
                });
            }
            Rule::value_pair => {
                let mut inner = p.into_inner();
                let address = inner.next().unwrap().as_str().to_string();
                let val = parse_value(inner.next().unwrap())?;
                bindings.push(InputBinding {
                    address,
                    alias: None,
                    source: InputSource::Literal(val),
                });
            }
            _ => (),
        }
    }

    Ok((bindings, deps))
}

fn parse_out_block(pair: pest::iterators::Pair<Rule>) -> Result<Vec<OutputBinding>, Box<dyn std::error::Error + Send + Sync>> {
    let mut bindings = Vec::new();
    for p in pair.into_inner() {
        if let Rule::type_pair = p.as_rule() {
            let mut inner = p.into_inner();
            let address = inner.next().unwrap().as_str().to_string();
            let type_str = inner.next().unwrap().as_str();
            let declared_type = AetherType::from_str(type_str)
                .ok_or_else(|| format!("Unknown type: {}", type_str))?;
            bindings.push(OutputBinding { address, declared_type });
        }
    }
    Ok(bindings)
}

fn parse_val_block(pair: pest::iterators::Pair<Rule>) -> Result<Vec<Assertion>, Box<dyn std::error::Error + Send + Sync>> {
    let mut assertions = Vec::new();
    for p in pair.into_inner() {
        if let Rule::assertion = p.as_rule() {
            let mut inner = p.into_inner();
            let expr_pair = inner.next().unwrap();
            let condition = parse_expression(expr_pair)?;
            let action_pair = inner.next().unwrap();
            let on_fail = parse_halt_action(action_pair)?;
            assertions.push(Assertion { condition, on_fail });
        }
    }
    Ok(assertions)
}

fn parse_halt_action(pair: pest::iterators::Pair<Rule>) -> Result<HaltAction, Box<dyn std::error::Error + Send + Sync>> {
    let text = pair.as_str();
    let inner: Vec<_> = pair.into_inner().collect();
    if text.starts_with("HALT") {
        Ok(HaltAction::Halt)
    } else if text.starts_with("RETRY") {
        let retries = inner.first()
            .and_then(|p| p.as_str().parse::<u32>().ok());
        Ok(HaltAction::Retry(retries))
    } else if text.starts_with("WARN") {
        Ok(HaltAction::Warn)
    } else {
        Err(format!("Unknown halt action: {}", text).into())
    }
}

fn parse_request(pair: pest::iterators::Pair<Rule>) -> Result<RequestBlock, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let content = inner.next().unwrap();

    let mut sender = None;
    let mut target = None;
    let mut context = None;
    let mut instructions = None;
    let mut data = HashMap::new();

    for p in content.into_inner() {
        match p.as_rule() {
            Rule::sender_field => {
                sender = Some(p.into_inner().next().unwrap().as_str()
                    .trim_matches('"').to_string());
            }
            Rule::target_field => {
                target = Some(p.into_inner().next().unwrap().as_str()
                    .trim_matches('"').to_string());
            }
            Rule::context_field => {
                let mut ctx = HashMap::new();
                for cp in p.into_inner() {
                    if let Rule::pair = cp.as_rule() {
                        let (k, v) = parse_pair(cp)?;
                        ctx.insert(k, v);
                    }
                }
                context = Some(ctx);
            }
            Rule::instructions_field => {
                instructions = Some(p.into_inner().next().unwrap().as_str()
                    .trim_matches('"').to_string());
            }
            Rule::pair => {
                let (k, v) = parse_pair(p)?;
                data.insert(k, v);
            }
            _ => (),
        }
    }

    Ok(RequestBlock { id, sender, target, context, instructions, data })
}

fn parse_failure(pair: pest::iterators::Pair<Rule>) -> Result<FailureBlock, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner = pair.into_inner();
    let id = inner.next().unwrap().as_str().to_string();
    let mut data = HashMap::new();
    for p in inner {
        if let Rule::pair = p.as_rule() {
            let (k, v) = parse_pair(p)?;
            data.insert(k, v);
        }
    }
    Ok(FailureBlock { id, data })
}

fn parse_parallel(pair: pest::iterators::Pair<Rule>) -> Result<ParallelBlock, Box<dyn std::error::Error + Send + Sync>> {
    let mut id = None;
    let mut nodes = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::hash => { id = Some(p.as_str().to_string()); }
            Rule::action_node => { nodes.push(parse_action(p)?); }
            _ => (),
        }
    }

    Ok(ParallelBlock { id, nodes })
}

// --- Expression parsing ---

fn parse_expression(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let inner = pair.into_inner().next().unwrap();
    parse_or_expr(inner)
}

fn parse_or_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut parts: Vec<_> = pair.into_inner().collect();
    let mut left = parse_and_expr(parts.remove(0))?;
    for part in parts {
        let right = parse_and_expr(part)?;
        left = Expr::BinOp {
            left: Box::new(left),
            op: BinOperator::Or,
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn parse_and_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut parts: Vec<_> = pair.into_inner().collect();
    let mut left = parse_not_expr(parts.remove(0))?;
    for part in parts {
        let right = parse_not_expr(part)?;
        left = Expr::BinOp {
            left: Box::new(left),
            op: BinOperator::And,
            right: Box::new(right),
        };
    }
    Ok(left)
}

fn parse_not_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner: Vec<_> = pair.into_inner().collect();
    if inner.len() == 2 {
        // ! <not_expr>
        let expr = parse_not_expr(inner.remove(1))?;
        Ok(Expr::UnaryOp { op: UnaryOperator::Not, expr: Box::new(expr) })
    } else {
        parse_comparison(inner.remove(0))
    }
}

fn parse_comparison(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner: Vec<_> = pair.into_inner().collect();
    let left = parse_additive(inner.remove(0))?;
    if inner.len() >= 2 {
        let op_str = inner.remove(0).as_str();
        let op = match op_str {
            "==" => BinOperator::Eq,
            "!=" => BinOperator::Ne,
            ">" => BinOperator::Gt,
            "<" => BinOperator::Lt,
            ">=" => BinOperator::Ge,
            "<=" => BinOperator::Le,
            _ => return Err(format!("Unknown comparison op: {}", op_str).into()),
        };
        let right = parse_additive(inner.remove(0))?;
        Ok(Expr::BinOp { left: Box::new(left), op, right: Box::new(right) })
    } else {
        Ok(left)
    }
}

fn parse_additive(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner: Vec<_> = pair.into_inner().collect();
    let mut left = parse_multiplicative(inner.remove(0))?;
    while !inner.is_empty() {
        let op_str = inner.remove(0).as_str();
        let op = match op_str {
            "+" => BinOperator::Add,
            "-" => BinOperator::Sub,
            _ => return Err(format!("Unknown additive op: {}", op_str).into()),
        };
        let right = parse_multiplicative(inner.remove(0))?;
        left = Expr::BinOp { left: Box::new(left), op, right: Box::new(right) };
    }
    Ok(left)
}

fn parse_multiplicative(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner: Vec<_> = pair.into_inner().collect();
    let mut left = parse_unary(inner.remove(0))?;
    while !inner.is_empty() {
        let op_str = inner.remove(0).as_str();
        let op = match op_str {
            "*" => BinOperator::Mul,
            "/" => BinOperator::Div,
            "%" => BinOperator::Mod,
            _ => return Err(format!("Unknown multiplicative op: {}", op_str).into()),
        };
        let right = parse_unary(inner.remove(0))?;
        left = Expr::BinOp { left: Box::new(left), op, right: Box::new(right) };
    }
    Ok(left)
}

fn parse_unary(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner: Vec<_> = pair.into_inner().collect();
    if inner.len() == 2 {
        let expr = parse_unary(inner.remove(1))?;
        Ok(Expr::UnaryOp { op: UnaryOperator::Neg, expr: Box::new(expr) })
    } else {
        parse_accessor(inner.remove(0))
    }
}

fn parse_accessor(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let mut inner: Vec<_> = pair.into_inner().collect();
    let mut expr = parse_atom(inner.remove(0))?;

    for access in inner {
        match access.as_rule() {
            Rule::index_access => {
                let key_pair = access.into_inner().next().unwrap();
                let key = parse_expression(key_pair)?;
                expr = Expr::Index { object: Box::new(expr), key: Box::new(key) };
            }
            Rule::dot_access => {
                let field = access.into_inner().next().unwrap().as_str().to_string();
                expr = Expr::DotAccess { object: Box::new(expr), field };
            }
            _ => (),
        }
    }

    Ok(expr)
}

fn parse_atom(pair: pest::iterators::Pair<Rule>) -> Result<Expr, Box<dyn std::error::Error + Send + Sync>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::expression => parse_expression(inner),
        Rule::func_call => {
            let mut fc_inner = inner.into_inner();
            let name = fc_inner.next().unwrap().as_str().to_string();
            let args: Result<Vec<_>, _> = fc_inner.map(|p| parse_expression(p)).collect();
            Ok(Expr::FuncCall { name, args: args? })
        }
        Rule::string_literal => {
            let s = inner.as_str();
            Ok(Expr::Str(s[1..s.len()-1].to_string()))
        }
        Rule::number => {
            let s = inner.as_str();
            if s.contains('.') {
                Ok(Expr::Float(s.parse()?))
            } else {
                Ok(Expr::Int(s.parse()?))
            }
        }
        Rule::boolean => {
            Ok(Expr::Bool(inner.as_str() == "true"))
        }
        Rule::address => {
            Ok(Expr::Address(inner.as_str().to_string()))
        }
        Rule::identifier => {
            let name = inner.as_str();
            if name == "null" {
                Ok(Expr::Null)
            } else {
                Ok(Expr::Identifier(name.to_string()))
            }
        }
        _ => Err(format!("Unexpected atom: {:?}", inner.as_rule()).into()),
    }
}

// --- Utility parsing ---

fn parse_pair(pair: pest::iterators::Pair<Rule>) -> Result<(String, AetherValue), Box<dyn std::error::Error + Send + Sync>> {
    let mut inner = pair.into_inner();
    let k = inner.next().unwrap().as_str().to_string();
    let v = parse_value(inner.next().unwrap())?;
    Ok((k, v))
}

fn parse_value(pair: pest::iterators::Pair<Rule>) -> Result<AetherValue, Box<dyn std::error::Error + Send + Sync>> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::string_literal => {
            let s = inner.as_str();
            Ok(AetherValue::String(s[1..s.len()-1].to_string()))
        }
        Rule::number => {
            let s = inner.as_str();
            if s.contains('.') {
                Ok(AetherValue::Float(s.parse()?))
            } else {
                Ok(AetherValue::Int(s.parse()?))
            }
        }
        Rule::boolean => {
            Ok(AetherValue::Bool(inner.as_str() == "true"))
        }
        Rule::hash | Rule::address => {
            Ok(AetherValue::String(inner.as_str().to_string()))
        }
        _ => {
            if inner.as_str() == "null" {
                Ok(AetherValue::Null)
            } else {
                Ok(AetherValue::String(inner.as_str().to_string()))
            }
        }
    }
}
