//! Methods to enforce constraints on expressions in a resolved aleo program.

use crate::constraints::{new_scope_from_variable, ResolvedProgram, ResolvedValue};
use crate::{
    new_variable_from_variable, Expression, RangeOrExpression, ResolvedStructMember,
    SpreadOrExpression, StructMember, Variable,
};

use snarkos_models::curves::{Field, PrimeField};
use snarkos_models::gadgets::r1cs::ConstraintSystem;
use snarkos_models::gadgets::utilities::boolean::Boolean;

impl<F: Field + PrimeField, CS: ConstraintSystem<F>> ResolvedProgram<F, CS> {
    /// Enforce a variable expression by getting the resolved value
    fn enforce_variable(
        &mut self,
        scope: String,
        unresolved_variable: Variable<F>,
    ) -> ResolvedValue<F> {
        // Evaluate the variable name in the current function scope
        let variable_name = new_scope_from_variable(scope, &unresolved_variable);

        if self.contains_name(&variable_name) {
            // Reassigning variable to another variable
            self.get_mut(&variable_name).unwrap().clone()
        } else if self.contains_variable(&unresolved_variable) {
            // Check global scope (function and struct names)
            self.get_mut_variable(&unresolved_variable).unwrap().clone()
        } else {
            unimplemented!("variable declaration \"{}\" not found", unresolved_variable)
        }
    }

