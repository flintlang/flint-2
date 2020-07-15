use crate::moveir::*;

pub struct MoveContract {
    pub contract_declaration: ContractDeclaration,
    pub contract_behaviour_declarations: Vec<ContractBehaviourDeclaration>,
    pub struct_declarations: Vec<StructDeclaration>,
    pub asset_declarations: Vec<AssetDeclaration>,
    pub external_traits: Vec<TraitDeclaration>,
    pub environment: Environment,
}

impl MoveContract {
    pub fn generate(&self) -> String {
        let imports = self.external_traits.clone();
        let imports: Vec<TraitDeclaration> = imports
            .into_iter()
            .filter(|i| i.get_module_address().is_some())
            .collect();
        let mut imports: Vec<MoveIRStatement> = imports
            .into_iter()
            .map(|i| {
                let module_address = i.get_module_address();
                let module_address = module_address.unwrap();
                MoveIRStatement::Import(MoveIRModuleImport {
                    name: i.identifier.token,
                    address: module_address,
                })
            })
            .collect();
        let mut runtime_imports = MoveRuntimeTypes::get_all_imports();
        imports.append(&mut runtime_imports);
        let imports = imports.clone();

        let import_code: Vec<String> = imports.into_iter().map(|a| format!("{}", a)).collect();
        let import_code = import_code.join("\n");

        let runtime_funcions = MoveRuntimeFunction::get_all_functions();
        let runtime_functions = runtime_funcions.join("\n\n");

        let functions: Vec<FunctionDeclaration> = self
            .contract_behaviour_declarations
            .clone()
            .into_iter()
            .flat_map(|c| {
                c.members.into_iter().filter_map(|m| match m {
                    ContractBehaviourMember::FunctionDeclaration(f) => Some(f),
                    _ => None,
                })
            })
            .collect();

        let functions: Vec<String> = functions
            .into_iter()
            .map(|f| MoveFunction {
                function_declaration: f,
                environment: self.environment.clone(),
                is_contract_function: false,
                enclosing_type: self.contract_declaration.identifier.clone(),
            })
            .map(|f| f.generate(true))
            .collect();
        let functions = functions.join("\n\n");

        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: Default::default(),
            enclosing_type: "".to_string(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: false,
            is_constructor: false,
        };

        let members: Vec<VariableDeclaration> = self
            .contract_declaration
            .contract_members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let ContractMember::VariableDeclaration(v) = m {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();

        let members: Vec<VariableDeclaration> = members
            .into_iter()
            .filter(|m| !m.variable_type.is_dictionary_type())
            .collect();
        let members: Vec<String> = members
            .into_iter()
            .map(|v| {
                let declaration =
                    MoveFieldDeclaration { declaration: v }.generate(&function_context);
                return format!("{declaration}", declaration = declaration);
            })
            .collect();
        let members = members.join(",\n");

        let dict_resources: Vec<VariableDeclaration> = self
            .contract_declaration
            .contract_members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let ContractMember::VariableDeclaration(v) = m {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();

        let dict_resources: Vec<VariableDeclaration> = dict_resources
            .into_iter()
            .filter(|m| m.variable_type.is_dictionary_type())
            .collect();
        let dict_runtime = dict_resources.clone();
        let dict_resources: Vec<String> = dict_resources
            .into_iter()
            .map(|d| {
                let result_type = MoveType::move_type(
                    d.variable_type.clone(),
                    Option::from(self.environment.clone()),
                );
                let result_type = result_type.generate(&function_context);
                format!(
                    "resource {name} {{ \n value: {dic_type} \n }}",
                    name = mangle_dictionary(d.identifier.token.clone()),
                    dic_type = result_type
                )
            })
            .collect();

        let dict_resources = dict_resources.join("\n\n");

        let dict_runtime: Vec<String> = dict_runtime
            .into_iter()
            .map(|d| {
                let r_name = mangle_dictionary(d.identifier.token.clone());
                let result_type = MoveType::move_type(
                    d.variable_type.clone(),
                    Option::from(self.environment.clone()),
                );
                let result_type = result_type.generate(&function_context);
                format!(
                    "_get_{r_name}(__address_this: address): {r_type} acquires {r_name} {{
    let this: &mut Self.{r_name};
    let temp: &{r_type};
    let result: {r_type};
    this = borrow_global_mut<{r_name}>(move(__address_this));
    temp = &copy(this).value;
    result = *copy(temp);
    return move(result);
  }}

        _insert_{r_name}(__address_this: address, v: {r_type}) acquires {r_name} {{
    let new_value: Self.{r_name};
    let cur: &mut Self.{r_name};
    let b: bool;
    b = exists<{r_name}>(copy(__address_this));
    if (move(b)) {{
      cur = borrow_global_mut<{r_name}>(move(__address_this));
      *(&mut move(cur).value) = move(v);
    }} else {{
       new_value = {r_name} {{
      value: move(v)
    }};
    move_to_sender<{r_name}>(move(new_value));
    }}
    return;
  }}",
                    r_name = r_name,
                    r_type = result_type
                )
            })
            .collect();

        let dict_runtime = dict_runtime.join("\n\n");

        let structs: Vec<StructDeclaration> = self
            .struct_declarations
            .clone()
            .into_iter()
            .filter(|s| s.identifier.token != "Quartz_Global".to_string())
            .collect();
        let mut structs: Vec<String> = structs
            .into_iter()
            .map(|s| {
                MoveStruct {
                    struct_declaration: s,
                    environment: self.environment.clone(),
                }
                .generate()
            })
            .collect();
        let mut runtime_structs = MoveRuntimeTypes::get_all_declarations();
        structs.append(&mut runtime_structs);
        let structs = structs.clone();
        let structs = structs.join("\n\n");

        let struct_functions: Vec<String> = self
            .struct_declarations
            .clone()
            .into_iter()
            .map(|s| {
                MoveStruct {
                    struct_declaration: s,
                    environment: self.environment.clone(),
                }
                .generate_all_functions()
            })
            .collect();
        let struct_functions = struct_functions.join("\n\n");

        let assets: Vec<String> = self
            .asset_declarations
            .clone()
            .into_iter()
            .map(|a| {
                MoveAsset {
                    declaration: a,
                    environment: self.environment.clone(),
                }
                .generate()
            })
            .collect();
        let assets = assets.join("\n");

        let asset_functions: Vec<String> = self
            .asset_declarations
            .clone()
            .into_iter()
            .map(|s| {
                MoveAsset {
                    declaration: s,
                    environment: self.environment.clone(),
                }
                .generate_all_functions()
            })
            .collect();
        let asset_functions = asset_functions.join("\n\n");

        let mut initialiser_declaration = None;
        for declarations in self.contract_behaviour_declarations.clone() {
            for member in declarations.members.clone() {
                if let ContractBehaviourMember::SpecialDeclaration(s) = member {
                    if s.is_init() && s.is_public() {
                        initialiser_declaration = Some(s.clone());
                    }
                }
            }
        }

        if initialiser_declaration.is_none() {
            panic!("Public Initiliaser not found")
        }
        let initialiser_declaration = initialiser_declaration.unwrap();

        let scope = ScopeContext {
            parameters: vec![],
            local_variables: vec![],
            counter: 0,
        };

        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: scope,
            enclosing_type: self.contract_declaration.identifier.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: false,
            is_constructor: false,
        };

