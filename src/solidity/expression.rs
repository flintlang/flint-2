use super::*;

pub struct SolidityExpression {
    pub expression: Expression,
    pub is_lvalue: bool,
}

impl SolidityExpression {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        match self.expression.clone() {
            Expression::Identifier(i) => SolidityIdentifier {
                identifier: i,
                is_lvalue: self.is_lvalue,
            }
            .generate(function_context),
            Expression::BinaryExpression(b) => SolidityBinaryExpression {
                expression: b,
                is_lvalue: self.is_lvalue,
            }
            .generate(function_context),
            Expression::InoutExpression(i) => SolidityExpression {
                expression: *i.expression.clone(),
                is_lvalue: true,
            }
            .generate(function_context),
            Expression::ExternalCall(e) => {
                SolidityExternalCall { call: e }.generate(function_context)
            }
            Expression::FunctionCall(f) => {
                SolidityFunctionCall { function_call: f }.generate(function_context)
            }
            Expression::VariableDeclaration(v) => {
                SolidityVariableDeclaration { declaration: v }.generate(function_context)
            }
            Expression::BracketedExpression(e) => SolidityExpression {
                expression: *e.expression,
                is_lvalue: false,
            }
            .generate(function_context),
            Expression::AttemptExpression(_) => {
                panic!("Attempt Expression Not Currently Supported")
            }
            Expression::Literal(l) => {
                YulExpression::Literal(SolidityLiteral { literal: l }.generate())
            }
            Expression::ArrayLiteral(a) => {
                for e in a.elements {
                    if let Expression::ArrayLiteral(_) = e {
                    } else {
                        panic!("Does not support Non-empty array literals")
                    }
                }
                YulExpression::Literal(YulLiteral::Num(0))
            }
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => SoliditySelfExpression {
                is_lvalue: self.is_lvalue,
            }
            .generate(function_context),
            Expression::SubscriptExpression(s) => SoliditySubscriptExpression {
                expression: s,
                is_lvalue: self.is_lvalue,
            }
            .generate(function_context),
            Expression::RangeExpression(_) => unimplemented!(),
            Expression::RawAssembly(a, _) => YulExpression::Inline(a),
            Expression::CastExpression(c) => {
                SolidityCastExpression { expression: c }.generate(function_context)
            }
            Expression::Sequence(s) => {
                let mut sequence = vec![];
                for expression in s {
                    let result = SolidityExpression {
                        expression,
                        is_lvalue: self.is_lvalue,
                    }
                    .generate(function_context);
                    sequence.push(result);
                }

                let sequence: Vec<String> =
                    sequence.into_iter().map(|s| format!("{}", s)).collect();
                let sequence = sequence.join("\n");

                YulExpression::Inline(sequence)
            }
        }
    }
}

struct SolidityCastExpression {
    pub expression: CastExpression,
}

impl SolidityCastExpression {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        let exp = *self.expression.expression.clone();
        let enclosing = exp.enclosing_type();
        let enclosing = if let Some(ref enclosing) = enclosing {
            enclosing
        } else {
            &function_context.enclosing_type
        };

        let original_type = function_context.environment.get_expression_type(
            &*self.expression.expression,
            enclosing,
            &[],
            &[],
            &function_context.scope_context,
        );
        let target_type = self.expression.cast_type.clone();

        let original_type_info = SolidityCastExpression::get_type_info(original_type);
        let target_type_info = SolidityCastExpression::get_type_info(target_type);

        let expression_ir = SolidityExpression {
            expression: *self.expression.expression.clone(),
            is_lvalue: false,
        }
        .generate(function_context);

        if original_type_info.0 <= target_type_info.0 {
            return expression_ir;
        }

