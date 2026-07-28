pub mod checker;
mod error;
mod expr;
mod unify;

pub use checker::TypeChecker;
pub use error::CheckerError;
