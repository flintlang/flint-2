use super::asset::MoveAsset;
use super::declaration::MoveFieldDeclaration;
use super::expression::MoveExpression;
use super::function::{FunctionContext, MoveFunction};
use super::identifier::MoveIdentifier;
use super::ir::{
    MoveIRAssignment, MoveIRBlock, MoveIRExpression, MoveIRFunctionCall, MoveIRModuleImport,
    MoveIROperation, MoveIRStatement, MoveIRStructConstructor, MoveIRTransfer, MoveIRType,
    MoveIRVariableDeclaration,
};
use super::r#struct::MoveStruct;
use super::r#type::{move_runtime_types, MoveType};
use super::runtime_function::MoveRuntimeFunction;
use super::statement::MoveStatement;
use super::MovePosition;
use crate::ast::{
    mangle_dictionary, ArrayType, AssetDeclaration, BinOp, ContractBehaviourDeclaration,
    ContractBehaviourMember, ContractDeclaration, ContractMember, Expression, FixedSizedArrayType,
    Identifier, InoutType, Statement, StructDeclaration, TraitDeclaration, Type,
    VariableDeclaration,
};
use crate::context::ScopeContext;
use crate::environment::Environment;
use crate::moveir::identifier::MoveSelf;
use crate::moveir::preprocessor::MovePreProcessor;

pub struct MoveContract {
    pub contract_declaration: ContractDeclaration,
    pub contract_behaviour_declarations: Vec<ContractBehaviourDeclaration>,
    pub struct_declarations: Vec<StructDeclaration>,
    pub asset_declarations: Vec<AssetDeclaration>,
    pub external_traits: Vec<TraitDeclaration>,
    pub environment: Environment,
}

impl MoveContract {
    const SHADOW: &'static str = "Flint_self";