        let target_max = SolidityCastExpression::maximum_value(target_type_info.0);
        SolidityRuntimeFunction::revert_if_greater(
            expression_ir,
            YulExpression::Literal(YulLiteral::Hex(target_max)),
        )
    }

    pub fn maximum_value(input: u64) -> String {
        assert!(input % 4 == 0 && input >= 8 && input <= 256);
        format!(
            "0x{}",
            std::iter::repeat("F")
                .take(input as usize / 4)
                .collect::<String>()
        )
    }

    pub fn get_type_info(input: Type) -> (u64, bool) {
        match input {
            Type::Bool => (256, false),
            Type::Int => (256, true),
            Type::String => (256, false),
            Type::Address => (256, false),
            Type::Solidity(s) => match s.clone() {
                SolidityType::ADDRESS => (256, false),
                SolidityType::STRING => (256, false),
                SolidityType::BOOL => (256, false),
                SolidityType::INT8 => (8, true),
                SolidityType::INT16 => (16, true),
                SolidityType::INT24 => (24, true),
                SolidityType::INT32 => (32, true),
                SolidityType::INT40 => (40, true),
                SolidityType::INT48 => (48, true),
                SolidityType::INT56 => (56, true),
                SolidityType::INT64 => (64, true),
                SolidityType::INT72 => (72, true),
                SolidityType::INT80 => (80, true),
                SolidityType::INT88 => (88, true),
                SolidityType::INT96 => (96, true),
                SolidityType::INT104 => (104, true),
                SolidityType::INT112 => (112, true),
                SolidityType::INT120 => (120, true),
                SolidityType::INT128 => (128, true),
                SolidityType::INT136 => (136, true),
                SolidityType::INT144 => (152, true),
                SolidityType::INT152 => (152, true),
                SolidityType::INT160 => (160, true),
                SolidityType::INT168 => (168, true),
                SolidityType::INT176 => (176, true),
                SolidityType::INT184 => (184, true),
                SolidityType::INT192 => (192, true),
                SolidityType::INT200 => (200, true),
                SolidityType::INT208 => (208, true),
                SolidityType::INT216 => (216, true),
                SolidityType::INT224 => (224, true),
                SolidityType::INT232 => (232, true),
                SolidityType::INT240 => (240, true),
                SolidityType::INT248 => (248, true),
                SolidityType::INT256 => (256, true),
                SolidityType::UINT8 => (8, false),
                SolidityType::UINT16 => (16, false),
                SolidityType::UINT24 => (24, false),
                SolidityType::UINT32 => (32, false),
                SolidityType::UINT40 => (40, false),
                SolidityType::UINT48 => (48, false),
                SolidityType::UINT56 => (56, false),
                SolidityType::UINT64 => (64, false),
                SolidityType::UINT72 => (72, false),
                SolidityType::UINT80 => (80, false),
                SolidityType::UINT88 => (88, false),
                SolidityType::UINT96 => (96, false),
                SolidityType::UINT104 => (104, false),
                SolidityType::UINT112 => (112, false),
                SolidityType::UINT120 => (120, false),
                SolidityType::UINT128 => (128, false),
                SolidityType::UINT136 => (136, false),
                SolidityType::UINT144 => (152, false),
                SolidityType::UINT152 => (152, false),
                SolidityType::UINT160 => (160, false),
                SolidityType::UINT168 => (168, false),
                SolidityType::UINT176 => (176, false),
                SolidityType::UINT184 => (184, false),
                SolidityType::UINT192 => (192, false),
                SolidityType::UINT200 => (200, false),
                SolidityType::UINT208 => (208, false),
                SolidityType::UINT216 => (216, false),
                SolidityType::UINT224 => (224, false),
                SolidityType::UINT232 => (232, false),
                SolidityType::UINT240 => (240, false),
                SolidityType::UINT248 => (248, false),
                SolidityType::UINT256 => (256, false),
            },
            _ => (256, false),
        }
    }
}

pub struct SoliditySubscriptExpression {
    pub expression: SubscriptExpression,
    pub is_lvalue: bool,
}

impl SoliditySubscriptExpression {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        let identifier = SoliditySubscriptExpression::base_identifier(
            Expression::SubscriptExpression(self.expression.clone()),
        );
        println!("{:?}", self.expression.clone());
        if identifier.enclosing_type.is_none() {
            panic!("Arrays not supported as local variables")
        }

        let enclosing = identifier.enclosing_type.clone();
        let enclosing = enclosing.unwrap_or_default();
        let offset = function_context
            .environment
            .property_offset(identifier.token.clone(), &enclosing);

        let mem_location = SoliditySubscriptExpression::nested_offset(
            self.expression.clone(),
            offset,
            function_context,
        );

        if self.is_lvalue {
            mem_location
        } else {
            YulExpression::FunctionCall(YulFunctionCall {
                name: "sload".to_string(),
                arguments: vec![mem_location],
            })
        }
    }

    pub fn base_identifier(expression: Expression) -> Identifier {
        if let Expression::Identifier(i) = expression {
            return i;
        }
        if let Expression::SubscriptExpression(s) = expression {
            return SoliditySubscriptExpression::base_identifier(Expression::Identifier(
                s.base_expression,
            ));
        }
        panic!("Can not find base identifier");
    }

    pub fn nested_offset(
        expression: SubscriptExpression,
        base_offset: u64,
        function_context: &mut FunctionContext,
    ) -> YulExpression {
        let index_expression = SolidityExpression {
            expression: *expression.index_expression.clone(),
            is_lvalue: false,
        }
        .generate(function_context);

        let base_type = function_context.environment.get_expression_type(
            &Expression::Identifier(expression.base_expression.clone()),
            &function_context.enclosing_type,
            &[],
            &[],
            &function_context.scope_context,
        );

        println!("{:?}", expression.base_expression.clone());

        let (a, b) = (
            YulExpression::Literal(YulLiteral::Num(base_offset)),
            index_expression,
        );

        match base_type.clone() {
            Type::ArrayType(_) => SolidityRuntimeFunction::storage_array_offset(a, b),
            Type::FixedSizedArrayType(_f) => {
                let size = function_context.environment.type_size(&base_type);
                SolidityRuntimeFunction::storage_fixed_array_offset(a, b, size)
            }
            Type::DictionaryType(_) => SolidityRuntimeFunction::storage_dictionary_offset_key(a, b),
            _ => panic!("Invalid Type"),
        }
    }
}

