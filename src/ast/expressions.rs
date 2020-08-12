use crate::ast::*;
use crate::context::Context;
use crate::type_checker::ExpressionChecker;
use crate::visitor::*;
use itertools::Itertools;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Identifier(Identifier),
    BinaryExpression(BinaryExpression),
    InoutExpression(InoutExpression),
    ExternalCall(ExternalCall),
    FunctionCall(FunctionCall),
    VariableDeclaration(VariableDeclaration),
    BracketedExpression(BracketedExpression),
    AttemptExpression(AttemptExpression),
    Literal(Literal),
    ArrayLiteral(ArrayLiteral),
    DictionaryLiteral(DictionaryLiteral),
    SelfExpression,
    SubscriptExpression(SubscriptExpression),
    RangeExpression(RangeExpression),
    RawAssembly(String, Option<Type>),
    CastExpression(CastExpression),
    Sequence(Vec<Expression>),
}

impl Expression {
    pub fn assign_enclosing_type(&mut self, type_id: &str) {
        match self {
            Expression::Identifier(i) => {
                i.enclosing_type = Some(String::from(type_id));
            }
            Expression::BinaryExpression(b) => {
                b.lhs_expression.assign_enclosing_type(type_id);
            }
            Expression::ExternalCall(e) => {
                e.function_call
                    .lhs_expression
                    .assign_enclosing_type(type_id);
            }
            Expression::FunctionCall(f) => {
                f.identifier.enclosing_type = Some(String::from(type_id));
            }
            Expression::BracketedExpression(b) => {
                b.expression.assign_enclosing_type(type_id);
            }
            Expression::SubscriptExpression(s) => {
                s.base_expression.enclosing_type = Some(String::from(type_id));
            }
            _ => {}
        }
    }

    pub fn enclosing_type(&self) -> Option<String> {
        match self.clone() {
            Expression::Identifier(i) => i.enclosing_type,
            Expression::InoutExpression(i) => i.expression.enclosing_type(),
            Expression::BinaryExpression(b) => b.lhs_expression.enclosing_type(),
            Expression::VariableDeclaration(v) => Option::from(v.identifier.token),
            Expression::BracketedExpression(b) => b.expression.enclosing_type(),
            Expression::FunctionCall(f) => f.identifier.enclosing_type,
            Expression::ExternalCall(e) => e.function_call.lhs_expression.enclosing_type(),
            Expression::SubscriptExpression(_) => unimplemented!(),
            _ => None,
        }
    }

    pub fn enclosing_identifier(&self) -> Option<&Identifier> {
        match self {
            Expression::Identifier(i) => Some(i),
            Expression::BinaryExpression(b) => b.rhs_expression.enclosing_identifier(),
            Expression::InoutExpression(i) => i.expression.enclosing_identifier(),
            Expression::ExternalCall(e) => e.function_call.lhs_expression.enclosing_identifier(),
            Expression::FunctionCall(f) => Some(&f.identifier),
            Expression::VariableDeclaration(v) => Some(&v.identifier),
            Expression::BracketedExpression(b) => b.expression.enclosing_identifier(),
            Expression::SubscriptExpression(s) => Some(&s.base_expression),
            _ => None,
        }
    }

    pub fn get_line_info(&self) -> LineInfo {
        match self {
            Expression::Identifier(i) => i.line_info.clone(),
            Expression::BinaryExpression(b) => b.line_info.clone(),
            Expression::InoutExpression(i) => i.expression.get_line_info(),
            Expression::ExternalCall(_) => unimplemented!(),
            Expression::FunctionCall(_) => unimplemented!(),
            Expression::VariableDeclaration(_) => unimplemented!(),
            Expression::BracketedExpression(_) => unimplemented!(),
            Expression::AttemptExpression(_) => unimplemented!(),
            Expression::Literal(_) => unimplemented!(),
            Expression::ArrayLiteral(_) => unimplemented!(),
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => unimplemented!(),
            Expression::SubscriptExpression(_) => unimplemented!(),
            Expression::RangeExpression(_) => unimplemented!(),
            Expression::RawAssembly(_, _) => unimplemented!(),
            Expression::CastExpression(_) => unimplemented!(),
            Expression::Sequence(_) => unimplemented!(),
        }
    }
}

