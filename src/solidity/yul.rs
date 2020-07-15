use super::*;

#[derive(Debug, Clone)]
pub struct YulBlock {
    pub statements: Vec<YulStatement>,
}

impl fmt::Display for YulBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let statements: Vec<String> = self
            .statements
            .clone()
            .into_iter()
            .map(|s| format!("{}", s))
            .collect();
        let statements = statements.join("\n");

        write!(f, "{{ \n {statements} \n }}", statements = statements)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum YulExpression {
    FunctionCall(YulFunctionCall),
    Identifier(String),
    Literal(YulLiteral),
    Catchable(Box<YulExpression>, Box<YulExpression>),
    VariableDeclaration(YulVariableDeclaration),
    Assignment(YulAssignment),
    Noop,
    Inline(String),
}

impl fmt::Display for YulExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YulExpression::FunctionCall(fc) => write!(f, "{fc}", fc = fc),
            YulExpression::Identifier(s) => write!(f, "{s}", s = s),
            YulExpression::Literal(l) => write!(f, "{l}", l = l),
            YulExpression::Catchable(v, _) => write!(f, "{v}", v = v),
            YulExpression::VariableDeclaration(v) => write!(f, "{v}", v = v),
            YulExpression::Assignment(a) => write!(f, "{a}", a = a),
            YulExpression::Noop => write!(f, ""),
            YulExpression::Inline(i) => write!(f, "{i}", i = i),
        }
    }
}

#[derive(Debug, Clone)]
pub struct YulFunctionDefinition {
    pub identifier: String,
    pub arguments: Vec<(String, YulType)>,
    pub returns: Vec<(String, YulType)>,
    pub body: YulBlock,
}

impl fmt::Display for YulFunctionDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = self.arguments.clone();
        let args: Vec<String> = args
            .into_iter()
            .map(|(a, b)| format!("{a}: {b}", a = a, b = b))
            .collect();
        let args = args.join(", ");

        let ret = if !self.returns.is_empty() {
            let p = self.arguments.clone();
            let p: Vec<String> = p
                .into_iter()
                .map(|(a, b)| format!("{a}: {b}", a = a, b = b))
                .collect();
            let p = p.join(", ");
            format!("-> {p}", p = p)
        } else {
            "".to_string()
        };

        write!(
            f,
            "{identifier}({arg}) {ret} {body}",
            identifier = self.identifier,
            arg = args,
            ret = ret,
            body = self.body
        )
    }
}

#[derive(Debug, Clone)]
pub struct YulFunctionCall {
    pub name: String,
    pub arguments: Vec<YulExpression>,
}

impl fmt::Display for YulFunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = self.arguments.clone();
        let args: Vec<String> = args.into_iter().map(|a| format!("{}", a)).collect();
        let args = args.join(", ");
        write!(f, "{name}({args})", name = self.name, args = args)
    }
}

#[derive(Debug, Clone)]
pub struct YulAssignment {
    pub identifiers: Vec<String>,
    pub expression: Box<YulExpression>,
}

impl fmt::Display for YulAssignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lhs: Vec<String> = self.identifiers.clone();
        let lhs = lhs.join(", ");
        write!(
            f,
            "{idents} := {expression}",
            idents = lhs,
            expression = self.expression
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum YulStatement {
    Block(YulBlock),
    FunctionDefinition(YulFunctionDefinition),
    If(YulIf),
    Expression(YulExpression),
    Switch(YulSwitch),
    For(YulForLoop),
    Break,
    Continue,
    Noop,
    Inline(String),
}

impl fmt::Display for YulStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YulStatement::Block(b) => write!(f, "{b}", b = b),
            YulStatement::FunctionDefinition(e) => write!(f, "{e}", e = e),
            YulStatement::If(e) => write!(f, "{e}", e = e),
            YulStatement::Expression(e) => write!(f, "{e}", e = e),
            YulStatement::Switch(s) => write!(f, "{s}", s = s),
            YulStatement::For(e) => write!(f, "{e}", e = e),
            YulStatement::Break => write!(f, "break"),
            YulStatement::Continue => write!(f, "continue"),
            YulStatement::Noop => write!(f, ""),
            YulStatement::Inline(i) => write!(f, "{i}", i = i),
        }
    }
}

