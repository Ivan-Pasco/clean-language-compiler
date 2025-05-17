use std::collections::HashMap;
use crate::ast::Type;

#[derive(Clone)]
pub struct Scope {
    variables: HashMap<String, Type>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn add_variable<S: Into<String>>(&mut self, name: S, type_: Type) {
        self.variables.insert(name.into(), type_);
    }

    pub fn get_variable(&self, name: &str) -> Option<Type> {
        self.variables.get(name).cloned().or_else(|| {
            self.parent.as_ref().and_then(|p| p.get_variable(name))
        })
    }

    pub fn set_parent(&mut self, parent: Box<Scope>) {
        self.parent = Some(parent);
    }

    pub fn get_parent(&self) -> Option<&Box<Scope>> {
        self.parent.as_ref()
    }

    pub fn take_parent(&mut self) -> Option<Box<Scope>> {
        self.parent.take()
    }

    pub fn clear(&mut self) {
        self.variables.clear();
        self.parent = None;
    }
} 