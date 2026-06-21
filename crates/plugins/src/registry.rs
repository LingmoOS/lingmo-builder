use std::collections::HashMap;

use lingmo_core_engine::error::{BuildError, BuildResult};

use crate::builtins;
use crate::trait_def::Plugin;

/// Central registry that holds all available plugins and resolves them
/// by name based on build configuration.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    index: HashMap<String, usize>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let mut registry = PluginRegistry {
            plugins: vec![],
            index: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        let builtins: Vec<Box<dyn Plugin>> = vec![
            Box::new(builtins::core::CorePlugin),
            Box::new(builtins::kde::KdePlugin),
            Box::new(builtins::gnome::GnomePlugin),
            Box::new(builtins::xfce::XfcePlugin),
            Box::new(builtins::networkmanager::NetworkManagerPlugin),
            Box::new(builtins::nvidia::NvidiaPlugin),
            Box::new(builtins::minimal::MinimalPlugin),
        ];

        for plugin in builtins {
            self.register(plugin);
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        if !self.index.contains_key(&name) {
            self.index.insert(name, self.plugins.len());
            self.plugins.push(plugin);
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.index.get(name).map(|&i| self.plugins[i].as_ref())
    }

    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    pub fn resolve(
        &self,
        names: &[String],
    ) -> BuildResult<Vec<&dyn Plugin>> {
        let mut resolved: Vec<&dyn Plugin> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for name in names {
            self.resolve_with_deps(name, &mut resolved, &mut seen)?;
        }

        // Sort by priority (descending) at each dependency level
        // For simplicity, stable sort by priority
        resolved.sort_by(|a, b| {
            b.priority().cmp(&a.priority())
        });

        Ok(resolved)
    }

    fn resolve_with_deps<'a>(
        &'a self,
        name: &str,
        resolved: &mut Vec<&'a dyn Plugin>,
        seen: &mut std::collections::HashSet<String>,
    ) -> BuildResult<()> {
        if seen.contains(name) {
            return Ok(());
        }

        let plugin = self
            .get(name)
            .ok_or_else(|| BuildError::Plugin {
                plugin: name.to_string(),
                detail: "Plugin not found in registry".into(),
            })?;

        seen.insert(name.to_string());

        // Resolve dependencies first
        for dep_name in plugin.dependencies() {
            self.resolve_with_deps(dep_name, resolved, seen)?;
        }

        resolved.push(plugin);
        Ok(())
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|p| PluginInfo {
                name: p.name().to_string(),
                description: p.description().to_string(),
                dependencies: p.dependencies().iter().map(|s| s.to_string()).collect(),
                package_count: p.packages().len(),
                volume_group: p.volume_group().map(|s| s.to_string()),
                volume_prefixes: p.volume_prefixes().iter().map(|s| s.to_string()).collect(),
                required_volumes: p.required_volumes().iter().map(|s| s.to_string()).collect(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub package_count: usize,
    pub volume_group: Option<String>,
    pub volume_prefixes: Vec<String>,
    pub required_volumes: Vec<String>,
}