    pub(crate) fn generate(&self) -> String {
        let import_code = self.generate_imports();

        let runtime_functions = MoveRuntimeFunction::get_all_functions().join("\n\n");
        let functions = self
            .contract_behaviour_declarations
            .clone()
            .into_iter()
            .flat_map(|c| {
                c.members.into_iter().filter_map(|m| match m {
                    ContractBehaviourMember::FunctionDeclaration(f) => Some(f),
                    _ => None,
                })
            })
            .map(|f| {
                MoveFunction {
                    function_declaration: f,
                    environment: self.environment.clone(),
                    is_contract_function: false,
                    enclosing_type: self.contract_declaration.identifier.clone(),
                }
                .generate(true)
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: Default::default(),
            enclosing_type: "".to_string(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: false,
            is_constructor: false,
        };

        let variable_declarations = self
            .contract_declaration
            .contract_members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let ContractMember::VariableDeclaration(v, _) = m {
                    Some(v)
                } else {
                    None
                }
            })
            .collect::<Vec<VariableDeclaration>>();

        let members = variable_declarations
            .clone()
            .into_iter()
            .filter(|m| !matches!(m.variable_type, Type::DictionaryType(_)))
            .map(|v| {
                format!(
                    "{}",
                    MoveFieldDeclaration { declaration: v }.generate(&function_context)
                )
            })
            .collect::<Vec<String>>()
            .join(",\n");

        let (dict_names, dict_resources, dict_runtime, dict_initialisation) =
            self.get_dict_code(&function_context, &*variable_declarations);

        let dict_names: String = dict_names.into_iter().collect::<Vec<String>>().join(", ");

        let structs_declarations = self
            .struct_declarations
            .clone()
            .into_iter()
            .filter(|s| s.identifier.token != crate::environment::FLINT_GLOBAL)
            .map(|s| MoveStruct {
                struct_declaration: s,
                environment: self.environment.clone(),
            })
            .collect::<Vec<MoveStruct>>();

        let mut structs = structs_declarations
            .iter()
            .map(|dec| dec.generate())
            .collect::<Vec<String>>();

        let mut runtime_structs = move_runtime_types::get_all_declarations();
        structs.append(&mut runtime_structs);
        let structs = structs.join("\n\n");

        let struct_functions = structs_declarations
            .into_iter()
            .map(|s| s.generate_all_functions())
            .collect::<Vec<String>>()
            .join("\n\n");

        let asset_declarations = self
            .asset_declarations
            .clone()
            .into_iter()
            .map(|a| MoveAsset {
                declaration: a,
                environment: self.environment.clone(),
            })
            .collect::<Vec<MoveAsset>>();

        let assets = asset_declarations
            .iter()
            .map(|dec| dec.generate())
            .collect::<Vec<String>>()
            .join("\n");

        let asset_functions = asset_declarations
            .iter()
            .map(|dec| dec.generate_all_functions())
            .collect::<Vec<String>>()
            .join("\n\n");

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
        let params: Vec<String> = params
            .into_iter()
            .map(|p| {
                MoveIdentifier {
                    identifier: p.identifier,
                    position: MovePosition::Left,
                }
                .generate(&function_context, false, false)
                .to_string()
            })
            .collect();

        let params_values = initialiser_declaration.head.parameters.clone();
        let mut params_values = params_values
            .into_iter()
            .map(|p| {
                MoveIdentifier {
                    identifier: p.identifier,
                    position: MovePosition::Left,
                }
                .generate(&function_context, true, false)
                .to_string()
            })
            .collect::<Vec<String>>()
            .join(", ");

        let param_types = initialiser_declaration.head.parameters.clone();
        let param_types = param_types
            .into_iter()
            .map(|p| {
                MoveType::move_type(p.type_assignment, Option::from(self.environment.clone()))
                    .generate(&function_context)
                    .to_string()
            })
            .collect::<Vec<String>>();

        let mut parameters: Vec<String> = params
            .into_iter()
            .zip(param_types)
            .map(|(k, v)| format!("{name}: {t}", name = k, t = v))
            .collect();

        let mut statements = initialiser_declaration.body;

        let properties: Vec<_> = self
            .contract_declaration
            .get_variable_declarations_without_dict()
            .collect();

        let scope_context = statements
            .clone()
            .into_iter()
            .filter_map(|statement| {
                if let Statement::Expression(Expression::VariableDeclaration(declaration)) =
                    statement
                {
                    Some(declaration)
                } else {
                    None
                }
            })
            .collect::<Vec<VariableDeclaration>>();

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: ScopeContext {
                parameters: vec![],
                local_variables: scope_context,
                counter: 0,
            },
            enclosing_type: self.contract_declaration.identifier.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: false,
            is_constructor: true,
        };

        let body;

        let mut field_declarations = vec![];

