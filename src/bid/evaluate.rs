use regex::Regex;
use serde_json::Value;

use crate::bid::{Bid, BinaryOperator, Expression, UnaryOperator};

/// Errors that can occur during bid evaluation
#[derive(Debug, Clone)]
pub enum EvaluationError {
    /// Variable path not found in the JSON data
    VariableNotFound {
        /// The variable path that was not found
        path: Vec<String>,
    },
    /// Type mismatch during operation
    TypeMismatch {
        /// Description of the type mismatch
        message: String,
    },
    /// Division by zero
    DivisionByZero,
    /// Invalid operation
    InvalidOperation {
        /// Description of the invalid operation
        message: String,
    },
    /// Regex compilation error
    RegexError {
        /// The regex pattern that failed to compile
        pattern: String,
        /// The regex error message
        error: String,
    },
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvaluationError::VariableNotFound { path } => {
                write!(f, "Variable not found: {}", path.join("."))
            }
            EvaluationError::TypeMismatch { message } => {
                write!(f, "Type mismatch: {}", message)
            }
            EvaluationError::DivisionByZero => {
                write!(f, "Division by zero")
            }
            EvaluationError::InvalidOperation { message } => {
                write!(f, "Invalid operation: {}", message)
            }
            EvaluationError::RegexError { pattern, error } => {
                write!(f, "Regex error for pattern {pattern:?}: {error}")
            }
        }
    }
}

impl std::error::Error for EvaluationError {}

impl Bid {
    /// Evaluate the bid against the given JSON data
    /// Returns Some(bid_value) if the condition is met, None otherwise
    pub fn evaluate(&self, data: &Value) -> Result<Option<Value>, EvaluationError> {
        let condition_result = evaluate_expression(&self.on_condition, data)?;

        // Check if condition evaluates to true
        if is_truthy(&condition_result) {
            let bid_result = evaluate_expression(&self.bid_value, data)?;
            Ok(Some(bid_result))
        } else {
            Ok(None)
        }
    }
}

/// Evaluate an expression against the given JSON data
fn evaluate_expression(expr: &Expression, data: &Value) -> Result<Value, EvaluationError> {
    match expr {
        Expression::Variable { path, .. } => resolve_variable_path(data, path),
        Expression::StringLiteral { value, .. } => Ok(Value::String(value.clone())),
        Expression::IntegerLiteral { value, .. } => {
            Ok(Value::Number(serde_json::Number::from(*value)))
        }
        Expression::FloatLiteral { value, .. } => {
            if let Some(num) = serde_json::Number::from_f64(*value) {
                Ok(Value::Number(num))
            } else {
                Err(EvaluationError::InvalidOperation {
                    message: format!("Invalid float value: {}", value),
                })
            }
        }
        Expression::BooleanLiteral { value, .. } => Ok(Value::Bool(*value)),
        Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } => {
            let left_val = evaluate_expression(left, data)?;
            let right_val = evaluate_expression(right, data)?;
            evaluate_binary_operation(&left_val, operator, &right_val)
        }
        Expression::UnaryOperation {
            operator, operand, ..
        } => {
            let operand_val = evaluate_expression(operand, data)?;
            evaluate_unary_operation(operator, &operand_val)
        }
    }
}

/// Resolve a variable path in the JSON data
fn resolve_variable_path(data: &Value, path: &[String]) -> Result<Value, EvaluationError> {
    let mut current = data;

    for segment in path {
        match current {
            Value::Object(map) => {
                current = map
                    .get(segment)
                    .ok_or_else(|| EvaluationError::VariableNotFound {
                        path: path.to_vec(),
                    })?;
            }
            _ => {
                return Err(EvaluationError::VariableNotFound {
                    path: path.to_vec(),
                });
            }
        }
    }

    Ok(current.clone())
}