struct SoliditySelfExpression {
    pub is_lvalue: bool,
}

impl SoliditySelfExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> YulExpression {
        let ident = if function_context.in_struct_function {
            "_QuartzSelf".to_string()
        } else if self.is_lvalue {
            "0".to_string()
        } else {
            "".to_string()
        };

        YulExpression::Identifier(ident)
    }
}

struct SolidityBinaryExpression {
    pub expression: BinaryExpression,
    pub is_lvalue: bool,
}

impl SolidityBinaryExpression {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        if let BinOp::Dot = self.expression.op {
            if let Expression::FunctionCall(f) = *self.expression.rhs_expression.clone() {
                return SolidityFunctionCall { function_call: f }.generate(function_context);
            }

            return SolidityPropertyAccess {
                lhs: *self.expression.lhs_expression.clone(),
                rhs: *self.expression.rhs_expression.clone(),
                is_left: self.is_lvalue,
            }
            .generate(function_context);
        }

        if let BinOp::Equal = self.expression.op {
            let lhs = self.expression.lhs_expression.clone();
            let rhs = self.expression.rhs_expression.clone();
            return SolidityAssignment {
                lhs: *lhs,
                rhs: *rhs,
            }
            .generate(function_context);
        }

        let lhs = self.expression.lhs_expression.clone();
        let rhs = self.expression.rhs_expression.clone();
        let lhs = SolidityExpression {
            expression: *lhs,
            is_lvalue: self.is_lvalue,
        }
        .generate(function_context);
        let rhs = SolidityExpression {
            expression: *rhs,
            is_lvalue: self.is_lvalue,
        }
        .generate(function_context);

        match self.expression.op {
            BinOp::Plus => SolidityRuntimeFunction::add(lhs, rhs),
            BinOp::OverflowingPlus => YulExpression::FunctionCall(YulFunctionCall {
                name: "add".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::Times => SolidityRuntimeFunction::mul(lhs, rhs),
            BinOp::Divide => SolidityRuntimeFunction::div(lhs, rhs),
            BinOp::Minus => SolidityRuntimeFunction::sub(lhs, rhs),
            BinOp::OverflowingMinus => YulExpression::FunctionCall(YulFunctionCall {
                name: "sub".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::OverflowingTimes => YulExpression::FunctionCall(YulFunctionCall {
                name: "mul".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::Power => SolidityRuntimeFunction::power(lhs, rhs),
            BinOp::Percent => YulExpression::FunctionCall(YulFunctionCall {
                name: "mod".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::DoubleEqual => YulExpression::FunctionCall(YulFunctionCall {
                name: "eq".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::NotEqual => YulExpression::FunctionCall(YulFunctionCall {
                name: "iszero".to_string(),
                arguments: vec![YulExpression::FunctionCall(YulFunctionCall {
                    name: "eq".to_string(),
                    arguments: vec![lhs, rhs],
                })],
            }),
            BinOp::LessThan => YulExpression::FunctionCall(YulFunctionCall {
                name: "lt".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::LessThanOrEqual => panic!("Not Supported Op Token"),
            BinOp::GreaterThan => YulExpression::FunctionCall(YulFunctionCall {
                name: "gt".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::GreaterThanOrEqual => panic!("Not Supported Op Token"),
            BinOp::Or => YulExpression::FunctionCall(YulFunctionCall {
                name: "or".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::And => YulExpression::FunctionCall(YulFunctionCall {
                name: "and".to_string(),
                arguments: vec![lhs, rhs],
            }),
            BinOp::Dot => panic!("Unexpected Operator"),
            BinOp::Equal => panic!("Unexpected Operator"),
            BinOp::PlusEqual => panic!("Unexpected Operator"),
            BinOp::MinusEqual => panic!("Unexpected Operator"),
            BinOp::TimesEqual => panic!("Unexpected Operator"),
            BinOp::DivideEqual => panic!("Unexpected Operator"),
        }
    }
}

pub struct SolidityVariableDeclaration {
    pub declaration: VariableDeclaration,
}

impl SolidityVariableDeclaration {
    pub fn generate(&self, function_context: &FunctionContext) -> YulExpression {
        let allocate = SolidityRuntimeFunction::allocate_memory(
            function_context
                .environment
                .type_size(&self.declaration.variable_type)
                * 32,
        );
        YulExpression::VariableDeclaration(YulVariableDeclaration {
            declaration: mangle(&self.declaration.identifier.token),
            declaration_type: YulType::Any,
            expression: Option::from(Box::from(allocate)),
        })
    }
}