        for property in &properties {
            let property_type = MoveType::move_type(
                property.variable_type.clone(),
                Option::from(self.environment.clone()),
            );
            let property_type = property_type.generate(&function_context);
            let identifier = format!("__this_{}", property.identifier.token);

            field_declarations.push(MoveIRStatement::Expression(
                MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                    identifier: identifier.clone(),
                    declaration_type: property_type.clone(),
                }),
            ));

            function_context.emit(MoveIRStatement::Expression(
                MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                    identifier,
                    declaration_type: property_type,
                }),
            ));
        }

        for property in properties {
            if let Some(ref expr) = property.expression {
                let identifier = format!("__this_{}", property.identifier.token);

                if let crate::ast::expressions::Expression::ArrayLiteral(array) = &**expr {
                    let elements: Vec<MoveIRExpression> = array
                        .clone()
                        .elements
                        .into_iter()
                        .map(|e| {
                            MoveExpression {
                                expression: e,
                                position: Default::default(),
                            }
                            .generate(&function_context)
                        })
                        .collect();

                    if let Type::FixedSizedArrayType(FixedSizedArrayType { key_type, .. })
                    | Type::ArrayType(ArrayType { key_type }) = &property.variable_type
                    {
                        let array_type = MoveType::move_type(*key_type.clone(), None)
                            .generate(&function_context);

                        function_context.emit(MoveIRStatement::Expression(
                            MoveIRExpression::Assignment(MoveIRAssignment {
                                identifier: identifier.clone(),
                                expression: Box::from(MoveIRExpression::Vector(
                                    crate::moveir::ir::MoveIRVector {
                                        elements: elements.clone(),
                                        vec_type: Some(array_type.clone()),
                                    },
                                )),
                            }),
                        ));

                        let elements: Vec<String> =
                            elements.into_iter().map(|e| format!("{}", e)).collect();

                        for element in elements {
                            function_context.emit(MoveIRStatement::Expression(
                                MoveIRExpression::Inline(format!(
                                    "Vector.push_back<{}>(&mut {}, {})",
                                    array_type.clone(),
                                    identifier,
                                    element
                                )),
                            ));
                        }
                    }
                } else {
                    let statement = MoveIRStatement::Expression(MoveIRExpression::Assignment(
                        MoveIRAssignment {
                            identifier,
                            expression: Box::from(
                                MoveExpression {
                                    expression: (**expr).clone(),
                                    position: Default::default(),
                                }
                                .generate(&function_context),
                            ),
                        },
                    ));
                    function_context.emit(statement);
                }
            }
        }

        for initialiser in dict_initialisation {
            function_context.emit(initialiser);
        }

        let mut unassigned: Vec<_> = self
            .contract_declaration
            .get_variable_declarations_without_dict()
            .map(|v| &v.identifier)
            .collect();

        // TODO refactor this loop
        while !(statements.is_empty() || unassigned.is_empty()) {
            match statements.remove(0) {
                Statement::Expression(e) => {
                    if let Expression::BinaryExpression(ref b) = e {
                        if let BinOp::Equal = b.op {
                            if let Expression::Identifier(ref i) = &*b.lhs_expression {
                                if let Some(ref enclosing) = i.enclosing_type {
                                    if *enclosing == self.contract_declaration.identifier.token {
                                        unassigned = unassigned
                                            .into_iter()
                                            .filter(|u| u.token != i.token)
                                            .collect();
                                    }
                                }
                                if let Expression::BinaryExpression(ref lb) = &*b.lhs_expression {
                                    let op = lb.op.clone();
                                    let lhs = &*lb.lhs_expression;
                                    let rhs = &*lb.rhs_expression;
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
                        statement: Statement::Expression(e),
                    }
                    .generate(&mut function_context);
                    function_context.emit(move_statement);
                }
                assertion @ Statement::Assertion(_) => {
                    let move_statement = MoveStatement {
                        statement: assertion,
                    }
                    .generate(&mut function_context);
                    function_context.emit(move_statement);
                }
                _ => (),
            }
        }

        let fields = self
            .contract_declaration
            .get_variable_declarations_without_dict()
            .map(|p| {
                (
                    p.identifier.token.clone(),
                    MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(
                        MoveIRExpression::Identifier(format!("__this_{}", p.identifier.token)),
                    ))),
                )
            })
            .collect();
        let mut constructor = MoveIRExpression::StructConstructor(MoveIRStructConstructor {
            identifier: Identifier::generated("T"),
            fields,
        });

        let initialiser_statements: Vec<MoveIRStatement> =
            function_context.clone().pop_block().statements;

        for initialiser_statement in initialiser_statements {
            if let MoveIRStatement::Expression(MoveIRExpression::Assignment(assignment)) =
                initialiser_statement
            {
                if let MoveIRExpression::Operation(MoveIROperation::MutableReference(id)) =
                    *assignment.expression.clone()
                {
                    if let MoveIRExpression::Identifier(id) = *id {
                        for field_declaration in &field_declarations {
                            if let MoveIRStatement::Expression(
                                MoveIRExpression::VariableDeclaration(vd),
                            ) = field_declaration
                            {
                                if id == vd.identifier {
                                    replace_borrowed_references(
                                        &mut constructor,
                                        &vd.identifier,
                                        &assignment.identifier,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if !(statements.is_empty()) {
            function_context.is_constructor = false;

            let self_type = MoveType::move_type(
                Type::type_from_identifier(self.contract_declaration.identifier.clone()),
                Option::from(self.environment.clone()),
            )
            .generate(&function_context);

            let emit = MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                identifier: Identifier::SELF.to_string(),
                declaration_type: MoveIRType::MutableReference(Box::from(self_type.clone())),
            });
            function_context.emit(MoveIRStatement::Expression(emit));

            let emit = MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                identifier: MoveContract::SHADOW.to_string(),
                declaration_type: self_type,
            });

            function_context.emit(MoveIRStatement::Expression(emit));

            let self_identifier = MoveSelf {
                token: Identifier::SELF.to_string(),
                position: Default::default(),
            };
            let self_identifier = Identifier {
                token: self_identifier.token,
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

        let params_without_signer = parameters.join(", ");

        let initialiser: String;
        let publisher: String;
        parameters.push(format!("account: {}", MovePreProcessor::SIGNER_TYPE));
        let parameters = parameters.join(", ");

        if !dict_names.is_empty() {
            initialiser = format!(
                "new({params}): Self.T acquires {dict_names} {{ \n{body}\n }} \n",
                params = parameters,
                dict_names = dict_names,
                body = body,
            );

            params_values = if !params_values.is_empty() {
                format!("{}, copy(account)", params_values)
            } else {
                "copy(account)".to_string()
            };

            publisher = format!("public publish({params}) acquires {dict_names} {{ \n let t: Self.T; \nt = Self.new({values});\n move_to<T>(move(account), move(t)); \nreturn; \n }}",
                                params = parameters,
                                dict_names = dict_names,
                                values = params_values);
        } else {
            initialiser = format!(
                "new({params}): Self.T {{ \n{body}\n }} \n",
                params = params_without_signer,
                body = body,
            );

            publisher = format!("public publish({params}) {{ \n let t: Self.T; \nt = Self.new({values});\n move_to<T>(move(account), move(t)); \nreturn; \n }}",
                                params = parameters,
                                values = params_values);
        }

        return format!("module {name} {{ \n  {imports} \n resource T {{ \n {members} \n }} {dict_resources} \n {assets}  \n {structs} \n {init} \n {publish}\n {asset_functions} \n \n {struct_functions} \n {functions} \n {runtime} \n {dict_runtime} }}"
                       , name = self.contract_declaration.identifier.token, functions = functions, members = members,
                       assets = assets, asset_functions = asset_functions, structs = structs, dict_resources = dict_resources,
                       init = initialiser, publish = publisher, struct_functions = struct_functions, imports = import_code,
                       runtime = runtime_functions, dict_runtime = dict_runtime
        );
    }

    fn generate_imports(&self) -> String {
        let imports = self.external_traits.clone();
        let mut imports: Vec<MoveIRStatement> = imports
            .into_iter()
            .filter_map(|i| {
                i.get_module_address().map(|module_address| {
                    MoveIRStatement::Import(MoveIRModuleImport {
                        name: i.identifier.token,
                        address: module_address,
                    })
                })
            })
            .collect();
        let mut runtime_imports = move_runtime_types::get_all_imports();
        imports.append(&mut runtime_imports);

        imports
            .into_iter()
            .map(|a| format!("{}", a))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn get_dict_code(
        &self,
        function_context: &FunctionContext,
        variable_declarations: &[VariableDeclaration],
    ) -> (Vec<String>, String, String, Vec<MoveIRStatement>) {
        let dict_resources: Vec<&VariableDeclaration> = variable_declarations
            .iter()
            .filter(|m| matches!(m.variable_type, Type::DictionaryType(_)))
            .collect();

        let dict_runtime = dict_resources.clone();

        let mut dict_initialisation: Vec<MoveIRStatement> = vec![];

        let mut dict_names: Vec<String> = vec![];

        let dict_resources = dict_resources
            .into_iter()
            .map(|d| {
                let result_type = MoveType::move_type(
                    d.variable_type.clone(),
                    Option::from(self.environment.clone()),
                );
                let result_type = result_type.generate(&function_context);
                dict_names.push(mangle_dictionary(&d.identifier.token));
                format!(
                    "resource {name} {{ \n value: {dic_type} \n }}",
                    name = mangle_dictionary(&d.identifier.token),
                    dic_type = result_type
                )
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        let dict_runtime = dict_runtime
            .into_iter()
            .map(|d| {
                let r_name = mangle_dictionary(&d.identifier.token);
                let result_type = MoveType::move_type(
                    d.variable_type.clone(),
                    Option::from(self.environment.clone()),
                );
                let result_type = result_type.generate(&function_context);

                if let Some(expr) = &d.expression {
                    if let Expression::DictionaryLiteral(dict_literal) = &**expr {
                        for elem in &dict_literal.elements {
                            let index = MoveExpression {
                                expression: elem.0.clone(),
                                position: Default::default(),
                            }
                                .generate(function_context);

                            let rhs = MoveExpression {
                                expression: elem.1.clone(),
                                position: Default::default(),
                            }
                                .generate(&function_context);

                            let f_name = format!("Self._insert_{}", r_name);
                            let caller_argument = Identifier::generated("account");
                            let caller_argument = MoveIdentifier {
                                identifier: caller_argument.clone(),
                                position: Default::default(),
                            }
                            .generate(function_context, false, true);

                            dict_initialisation.push(MoveIRStatement::Expression(
                                MoveIRExpression::FunctionCall(
                                    MoveIRFunctionCall {
                                        identifier: f_name,
                                        arguments: vec![index, rhs, caller_argument],
                                    },
                                ),
                            ));
                        }
                    }
                }

                format!(
                    "_get_{r_name}(_address_this: address): {r_type} acquires {r_name} {{
    let this: &mut Self.{r_name};
    let temp: &{r_type};
    let result: {r_type};
    this = borrow_global_mut<{r_name}>(move(_address_this));
    temp = &copy(this).value;
    result = *copy(temp);
    return move(result);
  }}

        _insert_{r_name}(_address_this: address, v: {r_type}, _contract_caller: &signer) acquires {r_name} {{
    let new_value: Self.{r_name};
    let cur: &mut Self.{r_name};
    let b: bool;
    b = exists<{r_name}>(copy(_address_this));
    if (move(b)) {{
      cur = borrow_global_mut<{r_name}>(move(_address_this));
      *(&mut move(cur).value) = move(v);
    }} else {{
       new_value = {r_name} {{
      value: move(v)
    }};
    move_to<{r_name}>(move(_contract_caller), move(new_value));
    }}
    return;
  }}",
                    r_name = r_name,
                    r_type = result_type
                )
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        (
            dict_names,
            dict_resources,
            dict_runtime,
            dict_initialisation,
        )
    }
}

fn replace_borrowed_references(
    constructor: &mut MoveIRExpression,
    field_declaration: &str,
    declaration: &str,
) {
    let mut fields = vec![];

    if let MoveIRExpression::StructConstructor(constructor) = constructor {
        for field in &constructor.fields {
            if let (token, MoveIRExpression::Transfer(MoveIRTransfer::Move(identifier))) = field {
                if let MoveIRExpression::Identifier(id) = &**identifier {
                    if *id == *field_declaration {
                        fields.push((
                            token.clone(),
                            MoveIRExpression::Operation(MoveIROperation::Dereference(Box::new(
                                MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::new(
                                    MoveIRExpression::Identifier(declaration.to_string()),
                                ))),
                            ))),
                        ));

                        //remove temp variable from post-statements
                        continue;
                    }
                }
            }

            fields.push(field.clone());
        }

        constructor.fields = fields;
    }
}
