use std::{hash::Hash, ops::Deref};

use fxhash::FxHashMap;

pub trait LeakedBounds: 'static + Hash + Eq {}

pub struct LeakedLabel<T: LeakedBounds>(&'static T);

impl<T: LeakedBounds> Deref for LeakedLabel<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: LeakedBounds> PartialEq for LeakedLabel<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other)
    }
}

pub trait RenderLabel: Hash + Eq {
    fn _object_safety(&self);
}

pub trait RenderNode {
    fn run(&self);
}

pub struct RenderEdge(LeakedLabel<dyn RenderLabel>, LeakedLabel<dyn RenderLabel>);

pub struct RenderGraph {
    nodes: FxHashMap<LeakedLabel<dyn RenderLabel>, Box<dyn RenderNode>>,
    edges: Vec<RenderEdge>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self {
            nodes: FxHashMap::default(),
            edges: Vec::new(),
        }
    }
}

impl RenderGraph {
    pub fn add_node<L: Into<LeakedLabel<dyn RenderLabel>>, T: RenderNode>(
        &mut self,
        label: L,
        node: T,
    ) {
        self.nodes.insert();
    }
}
