//! Render graph — multi-pass abstraction for configurable render pipelines.
//!
//! Allows defining render passes with dependencies, resource inputs/outputs,
//! and automatic ordering. Used for deferred shading, post-processing chains,
//! and complex multi-pass rendering.

use std::collections::HashMap;

/// A named render pass in the graph.
#[derive(Debug, Clone)]
pub struct RenderPassNode {
    /// Unique name for this pass.
    pub name: String,
    /// Passes that must execute before this one.
    pub dependencies: Vec<String>,
    /// Pass type (determines how it's executed).
    pub pass_type: PassType,
    /// Whether this pass is enabled.
    pub enabled: bool,
}

/// Type of render pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassType {
    /// Shadow depth pass.
    Shadow,
    /// Geometry/PBR pass (writes color + depth).
    Geometry,
    /// SSAO pass (reads depth + normals).
    Ssao,
    /// Bloom pass (reads HDR color).
    Bloom,
    /// Tone mapping / final composite.
    PostProcess,
    /// Debug overlay (lines, wireframes).
    Debug,
    /// Sprite/UI overlay.
    Ui,
    /// Compute dispatch (particles, culling).
    Compute,
    /// Custom pass.
    Custom,
}

/// Render graph — ordered collection of render passes.
pub struct RenderGraph {
    nodes: Vec<RenderPassNode>,
    name_to_index: HashMap<String, usize>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            name_to_index: HashMap::new(),
        }
    }

    /// Add a pass to the graph.
    pub fn add_pass(&mut self, name: impl Into<String>, pass_type: PassType) -> &mut Self {
        let name = name.into();
        let index = self.nodes.len();
        self.name_to_index.insert(name.clone(), index);
        self.nodes.push(RenderPassNode {
            name,
            dependencies: Vec::new(),
            pass_type,
            enabled: true,
        });
        self
    }

    /// Add a dependency: `pass` depends on `dependency` (dependency runs first).
    pub fn add_dependency(
        &mut self,
        pass: impl AsRef<str>,
        dependency: impl Into<String>,
    ) -> &mut Self {
        let dep = dependency.into();
        if let Some(&idx) = self.name_to_index.get(pass.as_ref()) {
            self.nodes[idx].dependencies.push(dep);
        }
        self
    }

    /// Enable or disable a pass.
    pub fn set_enabled(&mut self, name: impl AsRef<str>, enabled: bool) {
        if let Some(&idx) = self.name_to_index.get(name.as_ref()) {
            self.nodes[idx].enabled = enabled;
        }
    }

    /// Get the topologically sorted execution order (respecting dependencies).
    /// Returns pass names in execution order. Disabled passes are excluded.
    pub fn execution_order(&self) -> Vec<&str> {
        let mut visited = vec![false; self.nodes.len()];
        let mut order = Vec::with_capacity(self.nodes.len());

        for i in 0..self.nodes.len() {
            if !visited[i] && self.nodes[i].enabled {
                self.visit(i, &mut visited, &mut order);
            }
        }

        order
    }

    fn visit<'a>(&'a self, index: usize, visited: &mut Vec<bool>, order: &mut Vec<&'a str>) {
        if visited[index] || !self.nodes[index].enabled {
            return;
        }
        visited[index] = true;

        // Visit dependencies first
        for dep_name in &self.nodes[index].dependencies {
            if let Some(&dep_idx) = self.name_to_index.get(dep_name.as_str()) {
                self.visit(dep_idx, visited, order);
            }
        }

        order.push(&self.nodes[index].name);
    }

    /// Get a pass node by name.
    pub fn get_pass(&self, name: impl AsRef<str>) -> Option<&RenderPassNode> {
        self.name_to_index
            .get(name.as_ref())
            .map(|&idx| &self.nodes[idx])
    }

    /// Number of passes.
    pub fn pass_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of enabled passes.
    pub fn enabled_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.enabled).count()
    }

    /// Create a standard forward rendering graph.
    pub fn forward() -> Self {
        let mut graph = Self::new();
        graph
            .add_pass("shadow", PassType::Shadow)
            .add_pass("geometry", PassType::Geometry)
            .add_dependency("geometry", "shadow")
            .add_pass("debug", PassType::Debug)
            .add_dependency("debug", "geometry")
            .add_pass("ui", PassType::Ui)
            .add_dependency("ui", "debug")
            .add_pass("post_process", PassType::PostProcess)
            .add_dependency("post_process", "ui");
        graph
    }

    /// Create a forward rendering graph with bloom + SSAO.
    pub fn forward_with_effects() -> Self {
        let mut graph = Self::new();
        graph
            .add_pass("shadow", PassType::Shadow)
            .add_pass("geometry", PassType::Geometry)
            .add_dependency("geometry", "shadow")
            .add_pass("ssao", PassType::Ssao)
            .add_dependency("ssao", "geometry")
            .add_pass("bloom", PassType::Bloom)
            .add_dependency("bloom", "geometry")
            .add_pass("debug", PassType::Debug)
            .add_dependency("debug", "geometry")
            .add_pass("ui", PassType::Ui)
            .add_dependency("ui", "debug")
            .add_pass("post_process", PassType::PostProcess)
            .add_dependency("post_process", "ssao")
            .add_dependency("post_process", "bloom")
            .add_dependency("post_process", "ui");
        graph
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let graph = RenderGraph::new();
        assert_eq!(graph.pass_count(), 0);
        assert!(graph.execution_order().is_empty());
    }

    #[test]
    fn single_pass() {
        let mut graph = RenderGraph::new();
        graph.add_pass("geometry", PassType::Geometry);
        assert_eq!(graph.execution_order(), vec!["geometry"]);
    }

    #[test]
    fn dependency_ordering() {
        let mut graph = RenderGraph::new();
        graph
            .add_pass("post", PassType::PostProcess)
            .add_pass("geometry", PassType::Geometry)
            .add_pass("shadow", PassType::Shadow)
            .add_dependency("geometry", "shadow")
            .add_dependency("post", "geometry");

        let order = graph.execution_order();
        let shadow_pos = order.iter().position(|&p| p == "shadow").unwrap();
        let geo_pos = order.iter().position(|&p| p == "geometry").unwrap();
        let post_pos = order.iter().position(|&p| p == "post").unwrap();
        assert!(shadow_pos < geo_pos);
        assert!(geo_pos < post_pos);
    }

    #[test]
    fn disable_pass() {
        let mut graph = RenderGraph::new();
        graph
            .add_pass("shadow", PassType::Shadow)
            .add_pass("geometry", PassType::Geometry);
        graph.set_enabled("shadow", false);
        assert_eq!(graph.enabled_count(), 1);
        let order = graph.execution_order();
        assert!(!order.contains(&"shadow"));
        assert!(order.contains(&"geometry"));
    }

    #[test]
    fn forward_graph() {
        let graph = RenderGraph::forward();
        let order = graph.execution_order();
        assert_eq!(order[0], "shadow");
        assert_eq!(order[1], "geometry");
        assert_eq!(order.len(), 5);
    }

    #[test]
    fn forward_with_effects() {
        let graph = RenderGraph::forward_with_effects();
        let order = graph.execution_order();
        // shadow first, geometry second, then ssao/bloom/debug in some order, post_process last
        assert_eq!(order[0], "shadow");
        assert_eq!(order[1], "geometry");
        assert_eq!(*order.last().unwrap(), "post_process");
        assert_eq!(order.len(), 7);
    }

    #[test]
    fn get_pass() {
        let graph = RenderGraph::forward();
        let pass = graph.get_pass("geometry").unwrap();
        assert_eq!(pass.pass_type, PassType::Geometry);
        assert!(pass.enabled);
    }

    #[test]
    fn pass_type_values() {
        assert_ne!(PassType::Shadow, PassType::Geometry);
        assert_ne!(PassType::Bloom, PassType::Ssao);
    }
}