impl Visitable for Expression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_expression(self, ctx)?;

        match self {
            Expression::Identifier(i) => i.visit(v, ctx),
            Expression::BinaryExpression(b) => b.visit(v, ctx),
            Expression::InoutExpression(i) => i.visit(v, ctx),
            Expression::ExternalCall(e) => e.visit(v, ctx),
            Expression::FunctionCall(f) => f.visit(v, ctx),
            Expression::VariableDeclaration(d) => d.visit(v, ctx),
            Expression::BracketedExpression(b) => b.visit(v, ctx),
            Expression::AttemptExpression(a) => a.visit(v, ctx),
            Expression::Literal(l) => l.visit(v, ctx),
            Expression::ArrayLiteral(a) => a.visit(v, ctx),
            Expression::DictionaryLiteral(d) => d.visit(v, ctx),
            Expression::SelfExpression => return Ok(()),
            Expression::SubscriptExpression(s) => s.visit(v, ctx),
            Expression::RangeExpression(r) => r.visit(v, ctx),
            Expression::RawAssembly(_, _) => return Ok(()),
            Expression::CastExpression(c) => c.visit(v, ctx),
            Expression::Sequence(l) => {
                for i in l {
                    i.visit(v, ctx)?;
                }
                Ok(())
            }
        }?;
        v.finish_expression(self, ctx)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CastExpression {
    pub expression: Box<Expression>,
    pub cast_type: Type,
}

