//! Represents parsed Ninja strings with embedded variable references, e.g.
//! `c++ $in -o $out`, and mechanisms for expanding those into plain strings.

use crate::smallmap::SmallMap;
use std::{borrow::Cow, collections::HashMap};

/// An environment providing a mapping of variable name to variable value.
/// A given EvalString may be expanded with multiple environments as possible
/// context.
pub trait Env {
    fn get_var(&self, var: &str) -> Option<Cow<str>>;
}

/// One token within an EvalString, either literal text or a variable reference.
#[derive(Debug)]
pub enum EvalPart<T: AsRef<str>> {
    Literal(T),
    VarRef(T),
}

/// A parsed but unexpanded variable-reference string, e.g. "cc $in -o $out".
/// This is generic to support EvalString<&str>, which is used for immediately-
/// expanded evals, like top-level bindings, and EvalString<String>, which is
/// used for delayed evals like in `rule` blocks.
#[derive(Debug)]
pub struct EvalString<T: AsRef<str>>(Vec<EvalPart<T>>);
impl<T: AsRef<str>> EvalString<T> {
    pub fn new(parts: Vec<EvalPart<T>>) -> Self {
        EvalString(parts)
    }
    pub fn evaluate(&self, envs: &[&dyn Env]) -> String {
        let mut val = String::new();
        for part in &self.0 {
            match part {
                EvalPart::Literal(s) => val.push_str(s.as_ref()),
                EvalPart::VarRef(v) => {
                    for env in envs {
                        if let Some(v) = env.get_var(v.as_ref()) {
                            val.push_str(&v);
                            break;
                        }
                    }
                }
            }
        }
        val
    }
}
impl EvalString<&str> {
    pub fn into_owned(self) -> EvalString<String> {
        EvalString(
            self.0
                .into_iter()
                .map(|part| match part {
                    EvalPart::Literal(s) => EvalPart::Literal(s.to_owned()),
                    EvalPart::VarRef(s) => EvalPart::VarRef(s.to_owned()),
                })
                .collect(),
        )
    }
}

/// A single scope's worth of variable definitions.
#[derive(Debug, Default)]
pub struct Vars<'text>(HashMap<&'text str, String>);

impl<'text> Vars<'text> {
    pub fn insert(&mut self, key: &'text str, val: String) {
        self.0.insert(key, val);
    }
    pub fn get(&self, key: &'text str) -> Option<&String> {
        self.0.get(key)
    }
}
impl<'a> Env for Vars<'a> {
    fn get_var(&self, var: &str) -> Option<Cow<str>> {
        self.0.get(var).map(|str| Cow::Borrowed(str.as_str()))
    }
}

/// A single scope's worth of variable definitions, before $-expansion.
/// For variables attached to a rule we keep them unexpanded in memory because
/// they may be expanded in multiple different ways depending on which rule uses
/// them.
pub type LazyVars = SmallMap<String, EvalString<String>>;
impl Env for LazyVars {
    fn get_var(&self, var: &str) -> Option<Cow<str>> {
        self.get(var).map(|val| Cow::Owned(val.evaluate(&[])))
    }
}