#[derive(Debug, Clone)]
pub struct YulIf {
    pub expression: YulExpression,
    pub block: YulBlock,
}

impl fmt::Display for YulIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "if {expression} {block}",
            expression = self.expression,
            block = self.block,
        )
    }
}

#[derive(Debug, Clone)]
pub struct YulSwitch {
    pub expression: YulExpression,
    pub cases: Vec<(YulLiteral, YulBlock)>,
    pub default: Option<YulBlock>,
}

impl fmt::Display for YulSwitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cases: Vec<String> = self
            .cases
            .clone()
            .into_iter()
            .map(|(c, b)| format!("case {c} {b}", c = c, b = b))
            .collect();
        let cases = cases.join("\n");

        let default = if let Some(ref default) = self.default {
            format!("\n default {d}", d = default)
        } else {
            format!("")
        };

        write!(
            f,
            "switch {expression} \n {cases}{default}",
            expression = self.expression,
            cases = cases,
            default = default
        )
    }
}

#[derive(Debug, Clone)]
pub struct YulForLoop {
    pub initialise: YulBlock,
    pub condition: YulExpression,
    pub step: YulBlock,
    pub body: YulBlock,
}

impl fmt::Display for YulForLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "for {initialise} {condition} {step} {body}",
            initialise = self.initialise,
            condition = self.condition,
            step = self.step,
            body = self.body,
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum YulLiteral {
    Num(u64),
    String(String),
    Bool(bool),
    Decimal(u64, u64),
    Hex(String),
}

impl fmt::Display for YulLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YulLiteral::Num(n) => write!(f, "{n}", n = n),
            YulLiteral::String(s) => write!(f, "\"{n}\"", n = s),
            YulLiteral::Bool(b) => {
                let value = if *b { 1 } else { 0 };
                write!(f, "{n}", n = value)
            }
            YulLiteral::Decimal(_, _) => panic!("Float currently not supported"),
            YulLiteral::Hex(n) => write!(f, "{n}", n = n),
        }
    }
}

#[derive(Debug, Clone)]
pub struct YulVariableDeclaration {
    pub declaration: String,
    pub declaration_type: YulType,
    pub expression: Option<Box<YulExpression>>,
}

impl fmt::Display for YulVariableDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let declarations = if let YulType::Any = self.declaration_type {
            self.declaration.to_string()
        } else {
            format!(
                "{ident}: {var_type}",
                ident = self.declaration,
                var_type = self.declaration_type
            )
        };
        if self.expression.is_none() {
            write!(f, "let {declarations}", declarations = declarations)?;
        }
        let expression = self.expression.clone();
        let expression = expression.unwrap();
        write!(
            f,
            "let {declarations} := {expression}",
            declarations = declarations,
            expression = *expression
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum YulType {
    Bool,
    U8,
    S8,
    U32,
    S32,
    U64,
    S64,
    U128,
    S128,
    U256,
    S256,
    Any,
}

impl fmt::Display for YulType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YulType::Bool => write!(f, "bool"),
            YulType::U8 => write!(f, "u8"),
            YulType::S8 => write!(f, "s8"),
            YulType::U32 => write!(f, "u32"),
            YulType::S32 => write!(f, "s32"),
            YulType::U64 => write!(f, "u64"),
            YulType::S64 => write!(f, "s64"),
            YulType::U128 => write!(f, "u128"),
            YulType::S128 => write!(f, "s128"),
            YulType::U256 => write!(f, "u256"),
            YulType::S256 => write!(f, "s256"),
            YulType::Any => write!(f, "any"),
        }
    }
}
