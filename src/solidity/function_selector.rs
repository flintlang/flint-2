use super::*;

pub struct SolidityFunctionSelector {
    pub fallback: Option<SpecialDeclaration>,
    pub functions: Vec<SolidityFunction>,
    pub enclosing: Identifier,
    pub environment: Environment,
}

impl SolidityFunctionSelector {
    pub fn generate(&self) -> String {
        let state = Expression::Identifier(Identifier {
            token: format!("quartzState${}", self.enclosing.token.clone()),
            enclosing_type: None,
            line_info: Default::default(),
        });

        let state = Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(Expression::SelfExpression),
            rhs_expression: Box::new(state),
            op: BinOp::Dot,
            line_info: Default::default(),
        });

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: Default::default(),
            in_struct_function: false,
            block_stack: vec![YulBlock { statements: vec![] }],
            enclosing_type: self.enclosing.token.clone(),
            counter: 0,
        };

        let state = SolidityExpression {
            expression: state,
            is_lvalue: false,
        }
        .generate(&mut function_context);

        let protection = YulStatement::Inline(format!(
            "if eq({state}, 10000) {{ revert(0, 0)}}",
            state = state
        ));

        let selector = SolidityRuntimeFunction::selector();
        let mut _hasher = Keccak256::digest(b"helo");
        let cases: Vec<String> = self
            .functions
            .clone()
            .into_iter()
            .map(|f| {
                let signature = f.mangled_signature();
                let hash = Keccak256::digest(signature.as_bytes());
                let mut hex = encode(hash);
                hex.truncate(8);
                let hash = format!("0x{hash}", hash = hex);
                let caller_protection_check = SolidityCallerProtectionCheck {
                    caller_protections: f.caller_protections.clone(),
                    revert: false,
                    variable: "_quartzCallerCheck".to_string(),
                }
                .generate(&self.enclosing.token.clone(), self.environment.clone());

                let value_check = if !f.declaration.is_payable() {
                    format!(
                        "{check}({value}) \n",
                        check = SolidityRuntimeFunction::CheckNoValue.mangle_runtime(),
                        value = SolidityRuntimeFunction::call_value()
                    )
                } else {
                    format!("")
                };

                let wrapper = if f.has_any_caller() {
                    format!("")
                } else {
                    SolidityWrapperFunction::get_prefix_hard()
                };

                let parameters = f.declaration.head.parameters.clone();
                let parameters: Vec<SolidityIRType> = parameters
                    .into_iter()
                    .map(|p| SolidityIRType::map_to_solidity_type(p.type_assignment))
                    .collect();
                let parameters: Vec<String> = parameters
                    .into_iter()
                    .enumerate()
                    .map(|(k, v)| match v {
                        SolidityIRType::Uint256 => {
                            SolidityRuntimeFunction::decode_as_uint(k as u64)
                        }
                        SolidityIRType::Address => {
                            SolidityRuntimeFunction::decode_as_address(k as u64)
                        }
                        SolidityIRType::Bytes32 => {
                            SolidityRuntimeFunction::decode_as_uint(k as u64)
                        }
                    })
                    .collect();
                let parameters = parameters.join(", ");
                let mut call = format!(
                    "{wrapper}{name}({args})",
                    wrapper = wrapper,
                    name = f.clone().declaration.mangled_identifier.unwrap_or_default(),
                    args = parameters
                );

                if f.declaration.get_result_type().is_some() {
                    let result = f.declaration.get_result_type().clone();
                    let result = result.unwrap();
                    if SolidityIRType::if_maps_to_solidity_type(result.clone()) {
                        let _result = SolidityIRType::map_to_solidity_type(result);
                        call = SolidityRuntimeFunction::return_32_bytes(call);
                    }
                }

                let case_body = format!(
                    "{caller_protection} \n {value_check}{call}",
                    caller_protection = caller_protection_check,
                    value_check = value_check,
                    call = call
                );
                format!("case {hash} {{ {body} }}", hash = hash, body = case_body)
            })
            .collect();

        let cases = cases.join("\n");

        let fallback = if self.fallback.is_some() {
            panic!("User supplied Fallback not currently supported")
        } else {
            "revert(0, 0)".to_string()
        };

        format!(
            "{protection} \n \
             switch {selector} \n\
             {cases} \n\
             default {{ \n \
             {fallback} \n\
             }}",
            protection = protection,
            selector = selector,
            cases = cases,
            fallback = fallback
        )
    }
}

pub struct SolidityWrapperFunction {
    pub function: SolidityFunction,
}

impl SolidityWrapperFunction {
    pub fn generate(&self, t: &TypeIdentifier) -> String {
        let caller_check = SolidityCallerProtectionCheck {
            caller_protections: self.function.caller_protections.clone(),
            revert: false,
            variable: "_QuartzCallerCheck".to_string(),
        };

        let _caller_code = caller_check.generate(t, self.function.environment.clone());

        unimplemented!()
    }

    pub fn get_prefix_hard() -> String {
        "quartzAttemptCallWrapperHard$".to_string()
    }
}

struct SolidityCallerProtectionCheck {
    pub caller_protections: Vec<CallerProtection>,
    pub revert: bool,
    pub variable: String,
}

impl SolidityCallerProtectionCheck {
    pub fn generate(&self, t: &TypeIdentifier, environment: Environment) -> String {
        let checks: Vec<String> = self
            .caller_protections
            .clone()
            .into_iter()
            .filter_map(|c| {
                if !c.is_any() {
                    let caller_type = environment.get_property_type(
                        c.name(),
                        t,
                        ScopeContext {
                            parameters: vec![],
                            local_variables: vec![],
                            counter: 0,
                        },
                    );

                    let offset = environment.property_offset(c.name(), t);

                    let _function_context = FunctionContext {
                        environment: environment.clone(),
                        scope_context: Default::default(),
                        in_struct_function: false,
                        block_stack: vec![YulBlock { statements: vec![] }],
                        enclosing_type: t.to_string(),
                        counter: 0,
                    };

                    match caller_type {
                        Type::Address => {
                            let address = format!("sload({offset})", offset = offset);
                            let check =
                                SolidityRuntimeFunction::is_valid_caller_protection(address);
                            Option::from(format!(
                                "{variable} := add({variable}, {check})",
                                variable = self.variable,
                                check = check
                            ))
                        }
                        _ => unimplemented!(),
                    }
                } else {
                    None
                }
            })
            .collect();

        let revert = if self.revert {
            format!(
                "if eq({variable}, 0) {{ revert(0, 0) }}",
                variable = self.variable
            )
        } else {
            format!("")
        };

        if checks.is_empty() {
            return format!("");
        } else {
            let checks = checks.join("\n");
            return format!(
                "let {var} := 0 \n {checks} {revert}",
                var = self.variable,
                checks = checks,
                revert = revert
            );
        }
    }
}