        let params = initialiser_declaration.head.parameters.clone();
        let params: Vec<MoveIRExpression> = params
            .into_iter()
            .map(|p| {
                MoveIdentifier {
                    identifier: p.identifier,
                    position: MovePosition::Left,
                }
                .generate(&function_context, false, false)
            })
            .collect();
        let params: Vec<String> = params.into_iter().map(|i| format!("{}", i)).collect();

        let params_values = initialiser_declaration.head.parameters.clone();
        let params_values: Vec<MoveIRExpression> = params_values
            .into_iter()
            .map(|p| {
                MoveIdentifier {
                    identifier: p.identifier,
                    position: MovePosition::Left,
                }
                .generate(&function_context, true, false)
            })
            .collect();
        let params_values: Vec<String> = params_values
            .into_iter()
            .map(|i| format!("{}", i))
            .collect();
        let params_values = params_values.join(", ");

        let param_types = initialiser_declaration.head.parameters.clone();
        let param_types: Vec<MoveIRType> = param_types
            .into_iter()
            .map(|p| {
                MoveType::move_type(p.type_assignment, Option::from(self.environment.clone()))
                    .generate(&function_context)
            })
            .collect();
        let param_types: Vec<String> = param_types.into_iter().map(|i| format!("{}", i)).collect();

        let parameters: Vec<String> = params
            .into_iter()
            .zip(param_types)
            .map(|(k, v)| format!("{name}: {t}", name = k, t = v))
            .collect();

        let parameters = parameters.join(", ");

