// compiler-rs/analysis/src/resolve/scope.rs

use std::collections::HashMap;

use super::table::BindingId;

/// A typed identifier for one lexical scope.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(pub u32);

impl ScopeId {
    pub const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A typed identifier for an analysis-owned local binding.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

impl LocalId {
    pub const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A lexical scope in the read-only resolution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub bindings: Vec<BindingId>,
}

impl Scope {
    pub(crate) const fn new(parent: Option<ScopeId>) -> Self {
        Self {
            parent,
            bindings: Vec::new(),
        }
    }
}

/// Mutable scope state used only while constructing a resolution table.
#[derive(Debug, Default)]
pub(crate) struct ScopeTree {
    scopes: Vec<Scope>,
    names: Vec<HashMap<String, BindingId>>,
}

impl ScopeTree {
    pub(crate) fn new() -> Self {
        let mut tree = Self::default();
        tree.create(None);
        tree
    }

    pub(crate) fn create(&mut self, parent: Option<ScopeId>) -> ScopeId {
        let id = ScopeId::new(self.scopes.len());
        self.scopes.push(Scope::new(parent));
        self.names.push(HashMap::new());
        id
    }

    pub(crate) fn contains_current(&self, scope: ScopeId, name: &str) -> bool {
        self.names[scope.index()].contains_key(name)
    }

    pub(crate) fn insert(&mut self, scope: ScopeId, name: String, binding: BindingId) {
        self.scopes[scope.index()].bindings.push(binding);
        self.names[scope.index()].insert(name, binding);
    }

    pub(crate) fn lookup(&self, start: ScopeId, name: &str) -> Option<BindingId> {
        let mut scope = Some(start);
        while let Some(id) = scope {
            if let Some(binding) = self.names[id.index()].get(name) {
                return Some(*binding);
            }
            scope = self.scopes[id.index()].parent;
        }
        None
    }

    pub(crate) fn into_scopes(self) -> Vec<Scope> {
        self.scopes
    }
}
