use crate::ast::{Expression, Identifier};
use core::fmt;

#[derive(Debug, Clone)]
pub struct MoveIRBlock {
    pub statements: Vec<MoveIRStatement>,
}

impl fmt::Display for MoveIRBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let statements: Vec<String> = self
            .statements
            .clone()
            .into_iter()
            .map(|s| format!("{s}", s = s))
            .collect();
        let statements = statements.join("\n");
        write!(f, "{{ \n {statements} \n }}", statements = statements)
    }
}

#[derive(Debug, Clone)]
pub struct MoveIRFunctionDefinition {
    pub identifier: MoveIRIdentifier,
    pub arguments: Vec<MoveIRIdentifier>,
    pub returns: Vec<MoveIRIdentifier>,
    pub body: MoveIRBlock,
}

impl fmt::Display for MoveIRFunctionDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arguments: Vec<String> = self
            .arguments
            .clone()
            .into_iter()
            .map(|a| format!("{}", a))
            .collect();
        let arguments = arguments.join(", ");
        let returns: Vec<String> = self
            .returns
            .clone()
            .into_iter()
            .map(|a| format!("{}", a))
            .collect();
        let returns = returns.join(", ");
        write!(
            f,
            "{identifier}({arguments}) {returns}{body}",
            identifier = self.identifier,
            arguments = arguments,
            returns = returns,
            body = self.body
        )
    }
}

#[derive(Debug, Clone)]
pub struct MoveIRIdentifier {
    pub identifier: String,
    pub move_type: MoveIRType,
}

impl fmt::Display for MoveIRIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{identifier}: {move_type}",
            identifier = self.identifier,
            move_type = self.move_type
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum MoveIRExpression {
    FunctionCall(MoveIRFunctionCall),
    StructConstructor(MoveIRStructConstructor),
    Identifier(String),
    Transfer(MoveIRTransfer),
    Literal(MoveIRLiteral),
    Catchable,
    Inline(String),
    Assignment(MoveIRAssignment),
    VariableDeclaration(MoveIRVariableDeclaration),
    FieldDeclaration(MoveIRFieldDeclaration),
    Operation(MoveIROperation),
    Vector(MoveIRVector),
    Noop,
}

impl fmt::Display for MoveIRExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveIRExpression::FunctionCall(fc) => write!(f, "{fc}", fc = fc),
            MoveIRExpression::StructConstructor(s) => {
                let args = s.fields.clone();
                let args: Vec<String> = args
                    .into_iter()
                    .map(|(k, v)| format!("{k}: {v}", k = k, v = v))
                    .collect();
                let args = args.join(",\n");
                write!(
                    f,
                    "{name} {{ \n {args} }}",
                    name = s.identifier.token,
                    args = args
                )
            }
            MoveIRExpression::Identifier(s) => write!(f, "{s}", s = s),
            MoveIRExpression::Transfer(t) => write!(f, "{t}", t = t),
            MoveIRExpression::Literal(l) => write!(f, "{l}", l = l),
            MoveIRExpression::Catchable => unimplemented!(),
            MoveIRExpression::Inline(s) => write!(f, "{s}", s = s),
            MoveIRExpression::Assignment(a) => write!(f, "{a}", a = a),
            MoveIRExpression::VariableDeclaration(v) => write!(f, "{v}", v = v),
            MoveIRExpression::Noop => write!(f, ""),
            MoveIRExpression::FieldDeclaration(fd) => write!(f, "{fd}", fd = fd),
            MoveIRExpression::Operation(o) => write!(f, "{o}", o = o),
            MoveIRExpression::Vector(v) => write!(f, "{v}", v = v),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveIRVector {
    pub elements: Vec<MoveIRExpression>,
    pub vec_type: Option<MoveIRType>,
}

