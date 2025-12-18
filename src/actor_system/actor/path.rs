//! Actor path - unique identifier for running actors.

#![allow(dead_code)]

use std::cmp::Ordering;
use std::fmt::{Error, Formatter};

/// Unique identifier for running actors.
#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct ActorPath(Vec<String>);

impl ActorPath {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the root actor path (the first segment).
    pub fn root(&self) -> Self {
        if self.0.len() == 1 {
            self.clone()
        } else if !self.0.is_empty() {
            ActorPath(self.0.iter().take(1).cloned().collect())
        } else {
            ActorPath(Vec::new())
        }
    }

    pub fn parent(&self) -> Self {
        if self.0.len() > 1 {
            let mut tokens = self.0.clone();
            tokens.truncate(tokens.len() - 1);
            ActorPath(tokens)
        } else {
            ActorPath(Vec::new())
        }
    }

    pub fn key(&self) -> String {
        self.0.last().cloned().unwrap_or_default()
    }

    pub fn level(&self) -> usize {
        self.0.len()
    }

    pub fn at_level(&self, level: usize) -> Self {
        if level < 1 || level >= self.level() {
            self.clone()
        } else if self.is_top_level() {
            self.root()
        } else if level == self.level() - 1 {
            self.parent()
        } else {
            let mut tokens = self.0.clone();
            tokens.truncate(level);
            ActorPath(tokens)
        }
    }

    pub fn is_ancestor_of(&self, other: &ActorPath) -> bool {
        let me = format!("{self}/");
        other.to_string().as_str().starts_with(me.as_str())
    }

    pub fn is_descendant_of(&self, other: &ActorPath) -> bool {
        let me = self.to_string();
        me.as_str().starts_with(format!("{other}/").as_str())
    }

    pub fn is_parent_of(&self, other: &ActorPath) -> bool {
        *self == other.parent()
    }

    pub fn is_child_of(&self, other: &ActorPath) -> bool {
        self.parent() == *other
    }

    pub fn is_top_level(&self) -> bool {
        self.0.len() == 1
    }
}

impl From<&str> for ActorPath {
    fn from(str: &str) -> Self {
        let tokens: Vec<String> = str
            .split('/')
            .filter(|x| !x.trim().is_empty())
            .map(|s| s.to_string())
            .collect();
        ActorPath(tokens)
    }
}

impl From<String> for ActorPath {
    fn from(string: String) -> Self {
        ActorPath::from(string.as_str())
    }
}

impl From<&String> for ActorPath {
    fn from(string: &String) -> Self {
        ActorPath::from(string.as_str())
    }
}

impl std::ops::Div<&str> for ActorPath {
    type Output = ActorPath;

    fn div(self, rhs: &str) -> Self::Output {
        let mut keys = self.0;
        keys.push(rhs.to_string());
        ActorPath(keys)
    }
}

impl std::fmt::Display for ActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self.level().cmp(&1) {
            Ordering::Less => write!(f, "/"),
            Ordering::Equal => write!(f, "/{}", self.0[0]),
            Ordering::Greater => write!(f, "/{}", self.0.join("/")),
        }
    }
}

impl std::fmt::Debug for ActorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self.level().cmp(&1) {
            Ordering::Less => write!(f, "/"),
            Ordering::Equal => write!(f, "/{}", self.0[0]),
            Ordering::Greater => write!(f, "/{}", self.0.join("/")),
        }
    }
}
