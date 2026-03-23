use kinforge_core::KinforgeResult;

/// The core trait that all Kinforge plugins must implement.
pub trait KinforgePlugin: Send + Sync {
    /// Unique machine-readable identifier for this plugin.
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Version string.
    fn version(&self) -> &str;

    /// Called once when the plugin is loaded.
    fn on_load(&mut self) -> KinforgeResult<()> {
        Ok(())
    }

    /// Called once when the plugin is unloaded.
    fn on_unload(&mut self) -> KinforgeResult<()> {
        Ok(())
    }
}

/// A registry that tracks loaded plugins.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn KinforgePlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

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
}