impl fmt::Display for MoveIRVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref move_type) = self.vec_type {
            write!(f, "Vector.empty<{move_type}>()", move_type = *move_type,)
        } else {
            write!(f, "Vector.empty<>()")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveIRStructConstructor {
    pub identifier: Identifier,
    pub fields: Vec<(String, MoveIRExpression)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveIRFunctionCall {
    pub identifier: String,
    pub arguments: Vec<MoveIRExpression>,
}

impl fmt::Display for MoveIRFunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arguments: Vec<String> = self
            .arguments
            .clone()
            .into_iter()
            .map(|a| format!("{}", a))
            .collect();
        let arguments = arguments.join(", ");
        write!(
            f,
            "{i}({arguments})",
            i = self.identifier,
            arguments = arguments
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum MoveIRLiteral {
    U8(u8),
    U64(u64),
    String(String),
    Bool(bool),
    Decimal(u64, u64),
    Hex(String),
}

impl fmt::Display for MoveIRLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveIRLiteral::U8(num) => write!(f, "{}u8", num),
            MoveIRLiteral::U64(i) => write!(f, "{i}", i = i),
            MoveIRLiteral::String(s) => write!(f, "\"{s}\"", s = s),
            MoveIRLiteral::Bool(b) => write!(f, "{b}", b = b),
            MoveIRLiteral::Decimal(i1, i2) => write!(f, "{i1}.{i2}", i1 = i1, i2 = i2),
            MoveIRLiteral::Hex(h) => write!(f, "{h}", h = h),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveIRTransfer {
    Move(Box<MoveIRExpression>),
    Copy(Box<MoveIRExpression>),
}

impl fmt::Display for MoveIRTransfer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveIRTransfer::Move(e) => write!(f, "move({expression})", expression = e),
            MoveIRTransfer::Copy(e) => write!(f, "copy({expression})", expression = e),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveIRAssignment {
    pub identifier: String,
    pub expression: Box<MoveIRExpression>,
}

impl fmt::Display for MoveIRAssignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{identifier} = {expression}",
            identifier = self.identifier,
            expression = self.expression
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveIRFieldDeclaration {
    pub identifier: String,
    pub declaration_type: MoveIRType,
    pub expression: Option<Expression>,
}

impl fmt::Display for MoveIRFieldDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{ident}: {ident_type}",
            ident = self.identifier,
            ident_type = self.declaration_type
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveIRVariableDeclaration {
    pub identifier: String,
    pub declaration_type: MoveIRType,
}

impl fmt::Display for MoveIRVariableDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "let {ident}: {ident_type}",
            ident = self.identifier,
            ident_type = self.declaration_type
        )
    }
}

#[derive(Debug, Clone)]
pub struct MoveIRIf {
    pub expression: MoveIRExpression,
    pub block: MoveIRBlock,
    pub else_block: Option<MoveIRBlock>,
}

impl fmt::Display for MoveIRIf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let else_block = if let Some(ref block) = self.else_block {
            block.to_string()
        } else {
            "{}".to_string()
        };
        write!(
            f,
            "if ({expression}) {block} else {else_block} ",
            expression = self.expression,
            block = self.block,
            else_block = else_block
        )
    }
}

#[derive(Debug, Clone)]
pub struct MoveIRModuleImport {
    pub name: String,
    pub address: String,
}

impl fmt::Display for MoveIRModuleImport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "import {address}.{name}",
            address = self.address,
            name = self.name
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum MoveIRStatement {
    Block(MoveIRBlock),
    FunctionDefinition(MoveIRFunctionDefinition),
    If(MoveIRIf),
    Expression(MoveIRExpression),
    Switch,
    For,
    Break,
    Continue,
    Noop,
    Inline(String),
    Return(MoveIRExpression),
    Import(MoveIRModuleImport),
    Assert(MoveIRExpression, u32),
}

impl fmt::Display for MoveIRStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveIRStatement::Block(b) => write!(f, "{b}", b = b),
            MoveIRStatement::FunctionDefinition(fd) => write!(f, "{fd}", fd = fd),
            MoveIRStatement::If(i) => write!(f, "{i}", i = i),
            MoveIRStatement::Expression(e) => write!(f, "{e};", e = e),
            MoveIRStatement::Switch => write!(f, ""),
            MoveIRStatement::For => write!(f, ""),
            MoveIRStatement::Break => write!(f, "break"),
            MoveIRStatement::Continue => write!(f, "continue"),
            MoveIRStatement::Noop => write!(f, ""),
            MoveIRStatement::Inline(s) => write!(f, "{s};", s = s),
            MoveIRStatement::Return(e) => write!(f, "return {e};", e = e),
            MoveIRStatement::Import(m) => write!(f, "{s};", s = m),
            MoveIRStatement::Assert(expr, line) => write!(f, "assert({}, {});", expr, line),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveIRType {
    U8,
    U64,
    Address,
    Bool,
    ByteArray,
    Signer,
    Resource(String),
    StructType(String),
    Reference(Box<MoveIRType>),
    MutableReference(Box<MoveIRType>),
    Vector(Box<MoveIRType>),
}

