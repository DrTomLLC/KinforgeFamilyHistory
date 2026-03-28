use kinforge_core::KinforgeResult;

/// Events that Kinforge emits to plugins when notable database operations occur.
#[derive(Debug, Clone)]
pub enum PluginEvent {
    /// A new person was added to the database.
    PersonAdded { name: String },
    /// A new life event was recorded.
    EventAdded { event_type: String, person_name: String },
    /// A new relationship was recorded.
    RelationshipAdded { rel_type: String },
    /// A new research task was created.
    TaskAdded { description: String },
}

/// The core trait that all Kinforge plugins must implement.
pub trait KinforgePlugin: Send + Sync {
    /// Unique machine-readable identifier for this plugin.
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Version string.
    fn version(&self) -> &str;

    /// Called once when the plugin is loaded into the registry.
    fn on_load(&mut self) -> KinforgeResult<()> {
        Ok(())
    }

    /// Called once when the plugin is unloaded from the registry.
    fn on_unload(&mut self) -> KinforgeResult<()> {
        Ok(())
    }

    /// Called when a notable database event occurs. Default implementation is a no-op.
    fn on_event(&mut self, _event: &PluginEvent) {}
}

/// A registry that tracks loaded plugins and dispatches events to them.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn KinforgePlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin, calling its `on_load` hook.
    pub fn register(&mut self, mut plugin: Box<dyn KinforgePlugin>) -> KinforgeResult<()> {
        plugin.on_load()?;
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn plugins(&self) -> &[Box<dyn KinforgePlugin>] {
        &self.plugins
    }

    /// Notify all registered plugins of a database event.
    pub fn notify(&mut self, event: &PluginEvent) {
        for plugin in &mut self.plugins {
            plugin.on_event(event);
        }
    }

    /// Unload all plugins (calling `on_unload` on each) and clear the registry.
    pub fn unregister_all(&mut self) {
        for plugin in &mut self.plugins {
            let _ = plugin.on_unload();
        }
        self.plugins.clear();
    }
}