/// Evaluate a binary operation between two JSON values
fn evaluate_binary_operation(
    left: &Value,
    operator: &BinaryOperator,
    right: &Value,
) -> Result<Value, EvaluationError> {
    match operator {
        // Arithmetic operators
        BinaryOperator::Add => add_values(left, right),
        BinaryOperator::Subtract => subtract_values(left, right),
        BinaryOperator::Multiply => multiply_values(left, right),
        BinaryOperator::Divide => divide_values(left, right),
        BinaryOperator::Modulo => modulo_values(left, right),
        BinaryOperator::Power => power_values(left, right),

        // Comparison operators
        BinaryOperator::Equal => Ok(Value::Bool(values_equal(left, right))),
        BinaryOperator::NotEqual => Ok(Value::Bool(!values_equal(left, right))),
        BinaryOperator::LessThan => compare_values(left, right, |cmp| cmp < 0),
        BinaryOperator::LessThanOrEqual => compare_values(left, right, |cmp| cmp <= 0),
        BinaryOperator::GreaterThan => compare_values(left, right, |cmp| cmp > 0),
        BinaryOperator::GreaterThanOrEqual => compare_values(left, right, |cmp| cmp >= 0),

        // Logical operators
        BinaryOperator::LogicalAnd => {
            if is_truthy(left) {
                Ok(right.clone())
            } else {
                Ok(left.clone())
            }
        }
        BinaryOperator::LogicalOr => {
            if is_truthy(left) {
                Ok(left.clone())
            } else {
                Ok(right.clone())
            }
        }

        // Regex operators
        BinaryOperator::RegexMatch => regex_match_values(left, right),
    }
}

/// Evaluate a unary operation on a JSON value
fn evaluate_unary_operation(
    operator: &UnaryOperator,
    operand: &Value,
) -> Result<Value, EvaluationError> {
    match operator {
        UnaryOperator::Negate => negate_value(operand),
        UnaryOperator::LogicalNot => Ok(Value::Bool(!is_truthy(operand))),
    }
}

/// Check if a JSON value is truthy
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
    }
}

/// Check if two JSON values are equal
fn values_equal(left: &Value, right: &Value) -> bool {
    left == right
}

/// Compare two JSON values numerically
fn compare_values<F>(left: &Value, right: &Value, compare: F) -> Result<Value, EvaluationError>
where
    F: Fn(i8) -> bool,
{
    let left_num = extract_number(left)?;
    let right_num = extract_number(right)?;

    let result = if left_num < right_num {
        -1
    } else if left_num > right_num {
        1
    } else {
        0
    };

    Ok(Value::Bool(compare(result)))
}

/// Extract a numeric value from a JSON value
fn extract_number(value: &Value) -> Result<f64, EvaluationError> {
    match value {
        Value::Number(n) => n.as_f64().ok_or_else(|| EvaluationError::TypeMismatch {
            message: "Invalid number".to_string(),
        }),
        _ => Err(EvaluationError::TypeMismatch {
            message: format!("Expected number, found {}", type_name(value)),
        }),
    }
}

/// Check if a JSON value represents an integer
fn is_integer(value: &Value) -> Result<bool, EvaluationError> {
    match value {
        Value::Number(n) => Ok(n.is_i64() || n.is_u64()),
        _ => Err(EvaluationError::TypeMismatch {
            message: format!("Expected number, found {}", type_name(value)),
        }),
    }
}

/// Check if both JSON values represent integers
fn are_both_integers(left: &Value, right: &Value) -> Result<bool, EvaluationError> {
    Ok(is_integer(left)? && is_integer(right)?)
}

/// Check for division by zero
fn check_division_by_zero(divisor: f64) -> Result<(), EvaluationError> {
    if divisor == 0.0 {
        Err(EvaluationError::DivisionByZero)
    } else {
        Ok(())
    }
}

/// Perform a generic arithmetic operation between two numeric JSON values
fn perform_arithmetic_operation<F>(
    left: &Value,
    right: &Value,
    op: F,
    op_name: &str,
) -> Result<Value, EvaluationError>
where
    F: Fn(f64, f64) -> f64,
{
    let l_val = extract_number(left)?;
    let r_val = extract_number(right)?;
    let result = op(l_val, r_val);

    if let Some(num) = serde_json::Number::from_f64(result) {
        Ok(Value::Number(num))
    } else {
        Err(EvaluationError::InvalidOperation {
            message: format!("{} result overflow: {}", op_name, result),
        })
    }
}