        let mut statements = initialiser_declaration.body.clone();
        let properties = self
            .contract_declaration
            .get_variable_declarations_without_dict();

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: Default::default(),
            enclosing_type: self.contract_declaration.identifier.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: false,
            is_constructor: true,
        };

        let body;

        for property in properties {
            let property_type = MoveType::move_type(
                property.variable_type.clone(),
                Option::from(self.environment.clone()),
            );
            let property_type = property_type.generate(&function_context);
            function_context.emit(MoveIRStatement::Expression(
                MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                    identifier: format!("__this_{}", property.identifier.token),
                    declaration_type: property_type,
                }),
            ));
        }

        let unassigned = self
            .contract_declaration
            .get_variable_declarations_without_dict();
        let mut unassigned: Vec<Identifier> =
            unassigned.into_iter().map(|v| v.identifier).collect();

        while !(statements.is_empty() || unassigned.is_empty()) {
            let statement = statements.remove(0);
            if let Statement::Expression(e) = statement.clone() {
                if let Expression::BinaryExpression(b) = e {
                    if let BinOp::Equal = b.op {
                        if let Expression::Identifier(i) = *b.lhs_expression.clone() {
                            if i.enclosing_type.is_some() {
                                let enclosing = i.enclosing_type.clone();
                                let enclosing = enclosing.unwrap();
                                if enclosing == self.contract_declaration.identifier.token.clone() {
                                    unassigned = unassigned
                                        .into_iter()
                                        .filter(|u| u.token != i.token)
                                        .collect();
                                }
                            }
                            if let Expression::BinaryExpression(lb) = *b.lhs_expression.clone() {
                                let op = lb.op.clone();
                                let lhs = *lb.lhs_expression;
                                let rhs = *lb.rhs_expression;
                                if let BinOp::Dot = op {
                                    if let Expression::SelfExpression = lhs {
                                        if let Expression::Identifier(i) = rhs {
                                            unassigned = unassigned
                                                .into_iter()
                                                .filter(|u| u.token != i.token)
                                                .collect();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                let move_statement = MoveStatement {
                    statement: statement.clone(),
                }
                .generate(&mut function_context);
                function_context.emit(move_statement);
            }
        }

        let fields = self
            .contract_declaration
            .get_variable_declarations_without_dict();
        let fields: Vec<(String, MoveIRExpression)> = fields
            .into_iter()
            .map(|p| {
                (
                    p.identifier.token.clone(),
                    MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(
                        MoveIRExpression::Identifier(format!("__this_{}", p.identifier.token)),
                    ))),
                )
            })
            .collect();
        let constructor = MoveIRExpression::StructConstructor(MoveIRStructConstructor {
            identifier: Identifier::generated("T"),
            fields,
        });

        if !(statements.is_empty()) {
            function_context.is_constructor = false;

            let shadow = "Quartz$self".to_string();

            let self_type = MoveType::move_type(
                Type::type_from_identifier(self.contract_declaration.identifier.clone()),
                Option::from(self.environment.clone()),
            )
            .generate(&function_context);

            let emit = MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                identifier: "this".to_string(),
                declaration_type: MoveIRType::MutableReference(Box::from(self_type.clone())),
            });
            function_context.emit(MoveIRStatement::Expression(emit));

            let emit = MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                identifier: mangle(shadow.clone()),
                declaration_type: self_type,
            });

            function_context.emit(MoveIRStatement::Expression(emit));

            let self_identifier = MoveSelf {
                token: Identifier::SELF.to_string(),
                position: Default::default(),
            };
            let self_identifier = Identifier {
                token: self_identifier.token.clone(),
                enclosing_type: None,
                line_info: Default::default(),
            };

            let mut scope = function_context.scope_context.clone();
            scope.local_variables.push(VariableDeclaration {
                declaration_token: None,
                identifier: self_identifier,
                variable_type: Type::InoutType(InoutType {
                    key_type: Box::new(Type::UserDefinedType(Identifier {
                        token: function_context.enclosing_type.clone(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    })),
                }),
                expression: None,
            });

            while !statements.is_empty() {
                let statement = statements.remove(0);
                let statement = MoveStatement { statement }.generate(&mut function_context);
                function_context.emit(statement);
            }

            function_context.emit_release_references();
            body = function_context.generate()
        } else {
            function_context.emit_release_references();
            function_context.emit(MoveIRStatement::Return(constructor));
            body = function_context.generate()
        }

        let initialiser = format!(
            "new({params}): Self.T {{ {body} }} \n\n \
             public publish({params}) {{ \n move_to_sender<T>(Self.new({values})); \n return; \n }}",
            params = parameters,
            body = body,
            values = params_values
        );

        return format!("module {name} {{ \n  {imports} \n resource T {{ \n {members} \n }} {dict_resources} \n {assets}  \n {structs} \n {init} \n \n {asset_functions} \n \n {struct_functions} \n {functions} \n {runtime} \n {dict_runtime} }}"
                       , name = self.contract_declaration.identifier.token, functions = functions, members = members,
                       assets = assets, asset_functions = asset_functions, structs = structs, dict_resources = dict_resources,
                       init = initialiser, struct_functions = struct_functions, imports = import_code,
                       runtime = runtime_functions, dict_runtime = dict_runtime
        );
    }
}