    /// Enforce numerical operations
    fn enforce_add_expression(
        &mut self,
        cs: &mut CS,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => {
                Self::enforce_u32_add(cs, num1, num2)
            }
            (ResolvedValue::FieldElement(fe1), ResolvedValue::FieldElement(fe2)) => {
                self.enforce_field_add(fe1, fe2)
            }
            (val1, val2) => unimplemented!("cannot add {} + {}", val1, val2),
        }
    }

    fn enforce_sub_expression(
        &mut self,
        cs: &mut CS,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => {
                Self::enforce_u32_sub(cs, num1, num2)
            }
            (ResolvedValue::FieldElement(fe1), ResolvedValue::FieldElement(fe2)) => {
                self.enforce_field_sub(fe1, fe2)
            }
            (val1, val2) => unimplemented!("cannot subtract {} - {}", val1, val2),
        }
    }

    fn enforce_mul_expression(
        &mut self,
        cs: &mut CS,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => {
                Self::enforce_u32_mul(cs, num1, num2)
            }
            (ResolvedValue::FieldElement(fe1), ResolvedValue::FieldElement(fe2)) => {
                self.enforce_field_mul(fe1, fe2)
            }
            (val1, val2) => unimplemented!("cannot multiply {} * {}", val1, val2),
        }
    }

    fn enforce_div_expression(
        &mut self,
        cs: &mut CS,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => {
                Self::enforce_u32_div(cs, num1, num2)
            }
            (ResolvedValue::FieldElement(fe1), ResolvedValue::FieldElement(fe2)) => {
                self.enforce_field_div(fe1, fe2)
            }
            (val1, val2) => unimplemented!("cannot divide {} / {}", val1, val2),
        }
    }
    fn enforce_pow_expression(
        &mut self,
        cs: &mut CS,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => {
                Self::enforce_u32_pow(cs, num1, num2)
            }
            (ResolvedValue::FieldElement(fe1), ResolvedValue::U32(num2)) => {
                self.enforce_field_pow(fe1, num2)
            }
            (_, ResolvedValue::FieldElement(num2)) => {
                unimplemented!("exponent power must be an integer, got field {}", num2)
            }
            (val1, val2) => unimplemented!("cannot enforce exponentiation {} * {}", val1, val2),
        }
    }

    /// Evaluate Boolean operations
    fn evaluate_eq_expression(
        &mut self,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::Boolean(bool1), ResolvedValue::Boolean(bool2)) => {
                Self::boolean_eq(bool1, bool2)
            }
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => Self::u32_eq(num1, num2),
            (ResolvedValue::FieldElement(fe1), ResolvedValue::FieldElement(fe2)) => {
                Self::field_eq(fe1, fe2)
            }
            (val1, val2) => unimplemented!("cannot enforce equality between {} == {}", val1, val2),
        }
    }

    /// Enforce Boolean operations, returns true on success
    fn enforce_eq_expression(
        &mut self,
        cs: &mut CS,
        left: ResolvedValue<F>,
        right: ResolvedValue<F>,
    ) -> ResolvedValue<F> {
        match (left, right) {
            (ResolvedValue::Boolean(bool1), ResolvedValue::Boolean(bool2)) => {
                self.enforce_boolean_eq(cs, bool1, bool2)
            }
            (ResolvedValue::U32(num1), ResolvedValue::U32(num2)) => {
                Self::enforce_u32_eq(cs, num1, num2)
            }
            (ResolvedValue::FieldElement(fe1), ResolvedValue::FieldElement(fe2)) => {
                self.enforce_field_eq(fe1, fe2)
            }
            (val1, val2) => unimplemented!("cannot enforce equality between {} == {}", val1, val2),
        }
    }

    /// Enforce array expressions
    fn enforce_array_expression(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function_scope: String,
        array: Vec<Box<SpreadOrExpression<F>>>,
    ) -> ResolvedValue<F> {
        let mut result = vec![];
        array.into_iter().for_each(|element| match *element {
            SpreadOrExpression::Spread(spread) => match spread {
                Expression::Variable(variable) => {
                    let array_name = new_scope_from_variable(function_scope.clone(), &variable);
                    match self.get(&array_name) {
                        Some(value) => match value {
                            ResolvedValue::Array(array) => result.extend(array.clone()),
                            value => {
                                unimplemented!("spreads only implemented for arrays, got {}", value)
                            }
                        },
                        None => unimplemented!(
                            "cannot copy elements from array that does not exist {}",
                            variable.name
                        ),
                    }
                }
                value => unimplemented!("spreads only implemented for arrays, got {}", value),
            },
            SpreadOrExpression::Expression(expression) => {
                result.push(self.enforce_expression(
                    cs,
                    file_scope.clone(),
                    function_scope.clone(),
                    expression,
                ));
            }
        });
        ResolvedValue::Array(result)
    }

    pub(crate) fn enforce_index(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function_scope: String,
        index: Expression<F>,
    ) -> usize {
        match self.enforce_expression(cs, file_scope, function_scope, index) {
            ResolvedValue::U32(number) => number.value.unwrap() as usize,
            value => unimplemented!("From index must resolve to an integer, got {}", value),
        }
    }

    fn enforce_array_access_expression(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function_scope: String,
        array: Box<Expression<F>>,
        index: RangeOrExpression<F>,
    ) -> ResolvedValue<F> {
        match self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *array) {
            ResolvedValue::Array(array) => {
                match index {
                    RangeOrExpression::Range(from, to) => {
                        let from_resolved = match from {
                            Some(from_index) => from_index.to_usize(),
                            None => 0usize, // Array slice starts at index 0
                        };
                        let to_resolved = match to {
                            Some(to_index) => to_index.to_usize(),
                            None => array.len(), // Array slice ends at array length
                        };
                        ResolvedValue::Array(array[from_resolved..to_resolved].to_owned())
                    }
                    RangeOrExpression::Expression(index) => {
                        let index_resolved =
                            self.enforce_index(cs, file_scope, function_scope, index);
                        array[index_resolved].to_owned()
                    }
                }
            }
            value => unimplemented!("Cannot access element of untyped array {}", value),
        }
    }

    fn enforce_struct_expression(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function_scope: String,
        variable: Variable<F>,
        members: Vec<StructMember<F>>,
    ) -> ResolvedValue<F> {
        let struct_name = new_variable_from_variable(file_scope.clone(), &variable);

        if let Some(resolved_value) = self.get_mut_variable(&struct_name) {
            match resolved_value {
                ResolvedValue::StructDefinition(struct_definition) => {
                    let resolved_members = struct_definition
                        .fields
                        .clone()
                        .iter()
                        .zip(members.clone().into_iter())
                        .map(|(field, member)| {
                            if field.variable != member.variable {
                                unimplemented!("struct field variables do not match")
                            }
                            // Resolve and enforce struct fields
                            let member_value = self.enforce_expression(
                                cs,
                                file_scope.clone(),
                                function_scope.clone(),
                                member.expression,
                            );

                            ResolvedStructMember(member.variable, member_value)
                        })
                        .collect();

                    ResolvedValue::StructExpression(variable, resolved_members)
                }
                _ => unimplemented!("Inline struct type is not defined as a struct"),
            }
        } else {
            unimplemented!(
                "Struct {} must be declared before it is used in an inline expression",
                struct_name
            )
        }
    }

    fn enforce_struct_access_expression(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function_scope: String,
        struct_variable: Box<Expression<F>>,
        struct_member: Variable<F>,
    ) -> ResolvedValue<F> {
        match self.enforce_expression(cs, file_scope, function_scope, *struct_variable) {
            ResolvedValue::StructExpression(_name, members) => {
                let matched_member = members.into_iter().find(|member| member.0 == struct_member);
                match matched_member {
                    Some(member) => member.1,
                    None => unimplemented!("Cannot access struct member {}", struct_member.name),
                }
            }
            value => unimplemented!("Cannot access element of untyped struct {}", value),
        }
    }

    fn enforce_function_call_expression(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function: Variable<F>,
        arguments: Vec<Expression<F>>,
    ) -> ResolvedValue<F> {
        let function_name = new_variable_from_variable(file_scope.clone(), &function);
        match self.get_mut_variable(&function_name) {
            Some(value) => match value.clone() {
                ResolvedValue::Function(function) => {
                    // this function call is inline so we unwrap the return value
                    match self.enforce_function(cs, file_scope, function, arguments) {
                        ResolvedValue::Return(return_values) => {
                            if return_values.len() == 1 {
                                return_values[0].clone()
                            } else {
                                ResolvedValue::Return(return_values)
                            }
                        }
                        value => unimplemented!(
                            "function {} has no return value {}",
                            function_name,
                            value
                        ),
                    }
                }
                value => unimplemented!("Cannot make function call to {}", value),
            },
            None => unimplemented!("Cannot call unknown function {}", function_name),
        }
    }

    pub(crate) fn enforce_expression(
        &mut self,
        cs: &mut CS,
        file_scope: String,
        function_scope: String,
        expression: Expression<F>,
    ) -> ResolvedValue<F> {
        match expression {
            // Variables
            Expression::Variable(unresolved_variable) => {
                self.enforce_variable(function_scope, unresolved_variable)
            }

            // Values
            Expression::Integer(integer) => Self::get_integer_constant(integer),
            Expression::FieldElement(fe) => ResolvedValue::FieldElement(fe),
            Expression::Boolean(bool) => Self::get_boolean_constant(bool),

            // Binary operations
            Expression::Add(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_add_expression(cs, resolved_left, resolved_right)
            }
            Expression::Sub(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_sub_expression(cs, resolved_left, resolved_right)
            }
            Expression::Mul(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_mul_expression(cs, resolved_left, resolved_right)
            }
            Expression::Div(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_div_expression(cs, resolved_left, resolved_right)
            }
            Expression::Pow(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_pow_expression(cs, resolved_left, resolved_right)
            }

            // Boolean operations
            Expression::Not(expression) => Self::enforce_not(self.enforce_expression(
                cs,
                file_scope,
                function_scope,
                *expression,
            )),
            Expression::Or(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_or(cs, resolved_left, resolved_right)
            }
            Expression::And(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.enforce_and(cs, resolved_left, resolved_right)
            }
            Expression::Eq(left, right) => {
                let resolved_left =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *left);
                let resolved_right =
                    self.enforce_expression(cs, file_scope.clone(), function_scope.clone(), *right);

                self.evaluate_eq_expression(resolved_left, resolved_right)
            }
            Expression::Geq(left, right) => {
                unimplemented!("expression {} >= {} unimplemented", left, right)
            }
            Expression::Gt(left, right) => {
                unimplemented!("expression {} > {} unimplemented", left, right)
            }
            Expression::Leq(left, right) => {
                unimplemented!("expression {} <= {} unimplemented", left, right)
            }
            Expression::Lt(left, right) => {
                unimplemented!("expression {} < {} unimplemented", left, right)
            }

            // Conditionals
            Expression::IfElse(first, second, third) => {
                let resolved_first = match self.enforce_expression(
                    cs,
                    file_scope.clone(),
                    function_scope.clone(),
                    *first,
                ) {
                    ResolvedValue::Boolean(resolved) => resolved,
                    _ => unimplemented!("if else conditional must resolve to boolean"),
                };

                if resolved_first.eq(&Boolean::Constant(true)) {
                    self.enforce_expression(cs, file_scope, function_scope, *second)
                } else {
                    self.enforce_expression(cs, file_scope, function_scope, *third)
                }
            }

            // Arrays
            Expression::Array(array) => {
                self.enforce_array_expression(cs, file_scope, function_scope, array)
            }
            Expression::ArrayAccess(array, index) => {
                self.enforce_array_access_expression(cs, file_scope, function_scope, array, *index)
            }

            // Structs
            Expression::Struct(struct_name, members) => {
                self.enforce_struct_expression(cs, file_scope, function_scope, struct_name, members)
            }
            Expression::StructMemberAccess(struct_variable, struct_member) => self
                .enforce_struct_access_expression(
                    cs,
                    file_scope,
                    function_scope,
                    struct_variable,
                    struct_member,
                ),

            // Functions
            Expression::FunctionCall(function, arguments) => {
                self.enforce_function_call_expression(cs, file_scope, function, arguments)
            } // _ => unimplemented!(),
        }
    }
}