fn perform_arithmetic_operation_with_options<F>(
    left: &Value,
    right: &Value,
    op: F,
    op_name: &str,
    check_div_by_zero: bool,
    preserve_integer_types: bool,
) -> Result<Value, EvaluationError>
where
    F: Fn(f64, f64) -> f64,
{
    let l_val = extract_number(left)?;
    let r_val = extract_number(right)?;

    if check_div_by_zero {
        check_division_by_zero(r_val)?;
    }

    let result = op(l_val, r_val);

    if preserve_integer_types
        && are_both_integers(left, right)?
        && result.fract().abs() <= f64::EPSILON
    {
        Ok(Value::Number(serde_json::Number::from(result as i64)))
    } else if let Some(num) = serde_json::Number::from_f64(result) {
        Ok(Value::Number(num))
    } else {
        Err(EvaluationError::InvalidOperation {
            message: format!("{} result overflow: {}", op_name, result),
        })
    }
}

fn perform_unary_operation<F>(
    value: &Value,
    op: F,
    op_name: &str,
    preserve_integer_types: bool,
) -> Result<Value, EvaluationError>
where
    F: Fn(f64) -> f64,
{
    let num_val = extract_number(value)?;
    let result = op(num_val);

    if preserve_integer_types && is_integer(value)? && result.fract().abs() <= f64::EPSILON {
        Ok(Value::Number(serde_json::Number::from(result as i64)))
    } else if let Some(num) = serde_json::Number::from_f64(result) {
        Ok(Value::Number(num))
    } else {
        Err(EvaluationError::InvalidOperation {
            message: format!("{} result overflow: {}", op_name, result),
        })
    }
}

/// Add two JSON values
fn add_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => {
            let l_val = l.as_f64().ok_or_else(|| EvaluationError::TypeMismatch {
                message: "Invalid left number in addition".to_string(),
            })?;
            let r_val = r.as_f64().ok_or_else(|| EvaluationError::TypeMismatch {
                message: "Invalid right number in addition".to_string(),
            })?;
            let result = l_val + r_val;
            if let Some(num) = serde_json::Number::from_f64(result) {
                Ok(Value::Number(num))
            } else {
                Err(EvaluationError::InvalidOperation {
                    message: format!("Addition result overflow: {}", result),
                })
            }
        }
        (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
        _ => Err(EvaluationError::TypeMismatch {
            message: format!("Cannot add {} and {}", type_name(left), type_name(right)),
        }),
    }
}

/// Subtract two numeric JSON values
fn subtract_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    perform_arithmetic_operation(left, right, |lhs, rhs| lhs - rhs, "Subtraction")
}

/// Multiply two numeric JSON values
fn multiply_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    perform_arithmetic_operation(left, right, |lhs, rhs| lhs * rhs, "Multiplication")
}

/// Divide two numeric JSON values
fn divide_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    perform_arithmetic_operation_with_options(
        left,
        right,
        |lhs, rhs| lhs / rhs,
        "Division",
        true,
        false,
    )
}

/// Calculate modulo of two numeric JSON values
fn modulo_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    perform_arithmetic_operation_with_options(
        left,
        right,
        |lhs, rhs| lhs % rhs,
        "Modulo",
        true,
        true,
    )
}

/// Calculate power of two numeric JSON values
fn power_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    let l_val = extract_number(left)?;
    let r_val = extract_number(right)?;
    let result = l_val.powf(r_val);

    if result.is_finite() {
        if let Some(num) = serde_json::Number::from_f64(result) {
            Ok(Value::Number(num))
        } else {
            Err(EvaluationError::InvalidOperation {
                message: format!("Power result overflow: {}", result),
            })
        }
    } else {
        Err(EvaluationError::InvalidOperation {
            message: format!(
                "Power operation resulted in infinite or NaN: {}^{}",
                l_val, r_val
            ),
        })
    }
}

/// Negate a numeric JSON value
fn negate_value(value: &Value) -> Result<Value, EvaluationError> {
    perform_unary_operation(value, |val| -val, "Negation", true)
}

