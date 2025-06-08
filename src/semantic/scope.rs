use std::collections::HashMap;
use crate::ast::Type;

#[derive(Clone)]
pub struct Scope {
    variables: HashMap<String, Type>,
    parent: Option<Box<Scope>>,
    scope_stack: Vec<HashMap<String, Type>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
            scope_stack: Vec::new(),
        }
    }

    pub fn add_variable<S: Into<String>>(&mut self, name: S, type_: Type) {
        self.variables.insert(name.into(), type_);
    }

    pub fn define_variable<S: Into<String>>(&mut self, name: S, type_: Type) {
        if let Some(current_scope) = self.scope_stack.last_mut() {
            current_scope.insert(name.into(), type_);
        } else {
            self.variables.insert(name.into(), type_);
        }
    }

    pub fn lookup_variable(&self, name: &str) -> Option<Type> {
        // First check the current scope stack (most recent first)
        for scope in self.scope_stack.iter().rev() {
            if let Some(type_) = scope.get(name) {
                return Some(type_.clone());
            }
        }
        
        // Then check the base variables
        if let Some(type_) = self.variables.get(name) {
            return Some(type_.clone());
        }
        
        // Finally check parent scope
        self.parent.as_ref().and_then(|p| p.lookup_variable(name))
    }

    pub fn get_variable(&self, name: &str) -> Option<Type> {
        self.lookup_variable(name)
    }

    pub fn enter(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    pub fn exit(&mut self) {
        self.scope_stack.pop();
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
        self.scope_stack.clear();
    }
} 