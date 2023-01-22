pub use crate::ast::{BinaryOperator, DimensionExpression};
use crate::{number::Number, registry::BaseRepresentation};

pub type Type = BaseRepresentation;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Scalar(Number),
    Identifier(String, Type),
    Negate(Box<Expression>, Type),
    BinaryOperator(BinaryOperator, Box<Expression>, Box<Expression>, Type),
    FunctionCall(String, Vec<Expression>, Type),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    DeclareVariable(String, Expression, Type),
    DeclareFunction(String, Vec<(String, Type)>, Option<Expression>, Type),
    DeclareDimension(String),
    DeclareBaseUnit(String, Type),
    DeclareDerivedUnit(String, Expression),
    ProcedureCall(crate::ast::ProcedureKind, Vec<Expression>),
}

impl Expression {
    pub(crate) fn get_type(&self) -> Type {
        match self {
            Expression::Scalar(_) => Type::unity(),
            Expression::Identifier(_, type_) => type_.clone(),
            Expression::Negate(_, type_) => type_.clone(),
            Expression::BinaryOperator(_, _, _, type_) => type_.clone(),
            Expression::FunctionCall(_, _, type_) => type_.clone(),
        }
    }
}