/// Get a string representation of a JSON value's type
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Perform regex match operation
/// Left operand (haystack) should be a string, right operand (pattern) should be a string
fn regex_match_values(left: &Value, right: &Value) -> Result<Value, EvaluationError> {
    let haystack = match left {
        Value::String(s) => s,
        _ => {
            return Err(EvaluationError::TypeMismatch {
                message: format!(
                    "Regex match left operand must be a string, found {}",
                    type_name(left)
                ),
            });
        }
    };

    let pattern = match right {
        Value::String(s) => s,
        _ => {
            return Err(EvaluationError::TypeMismatch {
                message: format!(
                    "Regex match right operand must be a string, found {}",
                    type_name(right)
                ),
            });
        }
    };

    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            return Err(EvaluationError::RegexError {
                pattern: pattern.clone(),
                error: e.to_string(),
            });
        }
    };

    Ok(Value::Bool(regex.is_match(haystack)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BidParser;
    use serde_json::json;

    #[test]
    fn simple_condition_evaluation() {
        let bid = BidParser::parse("ON user.active BID user.score").unwrap();
        let data = json!({
            "user": {
                "active": true,
                "score": 100
            }
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(100)));
    }

    #[test]
    fn false_condition_evaluation() {
        let bid = BidParser::parse("ON user.active BID user.score").unwrap();
        let data = json!({
            "user": {
                "active": false,
                "score": 100
            }
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn arithmetic_evaluation() {
        let bid = BidParser::parse("ON true BID price * 0.9").unwrap();
        let data = json!({
            "price": 100.0
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(90.0)));
    }

    #[test]
    fn comparison_evaluation() {
        let bid = BidParser::parse("ON price > 50.0 BID discount").unwrap();
        let data = json!({
            "price": 75.0,
            "discount": 10.0
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(10.0)));

        let data_low_price = json!({
            "price": 25.0,
            "discount": 10.0
        });

        let result_low = bid.evaluate(&data_low_price).unwrap();
        assert_eq!(result_low, None);
    }

    #[test]
    fn string_concatenation() {
        let bid = BidParser::parse(r#"ON true BID prefix + suffix"#).unwrap();
        let data = json!({
            "prefix": "Hello ",
            "suffix": "World"
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!("Hello World")));
    }

    #[test]
    fn logical_operations() {
        let bid = BidParser::parse("ON active && premium BID bonus").unwrap();
        let data = json!({
            "active": true,
            "premium": true,
            "bonus": 50
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(50)));

        let data_partial = json!({
            "active": true,
            "premium": false,
            "bonus": 50
        });

        let result_partial = bid.evaluate(&data_partial).unwrap();
        assert_eq!(result_partial, None);
    }

    #[test]
    fn regex_match_operation() {
        let bid = BidParser::parse(r#"ON text ~= "^hello.*world$" BID score"#).unwrap();
        let data = json!({
            "text": "hello beautiful world",
            "score": 100
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(100)));

        let data_no_match = json!({
            "text": "goodbye cruel world",
            "score": 50
        });

        let result_no_match = bid.evaluate(&data_no_match).unwrap();
        assert_eq!(result_no_match, None);
    }

    #[test]
    fn regex_match_simple_pattern() {
        let bid = BidParser::parse(r#"ON email ~= "@.*\.com$" BID valid"#).unwrap();
        let data = json!({
            "email": "user@example.com",
            "valid": true
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(true)));
    }

    #[test]
    fn regex_match_type_error_left() {
        let bid = BidParser::parse(r#"ON 123 ~= "pattern" BID result"#).unwrap();
        let data = json!({"result": 1});

        let result = bid.evaluate(&data);
        assert!(matches!(result, Err(EvaluationError::TypeMismatch { .. })));
    }

    #[test]
    fn regex_match_type_error_right() {
        let bid = BidParser::parse(r#"ON "text" ~= 456 BID result"#).unwrap();
        let data = json!({"result": 1});

        let result = bid.evaluate(&data);
        assert!(matches!(result, Err(EvaluationError::TypeMismatch { .. })));
    }

    #[test]
    fn regex_match_invalid_pattern() {
        let bid = BidParser::parse(r#"ON text ~= "[invalid" BID result"#).unwrap();
        let data = json!({
            "text": "some text",
            "result": 1
        });

        let result = bid.evaluate(&data);
        assert!(matches!(result, Err(EvaluationError::RegexError { .. })));
    }

    #[test]
    fn unary_operations() {
        let bid = BidParser::parse("ON !disabled BID -penalty").unwrap();
        let data = json!({
            "disabled": false,
            "penalty": 10
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(-10)));
    }

    #[test]
    fn complex_expression() {
        let bid = BidParser::parse(
            r#"ON (user.tier == "premium" && price > 100.0) BID price * discount + bonus"#,
        )
        .unwrap();
        let data = json!({
            "user": {
                "tier": "premium"
            },
            "price": 150.0,
            "discount": 0.8,
            "bonus": 20.0
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(140.0))); // 150 * 0.8 + 20
    }

    #[test]
    fn variable_not_found() {
        let bid = BidParser::parse("ON user.missing BID 100").unwrap();
        let data = json!({
            "user": {}
        });

        let result = bid.evaluate(&data);
        assert!(matches!(
            result,
            Err(EvaluationError::VariableNotFound { .. })
        ));
    }

    #[test]
    fn type_mismatch_arithmetic() {
        let bid = BidParser::parse(r#"ON true BID "text" * 5"#).unwrap();
        let data = json!({});

        let result = bid.evaluate(&data);
        assert!(matches!(result, Err(EvaluationError::TypeMismatch { .. })));
    }

    #[test]
    fn division_by_zero() {
        let bid = BidParser::parse("ON true BID 10 / 0").unwrap();
        let data = json!({});

        let result = bid.evaluate(&data);
        assert!(matches!(result, Err(EvaluationError::DivisionByZero)));
    }

    #[test]
    fn power_operation() {
        let bid = BidParser::parse("ON true BID base ^ exponent").unwrap();
        let data = json!({
            "base": 2.0,
            "exponent": 3.0
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(8.0)));
    }

    #[test]
    fn truthiness_evaluation() {
        let test_cases = vec![
            (json!(null), false),
            (json!(false), false),
            (json!(true), true),
            (json!(0), false),
            (json!(1), true),
            (json!(-1), true),
            (json!(""), false),
            (json!("hello"), true),
            (json!([]), false),
            (json!([1, 2, 3]), true),
            (json!({}), false),
            (json!({"key": "value"}), true),
        ];

        for (value, expected) in test_cases {
            assert_eq!(
                is_truthy(&value),
                expected,
                "Truthiness failed for {:?}",
                value
            );
        }
    }

    #[test]
    fn deep_nested_path() {
        let bid =
            BidParser::parse("ON data.user.profile.settings.active BID data.user.profile.score")
                .unwrap();
        let data = json!({
            "data": {
                "user": {
                    "profile": {
                        "settings": {
                            "active": true
                        },
                        "score": 95
                    }
                }
            }
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(95)));
    }

    #[test]
    fn equality_comparison() {
        let bid = BidParser::parse(r#"ON category == "electronics" BID discount"#).unwrap();
        let data = json!({
            "category": "electronics",
            "discount": 15.0
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(15.0)));
    }

    #[test]
    fn inequality_comparison() {
        let bid = BidParser::parse(r#"ON category != "excluded" BID price"#).unwrap();
        let data = json!({
            "category": "books",
            "price": 25.0
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(25.0)));
    }

    #[test]
    fn modulo_operation() {
        let bid = BidParser::parse("ON true BID value % divisor").unwrap();
        let data = json!({
            "value": 17,
            "divisor": 5
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(2)));
    }

    #[test]
    fn logical_or_short_circuit() {
        let bid = BidParser::parse("ON first_condition || second_condition BID result").unwrap();
        let data = json!({
            "first_condition": true,
            "second_condition": "not_evaluated", // This won't be evaluated
            "result": 42
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, Some(json!(42)));
    }

    #[test]
    fn logical_and_short_circuit() {
        let bid = BidParser::parse("ON first_condition && second_condition BID result").unwrap();
        let data = json!({
            "first_condition": false,
            "second_condition": "not_evaluated", // This won't be evaluated
            "result": 42
        });

        let result = bid.evaluate(&data).unwrap();
        assert_eq!(result, None);
    }
}