impl fmt::Display for MoveIRType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveIRType::U8 => write!(f, "u8"),
            MoveIRType::U64 => write!(f, "u64"),
            MoveIRType::Address => write!(f, "address"),
            MoveIRType::Bool => write!(f, "bool"),
            MoveIRType::Signer => write!(f, "signer"),
            MoveIRType::ByteArray => write!(f, "vector<u8>"),
            MoveIRType::Resource(s) => write!(f, "{}", s),
            MoveIRType::StructType(s) => write!(f, "{}", s),
            MoveIRType::Reference(base) => write!(f, "&{base}", base = base),
            MoveIRType::MutableReference(base) => write!(f, "&mut {base}", base = base),
            MoveIRType::Vector(base) => write!(f, "vector<{base}>", base = base),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum MoveIROperation {
    Add(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Minus(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Times(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Divide(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Modulo(Box<MoveIRExpression>, Box<MoveIRExpression>),
    GreaterThan(Box<MoveIRExpression>, Box<MoveIRExpression>),
    GreaterThanEqual(Box<MoveIRExpression>, Box<MoveIRExpression>),
    LessThan(Box<MoveIRExpression>, Box<MoveIRExpression>),
    LessThanEqual(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Equal(Box<MoveIRExpression>, Box<MoveIRExpression>),
    NotEqual(Box<MoveIRExpression>, Box<MoveIRExpression>),
    And(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Or(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Not(Box<MoveIRExpression>),
    Power(Box<MoveIRExpression>, Box<MoveIRExpression>),
    Access(Box<MoveIRExpression>, String),
    Dereference(Box<MoveIRExpression>),
    MutableReference(Box<MoveIRExpression>),
    Reference(Box<MoveIRExpression>),
}

impl fmt::Display for MoveIROperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveIROperation::Add(l, r) => write!(f, "({l} + {r})", l = l, r = r),
            MoveIROperation::Minus(l, r) => write!(f, "({l} - {r})", l = l, r = r),
            MoveIROperation::Times(l, r) => write!(f, "({l} * {r})", l = l, r = r),
            MoveIROperation::GreaterThan(l, r) => write!(f, "({l} > {r})", l = l, r = r),
            MoveIROperation::LessThan(l, r) => write!(f, "({l} < {r})", l = l, r = r),
            MoveIROperation::Divide(l, r) => write!(f, "({l} / {r})", l = l, r = r),
            MoveIROperation::Modulo(l, r) => write!(f, "({l} % {r})", l = l, r = r),
            MoveIROperation::GreaterThanEqual(l, r) => write!(f, "({l} >= {r})", l = l, r = r),
            MoveIROperation::LessThanEqual(l, r) => write!(f, "({l} <= {r})", l = l, r = r),
            MoveIROperation::Equal(l, r) => write!(f, "({l} == {r})", l = l, r = r),
            MoveIROperation::NotEqual(l, r) => write!(f, "({l} != {r})", l = l, r = r),
            MoveIROperation::And(l, r) => write!(f, "({l} && {r})", l = l, r = r),
            MoveIROperation::Or(l, r) => write!(f, "({l} || {r})", l = l, r = r),
            MoveIROperation::Not(e) => write!(f, "!{expression}", expression = e),
            MoveIROperation::Power(l, r) => write!(f, "({l} ** {r})", l = l, r = r),
            MoveIROperation::Access(l, r) => write!(f, "{l}.{r}", l = l, r = r),
            MoveIROperation::Dereference(r) => write!(f, "*{r}", r = r),
            MoveIROperation::MutableReference(r) => write!(f, "&mut {r}", r = r),
            MoveIROperation::Reference(r) => write!(f, "&{r}", r = r),
        }
    }
}
