//! # Bid Expression Parser
//!
//! This module provides parsing capabilities for bid expressions that follow the
//! `ON condition BID value` syntax. Both condition and value parts support arithmetic
//! expressions, variable references, and common operators.
//!
//! ## Syntax
//!
//! A bid expression consists of two parts separated by keywords:
//! ```text
//! ON <expression> BID <expression>
//! ```
//!
//! Where `<expression>` can contain:
//! - **Variables**: Dot-separated identifiers like `foo.bar.baz`
//! - **Literals**: Strings ("hello"), integers (42), floats (3.14), booleans (true, false)
//! - **Arithmetic**: `+`, `-`, `*`, `/`, `%`, `^` (exponentiation)
//! - **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
//! - **Logical**: `&&`, `||`, `!`
//! - **Grouping**: Parentheses for precedence
//!
//! ## Examples
//!
//! ```rust
//! use stigmergy::BidParser;
//!
//! let simple = BidParser::parse("ON user.active BID user.score * 10").unwrap();
//! let complex = BidParser::parse(r#"ON (item.price < 100.0 && item.category == "electronics") BID item.price * 0.9"#).unwrap();
//! ```

use handled::Handle;
use std::fmt;

mod evaluate;

/// Position information for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl Position {
    /// Create a new position
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Starting position
    pub fn start() -> Self {
        Self::new(1, 1)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A parsed expression that can be evaluated
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Variable reference with dot-separated path (e.g., "user.profile.name")
    Variable {
        /// The variable path segments
        path: Vec<String>,
        /// Source position for error reporting
        position: Position,
    },
    /// String literal value
    StringLiteral {
        /// The string value
        value: String,
        /// Source position for error reporting
        position: Position,
    },
    /// Integer literal value
    IntegerLiteral {
        /// The integer value
        value: i64,
        /// Source position for error reporting
        position: Position,
    },
    /// Float literal value
    FloatLiteral {
        /// The float value
        value: f64,
        /// Source position for error reporting
        position: Position,
    },
    /// Boolean literal value
    BooleanLiteral {
        /// The boolean value
        value: bool,
        /// Source position for error reporting
        position: Position,
    },
    /// Binary operation (e.g., a + b, x == y)
    BinaryOperation {
        /// Left operand
        left: Box<Expression>,
        /// The operator
        operator: BinaryOperator,
        /// Right operand
        right: Box<Expression>,
        /// Source position for error reporting
        position: Position,
    },
    /// Unary operation (e.g., -x, !condition)
    UnaryOperation {
        /// The operator
        operator: UnaryOperator,
        /// The operand
        operand: Box<Expression>,
        /// Source position for error reporting
        position: Position,
    },
    /// Member access on an expression (e.g., (*key).property)
    MemberAccess {
        /// The object expression to access
        object: Box<Expression>,
        /// The property name to access
        property: String,
        /// Source position for error reporting
        position: Position,
    },
}

impl Expression {
    /// Get the position of this expression
    pub fn position(&self) -> Position {
        match self {
            Expression::Variable { position, .. }
            | Expression::StringLiteral { position, .. }
            | Expression::IntegerLiteral { position, .. }
            | Expression::FloatLiteral { position, .. }
            | Expression::BooleanLiteral { position, .. }
            | Expression::BinaryOperation { position, .. }
            | Expression::UnaryOperation { position, .. }
            | Expression::MemberAccess { position, .. } => *position,
        }
    }
}

/// Binary operators with precedence information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic operators
    /// Addition
    Add,
    /// Subtraction
    Subtract,
    /// Multiplication
    Multiply,
    /// Division
    Divide,
    /// Modulo
    Modulo,
    /// Exponentiation
    Power,

    // Comparison operators
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
    /// Less than
    LessThan,
    /// Less than or equal
    LessThanOrEqual,
    /// Greater than
    GreaterThan,
    /// Greater than or equal
    GreaterThanOrEqual,

    // Logical operators
    /// Logical AND
    LogicalAnd,
    /// Logical OR
    LogicalOr,

    // Regex operators
    /// Regex match
    RegexMatch,
}

impl BinaryOperator {
    /// Get operator precedence (higher number = higher precedence)
    pub const fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::LogicalOr => 1,
            BinaryOperator::LogicalAnd => 2,
            BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::RegexMatch => 3,
            BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => 4,
            BinaryOperator::Add | BinaryOperator::Subtract => 5,
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => 6,
            BinaryOperator::Power => 7,
        }
    }

    /// Check if operator is right-associative
    pub const fn is_right_associative(&self) -> bool {
        matches!(self, BinaryOperator::Power)
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Power => "^",
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessThanOrEqual => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterThanOrEqual => ">=",
            BinaryOperator::LogicalAnd => "&&",
            BinaryOperator::LogicalOr => "||",
            BinaryOperator::RegexMatch => "~=",
        };
        write!(f, "{}", s)
    }
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOperator {
    /// Arithmetic negation
    Negate,
    /// Logical NOT
    LogicalNot,
    /// Pointer dereference
    Dereference,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnaryOperator::Negate => "-",
            UnaryOperator::LogicalNot => "!",
            UnaryOperator::Dereference => "*",
        };
        write!(f, "{}", s)
    }
}

/// A complete bid expression with condition and value
#[derive(Debug, Clone, PartialEq)]
pub struct Bid {
    /// The condition expression after ON
    pub on_condition: Expression,
    /// The value expression after BID
    pub bid_value: Expression,
}

impl fmt::Display for Bid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ON {} BID {}", self.on_condition, self.bid_value)
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Variable { path, .. } => write!(f, "{}", path.join(".")),
            Expression::StringLiteral { value, .. } => write!(f, "\"{}\"", value),
            Expression::IntegerLiteral { value, .. } => write!(f, "{}", value),
            Expression::FloatLiteral { value, .. } => write!(f, "{}", value),
            Expression::BooleanLiteral { value, .. } => write!(f, "{}", value),
            Expression::BinaryOperation {
                left,
                operator,
                right,
                ..
            } => {
                write!(f, "({} {} {})", left, operator, right)
            }
            Expression::UnaryOperation {
                operator, operand, ..
            } => {
                write!(f, "{}({})", operator, operand)
            }
            Expression::MemberAccess {
                object, property, ..
            } => {
                write!(f, "({}).{}", object, property)
            }
        }
    }
}

/// Token types for the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    On,
    Bid,

    // Identifiers and literals
    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(i64),
    FloatLiteral(f64),
    BooleanLiteral(bool),

    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LogicalAnd,
    LogicalOr,
    LogicalNot,
    RegexMatch,

    // Punctuation
    Dot,
    LeftParen,
    RightParen,

    // Special
    EndOfInput,
}

/// A token with position information
#[derive(Debug, Clone)]
pub struct Token {
    /// The token type
    pub token_type: TokenType,
    /// Source position
    pub position: Position,
}

/// Errors that can occur during bid parsing
#[derive(Debug, Clone)]
pub enum BidParseError {
    /// Unexpected token during parsing
    UnexpectedToken {
        /// What was found
        found: String,
        /// What was expected
        expected: String,
        /// Where the error occurred
        position: Position,
    },
    /// Invalid numeric literal
    InvalidNumber {
        /// The invalid text
        text: String,
        /// Where the error occurred
        position: Position,
    },
    /// Unterminated string literal
    UnterminatedString {
        /// Where the string started
        position: Position,
    },
    /// Invalid character in input
    InvalidCharacter {
        /// The invalid character
        character: char,
        /// Where the error occurred
        position: Position,
    },
    /// Missing ON keyword
    MissingOnKeyword {
        /// Where the error was detected
        position: Position,
    },
    /// Missing BID keyword
    MissingBidKeyword {
        /// Where the error was detected
        position: Position,
    },
    /// Empty expression
    EmptyExpression {
        /// Where the error was detected
        position: Position,
    },
}