impl Visitable for CastExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_cast_expression(self, ctx)?;

        self.cast_type.visit(v, ctx)?;

        self.expression.visit(v, ctx)?;

        v.finish_cast_expression(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeExpression {
    pub start_expression: Box<Expression>,
    pub end_expression: Box<Expression>,
    pub op: std::string::String,
}

impl Visitable for RangeExpression {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubscriptExpression {
    pub base_expression: Identifier,
    pub index_expression: Box<Expression>,
}

impl Visitable for SubscriptExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_subscript_expression(self, ctx)?;

        let in_subscript = ctx.in_subscript;

        self.base_expression.visit(v, ctx)?;

        ctx.in_subscript = true;

        self.index_expression.visit(v, ctx)?;

        ctx.in_subscript = in_subscript;

        v.finish_subscript_expression(self, ctx)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttemptExpression {
    pub kind: String,
    pub function_call: FunctionCall,
    pub predicate: Option<Box<Expression>>,
}

impl AttemptExpression {
    pub fn is_soft(&self) -> bool {
        self.kind.eq("?")
    }
}

impl Visitable for AttemptExpression {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        let mut caller_protections: Vec<CallerProtection> = vec![];
        let caller_id: String;

        if let Some(contract_context) = &_ctx.contract_behaviour_declaration_context.as_ref() {
            let contract_name = contract_context.identifier.token.clone();

            if let Some(types) = _ctx.environment.types.get(&contract_name) {
                if let Some(function_call) =
                    types.functions.get(&self.function_call.identifier.token)
                {
                    caller_protections = function_call.get(0).unwrap().caller_protections.clone();
                }
            }

            if let Some(caller) = &contract_context.caller {
                caller_id = caller.token.clone();
            } else {
                caller_id = "caller".to_string();
            }
        } else {
            caller_id = "caller".to_string();
        }

        let caller_id: Identifier = Identifier {
            token: caller_id,
            enclosing_type: None,
            line_info: Default::default(),
        };

        // TODO: remove repeating code from generate_predicate

        let predicate = caller_protections
            .iter()
            .cloned()
            .filter_map(|c| {
                let mut ident = c.clone().identifier;
                let en_ident;
                let c_type;

                if let Some(contract_context) =
                    &_ctx.contract_behaviour_declaration_context.as_ref()
                {
                    ident.enclosing_type = Option::from(contract_context.identifier.token.clone());
                    en_ident = contract_context.identifier.clone();

                    c_type = _ctx.environment.get_expression_type(
                        &Expression::Identifier(ident.clone()),
                        &en_ident.token,
                        &[],
                        &[],
                        &ScopeContext {
                            parameters: vec![],
                            local_variables: vec![],
                            counter: 0,
                        },
                    );
                } else {
                    en_ident = Identifier {
                        token: "".to_string(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    };

                    c_type = _ctx.environment.get_expression_type(
                        &Expression::Identifier(ident.clone()),
                        "",
                        &[],
                        &[],
                        &ScopeContext {
                            parameters: vec![],
                            local_variables: vec![],
                            counter: 0,
                        },
                    );
                }

                match c_type {
                    Type::Address => Some(Expression::BinaryExpression(BinaryExpression {
                        lhs_expression: Box::new(Expression::Identifier(ident)),
                        rhs_expression: Box::new(Expression::RawAssembly(
                            format!("Signer.address_of(copy({}))", caller_id.token),
                            None,
                        )),
                        op: BinOp::DoubleEqual,
                        line_info: Default::default(),
                    })),

                    Type::ArrayType(array_type) => {
                        assert!(
                            *array_type.key_type == Type::Address,
                            "Array values for caller protection must have type Address"
                        );
                        if let Some(property) = _ctx.environment.get_caller_protection(&c) {
                            if let Some(Expression::ArrayLiteral(array)) =
                                property.property.get_value()
                            {
                                let predicate = array
                                    .elements
                                    .iter()
                                    .cloned()
                                    .map(|c| {
                                        Expression::BinaryExpression(BinaryExpression {
                                            lhs_expression: Box::new(c),
                                            rhs_expression: Box::new(Expression::RawAssembly(
                                                format!(
                                                    "Signer.address_of(copy({}))",
                                                    caller_id.token
                                                ),
                                                None,
                                            )),
                                            op: BinOp::DoubleEqual,
                                            line_info: Default::default(),
                                        })
                                    })
                                    .fold1(|left, right| {
                                        Expression::BinaryExpression(BinaryExpression {
                                            lhs_expression: Box::new(left),
                                            rhs_expression: Box::new(right),
                                            op: BinOp::Or,
                                            line_info: Default::default(),
                                        })
                                    })
                                    .unwrap();
                                Some(predicate)
                            } else {
                                panic!("Mismatching types for {:?}", c)
                            }
                        } else {
                            panic!("{:?} not found in caller protections", c)
                        }
                    }
                    Type::DictionaryType(dict_type) => {
                        assert!(
                            *dict_type.value_type == Type::Address,
                            "Dictionary values for caller protection must have type Address"
                        );
                        if let Some(property) = _ctx.environment.get_caller_protection(&c) {
                            if let Some(Expression::DictionaryLiteral(dict)) =
                                property.property.get_value()
                            {
                                let predicate = dict
                                    .elements
                                    .iter()
                                    .cloned()
                                    .map(|(_, v)| {
                                        Expression::BinaryExpression(BinaryExpression {
                                            lhs_expression: Box::new(v),
                                            rhs_expression: Box::new(Expression::RawAssembly(
                                                format!(
                                                    "Signer.address_of(copy({}))",
                                                    caller_id.token
                                                ),
                                                None,
                                            )),
                                            op: BinOp::DoubleEqual,
                                            line_info: Default::default(),
                                        })
                                    })
                                    .fold1(|left, right| {
                                        Expression::BinaryExpression(BinaryExpression {
                                            lhs_expression: Box::new(left),
                                            rhs_expression: Box::new(right),
                                            op: BinOp::Or,
                                            line_info: Default::default(),
                                        })
                                    })
                                    .unwrap();
                                Some(predicate)
                            } else {
                                panic!("Mismatching types for {:?}", c)
                            }
                        } else {
                            panic!("{:?} not found in caller protections", c)
                        }
                    }
                    _ => {
                        let enclosing_type =
                            ident.enclosing_type.as_deref().unwrap_or(&en_ident.token);
                        if let Some(types) = _ctx.environment.types.get(enclosing_type) {
                            if let Some(function_info) = types.functions.get(&ident.token) {
                                if let Some(function) = function_info.get(0) {
                                    let function_signature = &function.declaration.head;
                                    if function_signature.is_predicate() {
                                        // caller protection is a predicate function
                                        return Some(Expression::FunctionCall(FunctionCall {
                                            identifier: ident,
                                            arguments: vec![
                                                FunctionArgument {
                                                    identifier: None,
                                                    expression: Expression::Identifier(
                                                        Identifier {
                                                            token: "address_this".to_string(),
                                                            enclosing_type: None,
                                                            line_info: Default::default(),
                                                        },
                                                    ),
                                                },
                                                FunctionArgument {
                                                    identifier: None,
                                                    expression: Expression::RawAssembly(
                                                        format!(
                                                            "Signer.address_of(copy({}))",
                                                            caller_id.token
                                                        ),
                                                        None,
                                                    ),
                                                },
                                            ],
                                            mangled_identifier: None,
                                        }));
                                    } else if function_signature.is_0_ary_function() {
                                        // caller protection is a 0-ary function
                                        return Some(Expression::BinaryExpression(
                                            BinaryExpression {
                                                lhs_expression: Box::new(Expression::FunctionCall(
                                                    FunctionCall {
                                                        identifier: ident,
                                                        arguments: vec![FunctionArgument {
                                                            identifier: None,
                                                            expression: Expression::Identifier(
                                                                Identifier {
                                                                    token: "address_this"
                                                                        .to_string(),
                                                                    enclosing_type: None,
                                                                    line_info: Default::default(),
                                                                },
                                                            ),
                                                        }],
                                                        mangled_identifier: None,
                                                    },
                                                )),
                                                rhs_expression: Box::new(Expression::RawAssembly(
                                                    format!(
                                                        "Signer.address_of(copy({}))",
                                                        caller_id.token
                                                    ),
                                                    None,
                                                )),
                                                op: BinOp::DoubleEqual,
                                                line_info: Default::default(),
                                            },
                                        ));
                                    }
                                }
                            }
                        }
                        panic!(
                            "Invalid caller protection \"{}\" at line {}",
                            ident.token, ident.line_info.line
                        )
                    }
                }
            })
            .fold1(|left, right| {
                Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(left),
                    rhs_expression: Box::new(right),
                    op: BinOp::Or,
                    line_info: Default::default(),
                })
            });

        if let Some(predicate) = predicate {
            self.predicate = Some(Box::new(predicate));
        }

        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
pub struct Identifier {
    pub token: std::string::String,
    pub enclosing_type: Option<std::string::String>,
    pub line_info: LineInfo,
}

impl Identifier {
    pub const SELF: &'static str = "self";

    pub fn is_self(&self) -> bool {
        self.token == Identifier::SELF
    }

    pub fn generated<T: ToString + ?Sized>(name: &T) -> Identifier {
        Identifier {
            token: name.to_string(),
            enclosing_type: None,
            line_info: Default::default(),
        }
    }
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.enclosing_type == other.enclosing_type
    }
}

impl Visitable for Identifier {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_identifier(self, ctx)?;
        v.finish_identifier(self, ctx)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryExpression {
    pub lhs_expression: Box<Expression>,
    pub rhs_expression: Box<Expression>,
    pub op: BinOp,
    pub line_info: LineInfo,
}

impl Visitable for BinaryExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_binary_expression(self, ctx)?;

        let old_is_lvalue = ctx.is_lvalue;
        if self.op.is_assignment() {
            if let Expression::VariableDeclaration(_) = *self.lhs_expression {
            } else {
                ctx.is_lvalue = true;
            }
        }

        if let BinOp::Dot = self.op {
            ctx.is_enclosing = true;
        }

        let old_context = ctx.external_call_context.clone();
        ctx.external_call_context = None;

        self.lhs_expression.visit(v, ctx)?;

        if let BinOp::Dot = self.op.clone() {
            // ctx.is_lvalue = false;
        }

        ctx.external_call_context = old_context;
        ctx.is_enclosing = false;

        let scope = ctx.scope_context.as_ref().unwrap_or_default();

        let enclosing = ctx
            .enclosing_type_identifier()
            .map(|id| &*id.token)
            .unwrap_or_default();
        let lhs_type =
            ctx.environment
                .get_expression_type(&*self.lhs_expression, enclosing, &[], &[], scope);

        match lhs_type {
            Type::DictionaryType(_) | Type::ArrayType(_) | Type::FixedSizedArrayType(_) => {}
            _ => {
                if self.op.is_assignment() {
                    ctx.in_assignment = true;
                }
                self.rhs_expression.visit(v, ctx)?;
                ctx.in_assignment = false;
            }
        };

        v.finish_binary_expression(self, ctx)?;
        ctx.is_lvalue = old_is_lvalue; // TODO ensure is_lvalue is correct
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InoutExpression {
    pub ampersand_token: std::string::String,
    pub expression: Box<Expression>,
}

impl Visitable for InoutExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_inout_expression(self, ctx)?;
        self.expression.visit(v, ctx)?;
        v.finish_inout_expression(self, ctx)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BracketedExpression {
    pub expression: Box<Expression>,
}

impl Visitable for BracketedExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        self.expression.visit(v, ctx)
    }
}