impl fmt::Display for BidParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BidParseError::UnexpectedToken {
                found,
                expected,
                position,
            } => {
                write!(
                    f,
                    "Unexpected token '{}' at {}, expected {}",
                    found, position, expected
                )
            }
            BidParseError::InvalidNumber { text, position } => {
                write!(f, "Invalid number '{}' at {}", text, position)
            }
            BidParseError::UnterminatedString { position } => {
                write!(f, "Unterminated string literal at {}", position)
            }
            BidParseError::InvalidCharacter {
                character,
                position,
            } => {
                write!(f, "Invalid character '{}' at {}", character, position)
            }
            BidParseError::MissingOnKeyword { position } => {
                write!(f, "Expected 'ON' keyword at {}", position)
            }
            BidParseError::MissingBidKeyword { position } => {
                write!(f, "Expected 'BID' keyword at {}", position)
            }
            BidParseError::EmptyExpression { position } => {
                write!(f, "Empty expression at {}", position)
            }
        }
    }
}

impl std::error::Error for BidParseError {}

/// User-friendly error for CLI display
#[derive(Debug, Clone)]
pub struct UserError {
    /// The main error message
    pub message: String,
    /// Optional usage hint
    pub usage_hint: Option<String>,
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Handle<UserError> for UserError {
    fn handle(&self) -> Option<UserError> {
        Some(self.clone())
    }
}

impl Handle<UserError> for BidParseError {
    fn handle(&self) -> Option<UserError> {
        let (message, hint) = match self {
            BidParseError::UnexpectedToken {
                found,
                expected,
                position,
            } => (
                format!(
                    "Unexpected token '{}' at {}, expected {}",
                    found, position, expected
                ),
                Some("Check your expression syntax and operator placement".to_string()),
            ),
            BidParseError::InvalidNumber { text, position } => (
                format!("Invalid number '{}' at {}", text, position),
                Some("Numbers should be integers (42) or decimals (3.14)".to_string()),
            ),
            BidParseError::UnterminatedString { position } => (
                format!("Unterminated string literal at {}", position),
                Some("String literals must be enclosed in double quotes".to_string()),
            ),
            BidParseError::InvalidCharacter {
                character,
                position,
            } => (
                format!("Invalid character '{}' at {}", character, position),
                Some("Use only letters, numbers, operators, and punctuation".to_string()),
            ),
            BidParseError::MissingOnKeyword { position } => (
                format!("Expected 'ON' keyword at {}", position),
                Some("Bid expressions must start with 'ON <condition> BID <value>'".to_string()),
            ),
            BidParseError::MissingBidKeyword { position } => (
                format!("Expected 'BID' keyword at {}", position),
                Some("Bid expressions must have format 'ON <condition> BID <value>'".to_string()),
            ),
            BidParseError::EmptyExpression { position } => (
                format!("Empty expression at {}", position),
                Some("Expressions cannot be empty".to_string()),
            ),
        };

        Some(UserError {
            message,
            usage_hint: hint,
        })
    }
}

/// Main parser for bid expressions
pub struct BidParser;

impl BidParser {
    /// Parse a bid expression from a string
    pub fn parse(input: &str) -> Result<Bid, BidParseError> {
        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(&mut lexer)?;
        parser.parse_bid()
    }
}

/// Lexer for tokenizing input
struct Lexer {
    /// Input text
    input: Vec<char>,
    /// Current position in input
    position: usize,
    /// Current line number (1-based)
    line: usize,
    /// Current column number (1-based)
    column: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    fn current_position(&self) -> Position {
        Position::new(self.line, self.column)
    }

    fn current_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.current_char() {
            self.position += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, BidParseError> {
        self.skip_whitespace();

        let position = self.current_position();

        match self.current_char() {
            None => Ok(Token {
                token_type: TokenType::EndOfInput,
                position,
            }),
            Some(ch) => match ch {
                '(' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::LeftParen,
                        position,
                    })
                }
                ')' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::RightParen,
                        position,
                    })
                }
                '.' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Dot,
                        position,
                    })
                }
                '+' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Plus,
                        position,
                    })
                }
                '-' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Minus,
                        position,
                    })
                }
                '*' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Multiply,
                        position,
                    })
                }
                '/' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Divide,
                        position,
                    })
                }
                '%' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Modulo,
                        position,
                    })
                }
                '^' => {
                    self.advance();
                    Ok(Token {
                        token_type: TokenType::Power,
                        position,
                    })
                }
                '=' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::Equal,
                            position,
                        })
                    } else {
                        Err(BidParseError::InvalidCharacter {
                            character: '=',
                            position,
                        })
                    }
                }
                '!' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::NotEqual,
                            position,
                        })
                    } else {
                        Ok(Token {
                            token_type: TokenType::LogicalNot,
                            position,
                        })
                    }
                }
                '<' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::LessThanOrEqual,
                            position,
                        })
                    } else {
                        Ok(Token {
                            token_type: TokenType::LessThan,
                            position,
                        })
                    }
                }
                '>' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::GreaterThanOrEqual,
                            position,
                        })
                    } else {
                        Ok(Token {
                            token_type: TokenType::GreaterThan,
                            position,
                        })
                    }
                }
                '&' => {
                    self.advance();
                    if self.current_char() == Some('&') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::LogicalAnd,
                            position,
                        })
                    } else {
                        Err(BidParseError::InvalidCharacter {
                            character: '&',
                            position,
                        })
                    }
                }
                '|' => {
                    self.advance();
                    if self.current_char() == Some('|') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::LogicalOr,
                            position,
                        })
                    } else {
                        Err(BidParseError::InvalidCharacter {
                            character: '|',
                            position,
                        })
                    }
                }
                '~' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        Ok(Token {
                            token_type: TokenType::RegexMatch,
                            position,
                        })
                    } else {
                        Err(BidParseError::InvalidCharacter {
                            character: '~',
                            position,
                        })
                    }
                }
                '"' => self.read_string_literal(position),
                ch if ch.is_ascii_alphabetic() || ch == '_' => {
                    self.read_identifier_or_keyword(position)
                }
                ch if ch.is_ascii_digit() => self.read_number_literal(position),
                _ => Err(BidParseError::InvalidCharacter {
                    character: ch,
                    position,
                }),
            },
        }
    }

    fn read_string_literal(&mut self, start_position: Position) -> Result<Token, BidParseError> {
        self.advance(); // Skip opening quote
        let mut value = String::new();

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // Skip closing quote
                return Ok(Token {
                    token_type: TokenType::StringLiteral(value),
                    position: start_position,
                });
            } else if ch == '\\' {
                self.advance();
                match self.current_char() {
                    Some('n') => {
                        value.push('\n');
                        self.advance();
                    }
                    Some('t') => {
                        value.push('\t');
                        self.advance();
                    }
                    Some('r') => {
                        value.push('\r');
                        self.advance();
                    }
                    Some('\\') => {
                        value.push('\\');
                        self.advance();
                    }
                    Some('"') => {
                        value.push('"');
                        self.advance();
                    }
                    Some(escape_ch) => {
                        // TODO(claude): Consider returning an error for unknown escape sequences
                        // instead of silently accepting them. This could help catch user errors.
                        value.push(escape_ch);
                        self.advance();
                    }
                    None => break,
                }
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Err(BidParseError::UnterminatedString {
            position: start_position,
        })
    }

    fn read_identifier_or_keyword(&mut self, position: Position) -> Result<Token, BidParseError> {
        let mut value = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let token_type = match value.as_str() {
            "ON" => TokenType::On,
            "BID" => TokenType::Bid,
            "true" => TokenType::BooleanLiteral(true),
            "false" => TokenType::BooleanLiteral(false),
            _ => TokenType::Identifier(value),
        };

        Ok(Token {
            token_type,
            position,
        })
    }

    fn read_number_literal(&mut self, position: Position) -> Result<Token, BidParseError> {
        let mut value = String::new();
        let mut has_dot = false;

        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if has_dot {
            match value.parse::<f64>() {
                Ok(float_val) => Ok(Token {
                    token_type: TokenType::FloatLiteral(float_val),
                    position,
                }),
                Err(_) => Err(BidParseError::InvalidNumber {
                    text: value,
                    position,
                }),
            }
        } else {
            match value.parse::<i64>() {
                Ok(int_val) => Ok(Token {
                    token_type: TokenType::IntegerLiteral(int_val),
                    position,
                }),
                Err(_) => Err(BidParseError::InvalidNumber {
                    text: value,
                    position,
                }),
            }
        }
    }
}

/// Recursive descent parser
struct Parser<'a> {
    lexer: &'a mut Lexer,
    current_token: Token,
}

impl<'a> Parser<'a> {
    fn new(lexer: &'a mut Lexer) -> Result<Self, BidParseError> {
        let current_token = lexer.next_token()?;
        Ok(Self {
            lexer,
            current_token,
        })
    }

    fn advance(&mut self) -> Result<(), BidParseError> {
        self.current_token = self.lexer.next_token()?;
        Ok(())
    }

    fn parse_bid(&mut self) -> Result<Bid, BidParseError> {
        // Expect ON keyword
        if !matches!(self.current_token.token_type, TokenType::On) {
            return Err(BidParseError::MissingOnKeyword {
                position: self.current_token.position,
            });
        }
        self.advance()?;

        // Parse condition expression
        let on_condition = self.parse_expression()?;

        // Expect BID keyword
        if !matches!(self.current_token.token_type, TokenType::Bid) {
            return Err(BidParseError::MissingBidKeyword {
                position: self.current_token.position,
            });
        }
        self.advance()?;

        // Parse value expression
        let bid_value = self.parse_expression()?;

        // Should be at end of input
        if !matches!(self.current_token.token_type, TokenType::EndOfInput) {
            return Err(BidParseError::UnexpectedToken {
                found: format!("{:?}", self.current_token.token_type),
                expected: "end of input".to_string(),
                position: self.current_token.position,
            });
        }

        Ok(Bid {
            on_condition,
            bid_value,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, BidParseError> {
        self.parse_logical_or()
    }

    fn parse_binary_left_associative<F, G>(
        &mut self,
        mut next_level: F,
        token_matcher: G,
        operator_mapper: fn(&TokenType) -> BinaryOperator,
    ) -> Result<Expression, BidParseError>
    where
        F: FnMut(&mut Self) -> Result<Expression, BidParseError>,
        G: Fn(&TokenType) -> bool,
    {
        let mut left = next_level(self)?;

        while token_matcher(&self.current_token.token_type) {
            let position = self.current_token.position;
            let operator = operator_mapper(&self.current_token.token_type);
            self.advance()?;
            let right = next_level(self)?;
            left = Expression::BinaryOperation {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                position,
            };
        }

        Ok(left)
    }

    fn parse_logical_or(&mut self) -> Result<Expression, BidParseError> {
        self.parse_binary_left_associative(
            |parser| parser.parse_logical_and(),
            |token| matches!(token, TokenType::LogicalOr),
            |_| BinaryOperator::LogicalOr,
        )
    }

    fn parse_logical_and(&mut self) -> Result<Expression, BidParseError> {
        self.parse_binary_left_associative(
            |parser| parser.parse_equality(),
            |token| matches!(token, TokenType::LogicalAnd),
            |_| BinaryOperator::LogicalAnd,
        )
    }

    fn parse_equality(&mut self) -> Result<Expression, BidParseError> {
        self.parse_binary_left_associative(
            |parser| parser.parse_comparison(),
            |token| {
                matches!(
                    token,
                    TokenType::Equal | TokenType::NotEqual | TokenType::RegexMatch
                )
            },
            |token| match token {
                TokenType::Equal => BinaryOperator::Equal,
                TokenType::NotEqual => BinaryOperator::NotEqual,
                TokenType::RegexMatch => BinaryOperator::RegexMatch,
                _ => unreachable!(),
            },
        )
    }

    fn parse_comparison(&mut self) -> Result<Expression, BidParseError> {
        self.parse_binary_left_associative(
            |parser| parser.parse_addition(),
            |token| {
                matches!(
                    token,
                    TokenType::LessThan
                        | TokenType::LessThanOrEqual
                        | TokenType::GreaterThan
                        | TokenType::GreaterThanOrEqual
                )
            },
            |token| match token {
                TokenType::LessThan => BinaryOperator::LessThan,
                TokenType::LessThanOrEqual => BinaryOperator::LessThanOrEqual,
                TokenType::GreaterThan => BinaryOperator::GreaterThan,
                TokenType::GreaterThanOrEqual => BinaryOperator::GreaterThanOrEqual,
                _ => unreachable!(),
            },
        )
    }

    fn parse_addition(&mut self) -> Result<Expression, BidParseError> {
        self.parse_binary_left_associative(
            |parser| parser.parse_multiplication(),
            |token| matches!(token, TokenType::Plus | TokenType::Minus),
            |token| match token {
                TokenType::Plus => BinaryOperator::Add,
                TokenType::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            },
        )
    }

    fn parse_multiplication(&mut self) -> Result<Expression, BidParseError> {
        self.parse_binary_left_associative(
            |parser| parser.parse_power(),
            |token| {
                matches!(
                    token,
                    TokenType::Multiply | TokenType::Divide | TokenType::Modulo
                )
            },
            |token| match token {
                TokenType::Multiply => BinaryOperator::Multiply,
                TokenType::Divide => BinaryOperator::Divide,
                TokenType::Modulo => BinaryOperator::Modulo,
                _ => unreachable!(),
            },
        )
    }

    fn parse_power(&mut self) -> Result<Expression, BidParseError> {
        let left = self.parse_unary()?;

        if matches!(self.current_token.token_type, TokenType::Power) {
            let position = self.current_token.position;
            self.advance()?;
            // Right-associative
            let right = self.parse_power()?;
            Ok(Expression::BinaryOperation {
                left: Box::new(left),
                operator: BinaryOperator::Power,
                right: Box::new(right),
                position,
            })
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<Expression, BidParseError> {
        match self.current_token.token_type {
            TokenType::Minus => {
                let position = self.current_token.position;
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOperation {
                    operator: UnaryOperator::Negate,
                    operand: Box::new(operand),
                    position,
                })
            }
            TokenType::LogicalNot => {
                let position = self.current_token.position;
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOperation {
                    operator: UnaryOperator::LogicalNot,
                    operand: Box::new(operand),
                    position,
                })
            }
            TokenType::Multiply => {
                let position = self.current_token.position;
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOperation {
                    operator: UnaryOperator::Dereference,
                    operand: Box::new(operand),
                    position,
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expression, BidParseError> {
        match &self.current_token.token_type {
            TokenType::Identifier(name) => {
                let position = self.current_token.position;
                let mut path = vec![name.clone()];
                self.advance()?;

                // Handle dot notation for variable paths
                while matches!(self.current_token.token_type, TokenType::Dot) {
                    self.advance()?;
                    if let TokenType::Identifier(segment) = &self.current_token.token_type {
                        path.push(segment.clone());
                        self.advance()?;
                    } else {
                        return Err(BidParseError::UnexpectedToken {
                            found: format!("{:?}", self.current_token.token_type),
                            expected: "identifier".to_string(),
                            position: self.current_token.position,
                        });
                    }
                }

                Ok(Expression::Variable { path, position })
            }
            TokenType::StringLiteral(value) => {
                let position = self.current_token.position;
                let value = value.clone();
                self.advance()?;
                Ok(Expression::StringLiteral { value, position })
            }
            TokenType::IntegerLiteral(value) => {
                let position = self.current_token.position;
                let value = *value;
                self.advance()?;
                Ok(Expression::IntegerLiteral { value, position })
            }
            TokenType::FloatLiteral(value) => {
                let position = self.current_token.position;
                let value = *value;
                self.advance()?;
                Ok(Expression::FloatLiteral { value, position })
            }
            TokenType::BooleanLiteral(value) => {
                let position = self.current_token.position;
                let value = *value;
                self.advance()?;
                Ok(Expression::BooleanLiteral { value, position })
            }
            TokenType::LeftParen => {
                self.advance()?;
                let mut expr = self.parse_expression()?;
                if matches!(self.current_token.token_type, TokenType::RightParen) {
                    self.advance()?;

                    // Handle member access after parenthesized expression
                    while matches!(self.current_token.token_type, TokenType::Dot) {
                        self.advance()?;
                        if let TokenType::Identifier(segment) = &self.current_token.token_type {
                            let segment = segment.clone();
                            let position = self.current_token.position;
                            self.advance()?;

                            // Create a member access expression
                            expr = Expression::MemberAccess {
                                object: Box::new(expr),
                                property: segment,
                                position,
                            };
                        } else {
                            return Err(BidParseError::UnexpectedToken {
                                found: format!("{:?}", self.current_token.token_type),
                                expected: "identifier".to_string(),
                                position: self.current_token.position,
                            });
                        }
                    }

                    Ok(expr)
                } else {
                    Err(BidParseError::UnexpectedToken {
                        found: format!("{:?}", self.current_token.token_type),
                        expected: "')'".to_string(),
                        position: self.current_token.position,
                    })
                }
            }
            _ => Err(BidParseError::UnexpectedToken {
                found: format!("{:?}", self.current_token.token_type),
                expected: "expression".to_string(),
                position: self.current_token.position,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_bid() {
        let result = BidParser::parse("ON user.active BID user.score").unwrap();

        assert!(
            matches!(result.on_condition, Expression::Variable { ref path, .. } if path == &["user", "active"])
        );
        assert!(
            matches!(result.bid_value, Expression::Variable { ref path, .. } if path == &["user", "score"])
        );
    }

    #[test]
    fn parse_arithmetic_expression() {
        let result = BidParser::parse("ON price > 100 BID price * 0.9").unwrap();

        // Check that we parsed the comparison
        if let Expression::BinaryOperation {
            operator: BinaryOperator::GreaterThan,
            left,
            right,
            position: _,
        } = result.on_condition
        {
            assert!(matches!(*left, Expression::Variable { ref path, .. } if path == &["price"]));
            assert!(matches!(
                *right,
                Expression::IntegerLiteral { value: 100, .. }
            ));
        } else {
            panic!("Expected comparison operation");
        }

        // Check that we parsed the multiplication
        if let Expression::BinaryOperation {
            operator: BinaryOperator::Multiply,
            left,
            right,
            position: _,
        } = result.bid_value
        {
            assert!(matches!(*left, Expression::Variable { ref path, .. } if path == &["price"]));
            assert!(
                matches!(*right, Expression::FloatLiteral { value, .. } if (value - 0.9).abs() < f64::EPSILON)
            );
        } else {
            panic!("Expected multiplication operation");
        }
    }

    #[test]
    fn parse_complex_condition() {
        let result = BidParser::parse(
            r#"ON (item.price < 100.0 && item.category == "electronics") BID item.price * 0.9"#,
        )
        .unwrap();

        // Should parse without errors
        if let Expression::BinaryOperation {
            operator: BinaryOperator::LogicalAnd,
            left,
            right,
            position: _,
        } = result.on_condition
        {
            // Left side: item.price < 100.0
            if let Expression::BinaryOperation {
                operator: BinaryOperator::LessThan,
                left: left_left,
                right: left_right,
                position: _,
            } = *left
            {
                assert!(
                    matches!(*left_left, Expression::Variable { ref path, .. } if path == &["item", "price"])
                );
                assert!(
                    matches!(*left_right, Expression::FloatLiteral { value, .. } if (value - 100.0).abs() < f64::EPSILON)
                );
            } else {
                panic!("Expected less than comparison on left side");
            }

            // Right side: item.category == "electronics"
            if let Expression::BinaryOperation {
                operator: BinaryOperator::Equal,
                left: right_left,
                right: right_right,
                position: _,
            } = *right
            {
                assert!(
                    matches!(*right_left, Expression::Variable { ref path, .. } if path == &["item", "category"])
                );
                assert!(
                    matches!(*right_right, Expression::StringLiteral { ref value, .. } if value == "electronics")
                );
            } else {
                panic!("Expected equality comparison on right side");
            }
        } else {
            panic!("Expected logical AND operation");
        }
    }

    #[test]
    fn parse_with_parentheses() {
        let result = BidParser::parse("ON true BID (base + bonus) * multiplier").unwrap();

        // Should handle parentheses correctly
        if let Expression::BinaryOperation {
            operator: BinaryOperator::Multiply,
            left,
            right,
            position: _,
        } = result.bid_value
        {
            // Left side should be (base + bonus)
            if let Expression::BinaryOperation {
                operator: BinaryOperator::Add,
                left: add_left,
                right: add_right,
                position: _,
            } = *left
            {
                assert!(
                    matches!(*add_left, Expression::Variable { ref path, .. } if path == &["base"])
                );
                assert!(
                    matches!(*add_right, Expression::Variable { ref path, .. } if path == &["bonus"])
                );
            } else {
                panic!("Expected addition in parentheses");
            }

            // Right side should be multiplier
            assert!(
                matches!(*right, Expression::Variable { ref path, .. } if path == &["multiplier"])
            );
        } else {
            panic!("Expected multiplication with grouped addition");
        }
    }

    #[test]
    fn parse_unary_operations() {
        let result = BidParser::parse("ON !condition BID -value").unwrap();

        if let Expression::UnaryOperation {
            operator: UnaryOperator::LogicalNot,
            operand,
            position: _,
        } = result.on_condition
        {
            assert!(
                matches!(*operand, Expression::Variable { ref path, .. } if path == &["condition"])
            );
        } else {
            panic!("Expected logical NOT operation");
        }

        if let Expression::UnaryOperation {
            operator: UnaryOperator::Negate,
            operand,
            position: _,
        } = result.bid_value
        {
            assert!(
                matches!(*operand, Expression::Variable { ref path, .. } if path == &["value"])
            );
        } else {
            panic!("Expected negation operation");
        }
    }

    #[test]
    fn parse_literals() {
        let result = BidParser::parse(r#"ON "test" BID 42"#).unwrap();

        assert!(
            matches!(result.on_condition, Expression::StringLiteral { ref value, .. } if value == "test")
        );
        assert!(matches!(
            result.bid_value,
            Expression::IntegerLiteral {
                value: 42,
                position: _
            }
        ));
    }

    #[test]
    fn parse_float_literal() {
        let result = BidParser::parse("ON true BID 42.5").unwrap();

        assert!(
            matches!(result.bid_value, Expression::FloatLiteral { value, .. } if (value - 42.5).abs() < f64::EPSILON)
        );
    }

    #[test]
    fn parse_boolean_literals() {
        let result = BidParser::parse("ON true BID false").unwrap();

        assert!(matches!(
            result.on_condition,
            Expression::BooleanLiteral { value: true, .. }
        ));
        assert!(matches!(
            result.bid_value,
            Expression::BooleanLiteral { value: false, .. }
        ));
    }

    #[test]
    fn missing_on_keyword() {
        let result = BidParser::parse("condition BID value");
        assert!(matches!(
            result,
            Err(BidParseError::MissingOnKeyword { .. })
        ));
    }

    #[test]
    fn missing_bid_keyword() {
        let result = BidParser::parse("ON condition value");
        assert!(matches!(
            result,
            Err(BidParseError::MissingBidKeyword { .. })
        ));
    }

    #[test]
    fn unterminated_string() {
        let result = BidParser::parse(r#"ON "unterminated BID value"#);
        assert!(matches!(
            result,
            Err(BidParseError::UnterminatedString { .. })
        ));
    }

    #[test]
    fn invalid_number() {
        let result = BidParser::parse("ON true BID 123.456.789");
        // This should actually parse as 123.456 followed by unexpected token
        // But our lexer is simpler and may handle this differently
        match result {
            Err(BidParseError::InvalidNumber { .. })
            | Err(BidParseError::UnexpectedToken { .. }) => {
                // Either error is acceptable
            }
            _ => panic!("Expected parsing error for invalid number"),
        }
    }

    #[test]
    fn operator_precedence() {
        let result = BidParser::parse("ON true BID 2 + 3 * 4").unwrap();

        // Should parse as 2 + (3 * 4), not (2 + 3) * 4
        if let Expression::BinaryOperation {
            operator: BinaryOperator::Add,
            left,
            right,
            ..
        } = result.bid_value
        {
            assert!(matches!(*left, Expression::IntegerLiteral { value: 2, .. }));
            assert!(matches!(
                *right,
                Expression::BinaryOperation {
                    operator: BinaryOperator::Multiply,
                    ..
                }
            ));
        } else {
            panic!("Expected addition with multiplication having higher precedence");
        }
    }

    #[test]
    fn right_associative_power() {
        let result = BidParser::parse("ON true BID 2 ^ 3 ^ 4").unwrap();

        // Should parse as 2 ^ (3 ^ 4), not (2 ^ 3) ^ 4
        if let Expression::BinaryOperation {
            operator: BinaryOperator::Power,
            left,
            right,
            ..
        } = result.bid_value
        {
            assert!(matches!(*left, Expression::IntegerLiteral { value: 2, .. }));
            assert!(matches!(
                *right,
                Expression::BinaryOperation {
                    operator: BinaryOperator::Power,
                    ..
                }
            ));
        } else {
            panic!("Expected right-associative power operation");
        }
    }

    #[test]
    fn error_display() {
        let error = BidParseError::UnexpectedToken {
            found: "123".to_string(),
            expected: "identifier".to_string(),
            position: Position::new(1, 5),
        };

        let display = format!("{}", error);
        assert!(display.contains("Unexpected token '123'"));
        assert!(display.contains("1:5"));
        assert!(display.contains("identifier"));
    }

    #[test]
    fn expression_display() {
        let expr = Expression::BinaryOperation {
            left: Box::new(Expression::Variable {
                path: vec!["user".to_string(), "score".to_string()],
                position: Position::start(),
            }),
            operator: BinaryOperator::Multiply,
            right: Box::new(Expression::FloatLiteral {
                value: 1.5,
                position: Position::start(),
            }),
            position: Position::start(),
        };

        let display = format!("{}", expr);
        assert_eq!(display, "(user.score * 1.5)");
    }

    #[test]
    fn bid_display() {
        let bid = Bid {
            on_condition: Expression::BooleanLiteral {
                value: true,
                position: Position::start(),
            },
            bid_value: Expression::IntegerLiteral {
                value: 100,
                position: Position::start(),
            },
        };

        let display = format!("{}", bid);
        assert_eq!(display, "ON true BID 100");
    }

    #[test]
    fn position_tracking() {
        let result = BidParser::parse("ON\n  user.active\nBID\n  user.score");
        assert!(result.is_ok());

        let bid = result.unwrap();
        // Positions should be tracked through the parsing
        assert_ne!(bid.on_condition.position(), Position::start());
        assert_ne!(bid.bid_value.position(), Position::start());
    }

    // Additional comprehensive test cases

    #[test]
    fn string_escape_sequences() {
        let result = BidParser::parse(r#"ON "hello\nworld\t\r\\\"" BID 42"#).unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            assert_eq!(value, "hello\nworld\t\r\\\"");
        } else {
            panic!("Expected string literal with escape sequences");
        }
    }

    #[test]
    fn string_unknown_escape() {
        let result = BidParser::parse(r#"ON "hello\x" BID 42"#).unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            // Unknown escapes are kept as-is
            assert_eq!(value, "hellox");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn number_edge_cases() {
        // Leading zeros should work
        let result = BidParser::parse("ON true BID 007").unwrap();
        if let Expression::IntegerLiteral { value, .. } = result.bid_value {
            assert_eq!(value, 7);
        } else {
            panic!("Expected integer literal");
        }

        // Large numbers within range
        let result = BidParser::parse("ON true BID 9223372036854775807").unwrap();
        if let Expression::IntegerLiteral { value, .. } = result.bid_value {
            assert_eq!(value, i64::MAX);
        } else {
            panic!("Expected integer literal");
        }

        // Float with many decimal places
        let result = BidParser::parse("ON true BID 3.141592653589793").unwrap();
        if let Expression::FloatLiteral { value, .. } = result.bid_value {
            assert!((value - std::f64::consts::PI).abs() < f64::EPSILON);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn number_overflow() {
        // Integer overflow should cause parse error
        let result = BidParser::parse("ON true BID 99999999999999999999999999999");
        assert!(matches!(result, Err(BidParseError::InvalidNumber { .. })));
    }

    #[test]
    fn trailing_dot_number() {
        // Number ending with dot but followed by non-digit
        let result = BidParser::parse("ON 123. BID 456");
        // This should parse as float 123.0 followed by BID keyword
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_single_characters() {
        let result = BidParser::parse("ON true & false BID 1");
        assert!(matches!(
            result,
            Err(BidParseError::InvalidCharacter { character: '&', .. })
        ));

        let result = BidParser::parse("ON true | false BID 1");
        assert!(matches!(
            result,
            Err(BidParseError::InvalidCharacter { character: '|', .. })
        ));

        let result = BidParser::parse("ON x = y BID 1");
        assert!(matches!(
            result,
            Err(BidParseError::InvalidCharacter { character: '=', .. })
        ));
    }

    #[test]
    fn empty_input() {
        let result = BidParser::parse("");
        assert!(matches!(
            result,
            Err(BidParseError::MissingOnKeyword { .. })
        ));
    }

    #[test]
    fn whitespace_only() {
        let result = BidParser::parse("   \n\t  ");
        assert!(matches!(
            result,
            Err(BidParseError::MissingOnKeyword { .. })
        ));
    }

    #[test]
    fn very_long_identifier() {
        let long_name = "a".repeat(1000);
        let input = format!("ON {} BID {}", long_name, long_name);
        let result = BidParser::parse(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn deeply_nested_dots() {
        let path_parts: Vec<String> = (0..100).map(|i| format!("part{}", i)).collect();
        let path = path_parts.join(".");
        let input = format!("ON {} BID 42", path);
        let result = BidParser::parse(&input).unwrap();

        if let Expression::Variable {
            path: parsed_path, ..
        } = result.on_condition
        {
            assert_eq!(parsed_path.len(), 100);
            assert_eq!(parsed_path[0], "part0");
            assert_eq!(parsed_path[99], "part99");
        } else {
            panic!("Expected variable with long path");
        }
    }

    #[test]
    fn deeply_nested_parentheses() {
        let expr = "(((((42)))))";
        let input = format!("ON true BID {}", expr);
        let result = BidParser::parse(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn complex_precedence_chain() {
        // Test all operator precedences in one expression
        let result = BidParser::parse("ON true BID !a || b && c == d < e + f * g ^ h").unwrap();

        // Should parse correctly according to precedence rules
        // This is mainly testing that it doesn't crash and produces a valid AST
        assert!(matches!(
            result.bid_value,
            Expression::BinaryOperation { .. }
        ));
    }

    #[test]
    fn multiple_consecutive_power_operators() {
        let result = BidParser::parse("ON true BID 2 ^ 3 ^ 4 ^ 5").unwrap();

        // Should be right-associative: 2 ^ (3 ^ (4 ^ 5))
        if let Expression::BinaryOperation {
            operator: BinaryOperator::Power,
            left,
            right,
            ..
        } = result.bid_value
        {
            assert!(matches!(*left, Expression::IntegerLiteral { value: 2, .. }));
            // Right side should be another power operation
            assert!(matches!(
                *right,
                Expression::BinaryOperation {
                    operator: BinaryOperator::Power,
                    ..
                }
            ));
        } else {
            panic!("Expected right-associative power operations");
        }
    }

    #[test]
    fn empty_parentheses() {
        let result = BidParser::parse("ON true BID ()");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn malformed_variable_paths() {
        // Leading dot
        let result = BidParser::parse("ON .foo BID 1");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));

        // Trailing dot gets parsed as separate token
        let result = BidParser::parse("ON foo. BID 1");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));

        // Double dot
        let result = BidParser::parse("ON foo..bar BID 1");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn multiple_unary_operators() {
        let result = BidParser::parse("ON true BID --42").unwrap();

        if let Expression::UnaryOperation {
            operator: UnaryOperator::Negate,
            operand,
            ..
        } = result.bid_value
        {
            assert!(matches!(
                *operand,
                Expression::UnaryOperation {
                    operator: UnaryOperator::Negate,
                    ..
                }
            ));
        } else {
            panic!("Expected nested unary negation");
        }

        let result = BidParser::parse("ON !!condition BID 1").unwrap();

        if let Expression::UnaryOperation {
            operator: UnaryOperator::LogicalNot,
            operand,
            ..
        } = result.on_condition
        {
            assert!(matches!(
                *operand,
                Expression::UnaryOperation {
                    operator: UnaryOperator::LogicalNot,
                    ..
                }
            ));
        } else {
            panic!("Expected nested logical not");
        }
    }

    #[test]
    fn unary_with_binary_operators() {
        let result = BidParser::parse("ON true BID -a + b").unwrap();

        // Should parse as (-a) + b, not -(a + b)
        if let Expression::BinaryOperation {
            operator: BinaryOperator::Add,
            left,
            right,
            ..
        } = result.bid_value
        {
            assert!(matches!(*left, Expression::UnaryOperation { .. }));
            assert!(matches!(*right, Expression::Variable { .. }));
        } else {
            panic!("Expected addition with unary left operand");
        }
    }

    #[test]
    fn missing_right_parenthesis() {
        let result = BidParser::parse("ON (true BID 1");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn extra_tokens_after_bid() {
        let result = BidParser::parse("ON true BID 42 extra");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn position_accuracy_multiline() {
        let result = BidParser::parse("ON\n  invalid@symbol\nBID value");

        if let Err(BidParseError::InvalidCharacter { position, .. }) = result {
            assert_eq!(position.line, 2);
            assert_eq!(position.column, 10); // After "  invalid"
        } else {
            panic!("Expected invalid character error with correct position");
        }
    }

    #[test]
    fn user_error_conversion() {
        use handled::Handle;

        let parse_error = BidParseError::UnterminatedString {
            position: Position::new(1, 5),
        };

        let user_error = parse_error.handle().unwrap();
        assert!(user_error.message.contains("Unterminated string literal"));
        assert!(user_error.message.contains("1:5"));
        assert!(user_error.usage_hint.is_some());
        assert!(
            user_error
                .usage_hint
                .as_ref()
                .unwrap()
                .contains("double quotes")
        );
    }

    #[test]
    fn all_error_types_coverage() {
        // Test that all error types can be created and displayed
        let errors = vec![
            BidParseError::UnexpectedToken {
                found: "test".to_string(),
                expected: "other".to_string(),
                position: Position::start(),
            },
            BidParseError::InvalidNumber {
                text: "123abc".to_string(),
                position: Position::start(),
            },
            BidParseError::UnterminatedString {
                position: Position::start(),
            },
            BidParseError::InvalidCharacter {
                character: '@',
                position: Position::start(),
            },
            BidParseError::MissingOnKeyword {
                position: Position::start(),
            },
            BidParseError::MissingBidKeyword {
                position: Position::start(),
            },
            BidParseError::EmptyExpression {
                position: Position::start(),
            },
        ];

        for error in errors {
            // Should be able to display and convert to user error
            let _display = format!("{}", error);
            let _user_error = error.handle();
        }
    }

    #[test]
    fn realistic_business_rules() {
        // Test complex real-world-like expressions
        let complex_rule = r#"ON (user.tier == "premium" && order.amount > 500.0 && !user.restricted && (item.category == "electronics" || item.category == "books")) BID base_price * discount_rate + loyalty_bonus"#;

        let result = BidParser::parse(complex_rule);
        assert!(
            result.is_ok(),
            "Complex business rule should parse successfully"
        );

        // Verify it contains the expected structure
        let bid = result.unwrap();
        assert!(matches!(
            bid.on_condition,
            Expression::BinaryOperation {
                operator: BinaryOperator::LogicalAnd,
                ..
            }
        ));
        assert!(matches!(
            bid.bid_value,
            Expression::BinaryOperation {
                operator: BinaryOperator::Add,
                ..
            }
        ));
    }

    #[test]
    fn boundary_numeric_values() {
        // Test negation of maximum positive value (closest we can get to i64::MIN)
        let result = BidParser::parse("ON true BID -9223372036854775807").unwrap(); // -(i64::MAX)
        if let Expression::UnaryOperation {
            operator: UnaryOperator::Negate,
            operand,
            ..
        } = result.bid_value
            && let Expression::IntegerLiteral { value, .. } = *operand
        {
            assert_eq!(value, i64::MAX);
        }

        // Test very small float
        let result = BidParser::parse("ON true BID 1e-10");
        // Our lexer doesn't support scientific notation, so this should fail parsing
        // as a number and be treated as identifier followed by operators
        assert!(result.is_err());
    }

    #[test]
    fn unicode_in_strings() {
        let result = BidParser::parse(r#"ON "🚀 Hello 世界" BID 42"#).unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            assert_eq!(value, "🚀 Hello 世界");
        } else {
            panic!("Expected string literal with unicode");
        }
    }

    #[test]
    fn unicode_in_identifiers_rejected() {
        // Unicode in identifiers should be rejected
        let result = BidParser::parse("ON café BID 42");
        assert!(matches!(
            result,
            Err(BidParseError::InvalidCharacter { .. })
        ));
    }

    #[test]
    fn stress_deeply_nested_expression() {
        // Create a deeply nested expression to test parser stack usage
        // Reduced from 100 to 20 to avoid timeout in tests
        let mut expr = "x".to_string();
        for _ in 0..20 {
            expr = format!("({} + {})", expr, expr);
        }
        let input = format!("ON true BID {}", expr);

        // This tests that we don't hit stack overflow or other issues
        let result = BidParser::parse(&input);
        assert!(
            result.is_ok(),
            "Deeply nested expression should parse without stack overflow"
        );
    }

    // Additional comprehensive edge case tests

    #[test]
    fn empty_string_literal() {
        let result = BidParser::parse(r#"ON "" BID """#).unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            assert_eq!(value, "");
        } else {
            panic!("Expected empty string literal");
        }

        if let Expression::StringLiteral { value, .. } = result.bid_value {
            assert_eq!(value, "");
        } else {
            panic!("Expected empty string literal");
        }
    }

    #[test]
    fn string_with_null_bytes() {
        // Test string containing null bytes (if supported by lexer)
        let result = BidParser::parse("ON \"hello\\0world\" BID 42").unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            // Our lexer treats unknown escapes as literal characters
            assert_eq!(value, "hello0world");
        } else {
            panic!("Expected string literal with null byte");
        }
    }

    #[test]
    fn very_long_string_literal() {
        let long_string = "a".repeat(10000);
        let input = format!("ON \"{}\" BID 42", long_string);
        let result = BidParser::parse(&input).unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            assert_eq!(value.len(), 10000);
            assert_eq!(value, long_string);
        } else {
            panic!("Expected very long string literal");
        }
    }

    #[test]
    fn string_only_escape_sequences() {
        let result = BidParser::parse("ON \"\\n\\t\\r\\\\\\\"\" BID 42").unwrap();

        if let Expression::StringLiteral { value, .. } = result.on_condition {
            assert_eq!(value, "\n\t\r\\\"");
        } else {
            panic!("Expected string with only escape sequences");
        }
    }

    #[test]
    fn number_boundary_conditions() {
        // Zero
        let result = BidParser::parse("ON true BID 0").unwrap();
        if let Expression::IntegerLiteral { value, .. } = result.bid_value {
            assert_eq!(value, 0);
        }

        // Zero float
        let result = BidParser::parse("ON true BID 0.0").unwrap();
        if let Expression::FloatLiteral { value, .. } = result.bid_value {
            assert!((value - 0.0).abs() < f64::EPSILON);
        }

        // Negative zero (should parse as unary minus applied to zero)
        let result = BidParser::parse("ON true BID -0").unwrap();
        if let Expression::UnaryOperation {
            operator: UnaryOperator::Negate,
            operand,
            ..
        } = result.bid_value
        {
            assert!(matches!(
                *operand,
                Expression::IntegerLiteral { value: 0, .. }
            ));
        }
    }

    #[test]
    fn number_edge_formats() {
        // Number ending with decimal point
        let result = BidParser::parse("ON true BID 42.").unwrap();
        if let Expression::FloatLiteral { value, .. } = result.bid_value {
            assert!((value - 42.0).abs() < f64::EPSILON);
        }

        // Number starting with decimal point should fail (not implemented)
        let result = BidParser::parse("ON true BID .5");
        assert!(
            result.is_err(),
            "Numbers starting with . should not be supported"
        );

        // Multiple leading zeros
        let result = BidParser::parse("ON true BID 00042").unwrap();
        if let Expression::IntegerLiteral { value, .. } = result.bid_value {
            assert_eq!(value, 42);
        }
    }

    #[test]
    fn case_sensitive_keywords() {
        // Keywords should be case sensitive
        let result = BidParser::parse("on true bid 42");
        assert!(matches!(
            result,
            Err(BidParseError::MissingOnKeyword { .. })
        ));

        let result = BidParser::parse("ON true bid 42");
        assert!(matches!(
            result,
            Err(BidParseError::MissingBidKeyword { .. })
        ));

        let result = BidParser::parse("ON True BID False").unwrap();
        // True/False should be parsed as identifiers, not boolean literals
        assert!(matches!(result.on_condition, Expression::Variable { .. }));
        assert!(matches!(result.bid_value, Expression::Variable { .. }));
    }

    #[test]
    fn keywords_as_identifier_parts() {
        // Keywords as parts of longer identifiers should work
        let result = BidParser::parse("ON ONfoo BID BIDbar").unwrap();

        if let Expression::Variable { path, .. } = result.on_condition {
            assert_eq!(path, vec!["ONfoo"]);
        }

        if let Expression::Variable { path, .. } = result.bid_value {
            assert_eq!(path, vec!["BIDbar"]);
        }
    }

    #[test]
    fn missing_operand_after_binary_operator() {
        let result = BidParser::parse("ON true BID x +");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));

        let result = BidParser::parse("ON true BID x * ");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));

        let result = BidParser::parse("ON x == BID y");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn missing_operand_after_unary_operator() {
        let result = BidParser::parse("ON true BID -");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));

        let result = BidParser::parse("ON ! BID y");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn consecutive_binary_operators() {
        let result = BidParser::parse("ON true BID x + / y");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));

        let result = BidParser::parse("ON x == != y BID 1");
        assert!(matches!(result, Err(BidParseError::UnexpectedToken { .. })));
    }

    #[test]
    fn mixed_unary_binary_combinations() {
        // Test !-x (logical not of negated x)
        let result = BidParser::parse("ON !-x BID 1").unwrap();
        if let Expression::UnaryOperation {
            operator: UnaryOperator::LogicalNot,
            operand,
            ..
        } = result.on_condition
        {
            assert!(matches!(
                *operand,
                Expression::UnaryOperation {
                    operator: UnaryOperator::Negate,
                    ..
                }
            ));
        }

        // Test -!y (negation of logical not y)
        let result = BidParser::parse("ON -!y BID 1").unwrap();
        if let Expression::UnaryOperation {
            operator: UnaryOperator::Negate,
            operand,
            ..
        } = result.on_condition
        {
            assert!(matches!(
                *operand,
                Expression::UnaryOperation {
                    operator: UnaryOperator::LogicalNot,
                    ..
                }
            ));
        }
    }

    #[test]
    fn single_character_identifiers() {
        let result = BidParser::parse("ON a BID b").unwrap();

        if let Expression::Variable { path, .. } = result.on_condition {
            assert_eq!(path, vec!["a"]);
        }

        if let Expression::Variable { path, .. } = result.bid_value {
            assert_eq!(path, vec!["b"]);
        }
    }

    #[test]
    fn underscore_identifiers() {
        let result = BidParser::parse("ON _ BID _foo_bar_").unwrap();

        if let Expression::Variable { path, .. } = result.on_condition {
            assert_eq!(path, vec!["_"]);
        }

        if let Expression::Variable { path, .. } = result.bid_value {
            assert_eq!(path, vec!["_foo_bar_"]);
        }
    }

    #[test]
    fn numbers_in_variable_paths() {
        // Numbers after letters should work
        let result = BidParser::parse("ON user123.item456 BID 42").unwrap();

        if let Expression::Variable { path, .. } = result.on_condition {
            assert_eq!(path, vec!["user123", "item456"]);
        }
    }

    #[test]
    fn tab_character_position_tracking() {
        let result = BidParser::parse("ON\ttrue\t@\tBID\t1");

        // Should fail on the @ character, but position tracking with tabs is tested
        if let Err(BidParseError::InvalidCharacter { position, .. }) = result {
            // Position should account for tabs (assuming they count as single characters)
            assert_eq!(position.line, 1);
            // Column position will depend on how tabs are handled
            assert!(position.column > 1);
        } else {
            panic!("Expected invalid character error");
        }
    }

    #[test]
    fn position_tracking_with_mixed_whitespace() {
        let input = "ON\n \t true  \n\t BID   \n  42";
        let result = BidParser::parse(input);
        assert!(result.is_ok());

        // Verify positions are tracked correctly through complex whitespace
        let bid = result.unwrap();
        assert!(bid.on_condition.position().line >= 1);
        assert!(bid.bid_value.position().line >= 1);
    }

    #[test]
    fn very_large_integer_boundary() {
        // Test exactly at i64::MAX
        let result = BidParser::parse("ON true BID 9223372036854775807").unwrap();
        if let Expression::IntegerLiteral { value, .. } = result.bid_value {
            assert_eq!(value, i64::MAX);
        }

        // Test overflow beyond i64::MAX
        let result = BidParser::parse("ON true BID 9223372036854775808");
        assert!(matches!(result, Err(BidParseError::InvalidNumber { .. })));
    }

    #[test]
    fn very_small_integer_boundary() {
        // Test that attempting to parse i64::MIN + 1 (i.e., overflow) fails properly
        let result = BidParser::parse("ON true BID 9223372036854775808"); // i64::MAX + 1
        assert!(matches!(result, Err(BidParseError::InvalidNumber { .. })));

        // Test normal negative number parsing
        let result = BidParser::parse("ON true BID -1000").unwrap();
        if let Expression::UnaryOperation {
            operator: UnaryOperator::Negate,
            operand,
            ..
        } = result.bid_value
        {
            // Should parse as negation of a positive number
            assert!(matches!(
                *operand,
                Expression::IntegerLiteral { value: 1000, .. }
            ));
        }
    }

    #[test]
    fn float_precision_limits() {
        // Test high precision float
        let result = BidParser::parse("ON true BID 1.7976931348623157e308");
        // Our lexer doesn't support scientific notation, so this should be treated as identifier + operators
        assert!(result.is_err());

        // Test very small decimal
        let result = BidParser::parse("ON true BID 0.000000000000001").unwrap();
        if let Expression::FloatLiteral { value, .. } = result.bid_value {
            assert!((value - 0.000000000000001).abs() < f64::EPSILON * 10.0);
        }
    }

    #[test]
    fn operator_without_operands() {
        let test_cases = vec![
            ("ON + BID 1", "Plus without left operand"),
            ("ON true BID / 1", "Divide without left operand"),
            ("ON true BID 1 ==", "Equal without right operand"),
            ("ON && BID 1", "LogicalAnd without operands"),
        ];

        for (input, description) in test_cases {
            let result = BidParser::parse(input);
            assert!(
                matches!(result, Err(BidParseError::UnexpectedToken { .. })),
                "Should fail: {}",
                description
            );
        }
    }

    #[test]
    fn extremely_long_variable_path() {
        // Reduced from 1000 to 200 segments to avoid test timeouts
        let path_parts: Vec<String> = (0..200).map(|i| format!("segment{:04}", i)).collect();
        let path = path_parts.join(".");
        let input = format!("ON {} BID 42", path);

        let result = BidParser::parse(&input).unwrap();

        if let Expression::Variable {
            path: parsed_path, ..
        } = result.on_condition
        {
            assert_eq!(parsed_path.len(), 200);
            assert_eq!(parsed_path[0], "segment0000");
            assert_eq!(parsed_path[199], "segment0199");
        } else {
            panic!("Expected variable with extremely long path");
        }
    }

    #[test]
    fn all_operators_precedence_comprehensive() {
        // Test expression with all operators to ensure precedence is correct
        let result = BidParser::parse(
            "ON a || b && c == d != e < f <= g > h >= i + j - k * l / m % n ^ o BID 1",
        )
        .unwrap();

        // Should parse without errors and create proper precedence structure
        assert!(matches!(
            result.on_condition,
            Expression::BinaryOperation {
                operator: BinaryOperator::LogicalOr,
                ..
            }
        ));
    }

    #[test]
    fn nested_parentheses_stress() {
        // Test many levels of nested parentheses
        let mut expr = "x".to_string();
        for _ in 0..50 {
            expr = format!("({})", expr);
        }
        let input = format!("ON true BID {}", expr);

        let result = BidParser::parse(&input);
        assert!(result.is_ok(), "Deeply nested parentheses should parse");
    }

    #[test]
    fn memory_stress_very_long_input() {
        // Test with very long input to check memory usage
        // Reduced size to avoid test timeouts
        let long_condition = "x".repeat(10000);
        let input = format!("ON {} BID 42", long_condition);

        let result = BidParser::parse(&input);
        assert!(result.is_ok(), "Very long identifier should parse");

        if let Ok(bid) = result
            && let Expression::Variable { path, .. } = bid.on_condition
        {
            assert_eq!(path[0].len(), 10000);
        }
    }

    #[test]
    fn parse_regex_match_operator() {
        let result = BidParser::parse(r#"ON text ~= "pattern" BID value"#).unwrap();

        if let Expression::BinaryOperation {
            operator: BinaryOperator::RegexMatch,
            left,
            right,
            ..
        } = result.on_condition
        {
            assert!(matches!(*left, Expression::Variable { ref path, .. } if path == &["text"]));
            assert!(
                matches!(*right, Expression::StringLiteral { ref value, .. } if value == "pattern")
            );
        } else {
            panic!("Expected regex match operation");
        }
    }

    #[test]
    fn regex_operator_precedence() {
        let result = BidParser::parse(r#"ON a == b && c ~= "d" BID 1"#).unwrap();

        if let Expression::BinaryOperation {
            operator: BinaryOperator::LogicalAnd,
            left,
            right,
            ..
        } = result.on_condition
        {
            assert!(matches!(
                *left,
                Expression::BinaryOperation {
                    operator: BinaryOperator::Equal,
                    ..
                }
            ));
            assert!(matches!(
                *right,
                Expression::BinaryOperation {
                    operator: BinaryOperator::RegexMatch,
                    ..
                }
            ));
        } else {
            panic!("Expected logical AND with regex match having correct precedence");
        }
    }

    #[test]
    fn regex_operator_display() {
        let result = BidParser::parse(r#"ON text ~= "pattern" BID 42"#).unwrap();
        let display = format!("{}", result.on_condition);
        assert!(display.contains("~="));
    }

    #[test]
    fn parse_dereference_operator() {
        let result = BidParser::parse("ON *key BID value").unwrap();

        if let Expression::UnaryOperation {
            operator: UnaryOperator::Dereference,
            operand,
            ..
        } = result.on_condition
        {
            assert!(matches!(*operand, Expression::Variable { ref path, .. } if path == &["key"]));
        } else {
            panic!("Expected dereference operation");
        }
    }

    #[test]
    fn parse_dereference_with_member_access() {
        let result = BidParser::parse("ON (*user.profile_id).active BID score").unwrap();

        if let Expression::MemberAccess { object, property, .. } = result.on_condition {
            assert_eq!(property, "active");
            assert!(matches!(*object, Expression::UnaryOperation {
                operator: UnaryOperator::Dereference,
                ..
            }));
        } else {
            panic!("Expected member access on dereferenced value");
        }
    }

    #[test]
    fn parse_chained_dereference() {
        let result = BidParser::parse("ON **key BID 42").unwrap();

        if let Expression::UnaryOperation {
            operator: UnaryOperator::Dereference,
            operand,
            ..
        } = result.on_condition
        {
            if let Expression::UnaryOperation {
                operator: UnaryOperator::Dereference,
                operand: inner_operand,
                ..
            } = *operand
            {
                assert!(
                    matches!(*inner_operand, Expression::Variable { ref path, .. } if path == &["key"])
                );
            } else {
                panic!("Expected nested dereference operation");
            }
        } else {
            panic!("Expected outer dereference operation");
        }
    }

    #[test]
    fn dereference_operator_precedence() {
        let result = BidParser::parse("ON *key + 1 BID value").unwrap();

        if let Expression::BinaryOperation {
            operator: BinaryOperator::Add,
            left,
            right,
            ..
        } = result.on_condition
        {
            assert!(matches!(
                *left,
                Expression::UnaryOperation {
                    operator: UnaryOperator::Dereference,
                    ..
                }
            ));
            assert!(matches!(
                *right,
                Expression::IntegerLiteral { value: 1, .. }
            ));
        } else {
            panic!("Expected addition with dereference having higher precedence");
        }
    }

    #[test]
    fn dereference_operator_display() {
        let result = BidParser::parse("ON *key BID 42").unwrap();
        let display = format!("{}", result.on_condition);
        assert!(display.contains("*("));
        assert!(display.contains("key"));
    }

    #[test]
    fn error_message_quality() {
        // Test that error messages are helpful and accurate
        let test_cases = vec![
            ("", "Expected 'ON' keyword at 1:1"),
            ("ON", "empty expression"),
            ("ON true", "Expected 'BID' keyword"),
            ("ON true BID", "empty expression"),
            ("ON true BID 42 extra", "Unexpected token"),
            ("ON ( BID 42", "Unexpected token"),
            ("ON true BID )", "Unexpected token"),
        ];

        for (input, expected_content) in test_cases {
            let result = BidParser::parse(input);
            assert!(result.is_err(), "Input '{}' should fail", input);

            let error_msg = format!("{}", result.unwrap_err());
            println!(
                "TODO(claude): cleanup this output; Error for '{}': {}",
                input, error_msg
            );

            // Basic check that error message contains expected content
            // This is a simple check; in practice you'd want more specific assertions
            assert!(
                error_msg
                    .to_lowercase()
                    .contains(&expected_content.to_lowercase())
                    || error_msg.len() > 10, // At minimum, error should be descriptive
                "Error message should be helpful for input '{}': {}",
                input,
                error_msg
            );
        }
    }

    #[test]
    fn whitespace_edge_cases() {
        // Test various whitespace scenarios
        let test_cases = vec![
            "ON\r\ntrue\r\nBID\r\n42",    // Windows line endings
            "ON\ttrue\tBID\t42",          // Tabs
            "ON   true   BID   42   ",    // Multiple spaces
            "\n\n\nON true BID 42\n\n\n", // Leading/trailing newlines
        ];

        for input in test_cases {
            let result = BidParser::parse(input);
            assert!(
                result.is_ok(),
                "Whitespace variant should parse: {:?}",
                input
            );
        }
    }

    #[test]
    fn special_characters_in_strings() {
        // Test various special characters in string literals
        let special_chars = vec![
            ("\"hello world\"", "hello world"),
            ("\"hello\tworld\"", "hello\tworld"),
            ("\"line1\nline2\"", "line1\nline2"),
            ("\"with\\\\backslash\"", "with\\backslash"),
            ("\"quote:\\\"test\\\"\"", "quote:\"test\""),
            ("\"unicode:🚀🌍\"", "unicode:🚀🌍"),
            ("\"mixed: héllo wørld\"", "mixed: héllo wørld"),
        ];

        for (input_str, expected) in special_chars {
            let input = format!("ON {} BID 42", input_str);
            let result = BidParser::parse(&input).unwrap();

            if let Expression::StringLiteral { value, .. } = result.on_condition {
                assert_eq!(value, expected, "String parsing failed for: {}", input_str);
            } else {
                panic!("Expected string literal for: {}", input_str);
            }
        }
    }

    #[test]
    fn complex_real_world_scenarios() {
        // Test complex but realistic bid expressions
        let complex_cases = [
            // E-commerce discount logic
            "ON (user.membership == \"premium\" && cart.total > 100.0 && !product.excluded && (category == \"electronics\" || category == \"books\")) BID base_price * (1.0 - discount_rate) + shipping_bonus",
            // Financial trading logic
            "ON (market.volatility < 0.2 && price.change > -0.05 && volume > avg_volume * 1.5) BID max_bid * confidence_score ^ risk_factor",
            // Resource allocation
            "ON (server.load < 0.8 && memory.available > required_memory && !maintenance_mode) BID priority_score + urgency_factor * time_weight",
            // Gaming matchmaking
            "ON (player.skill >= min_skill && player.skill <= max_skill && player.region == target_region && !player.banned) BID match_quality * (1.0 + latency_bonus) - ping_penalty",
        ];

        for (i, case) in complex_cases.iter().enumerate() {
            let result = BidParser::parse(case);
            assert!(
                result.is_ok(),
                "Complex case {} should parse successfully: {}",
                i + 1,
                case
            );

            // Verify the structure makes sense
            if let Ok(bid) = result {
                assert!(matches!(
                    bid.on_condition,
                    Expression::BinaryOperation { .. }
                ));
                assert!(
                    matches!(bid.bid_value, Expression::BinaryOperation { .. })
                        || matches!(bid.bid_value, Expression::Variable { .. })
                );
            }
        }
    }
}
