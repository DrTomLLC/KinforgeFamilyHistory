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

// ─── Built-in example plugins ────────────────────────────────────────────────

/// A plugin that prints every database event to stderr.
///
/// Useful for debugging and as a reference implementation.
///
/// # Example
/// ```rust
/// use kinforge_plugin_api::{PluginRegistry, plugins::ConsoleLogPlugin};
/// let mut registry = PluginRegistry::new();
/// registry.register(Box::new(ConsoleLogPlugin::new())).unwrap();
/// ```
pub mod plugins {
    use super::{KinforgePlugin, KinforgeResult, PluginEvent};

    /// Logs every plugin event to stderr.
    pub struct ConsoleLogPlugin {
        prefix: String,
    }

    impl ConsoleLogPlugin {
        pub fn new() -> Self {
            Self { prefix: "[kinforge]".to_string() }
        }

        pub fn with_prefix(prefix: impl Into<String>) -> Self {
            Self { prefix: prefix.into() }
        }
    }

    impl Default for ConsoleLogPlugin {
        fn default() -> Self { Self::new() }
    }

    impl KinforgePlugin for ConsoleLogPlugin {
        fn id(&self) -> &str { "builtin.console_log" }
        fn name(&self) -> &str { "Console Logger" }
        fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

        fn on_load(&mut self) -> KinforgeResult<()> {
            eprintln!("{} ConsoleLogPlugin loaded", self.prefix);
            Ok(())
        }

        fn on_unload(&mut self) -> KinforgeResult<()> {
            eprintln!("{} ConsoleLogPlugin unloaded", self.prefix);
            Ok(())
        }

        fn on_event(&mut self, event: &PluginEvent) {
            match event {
                PluginEvent::PersonAdded { name } =>
                    eprintln!("{} person added: {}", self.prefix, name),
                PluginEvent::EventAdded { event_type, person_name } =>
                    eprintln!("{} event added: {} for {}", self.prefix, event_type, person_name),
                PluginEvent::RelationshipAdded { rel_type } =>
                    eprintln!("{} relationship added: {}", self.prefix, rel_type),
                PluginEvent::TaskAdded { description } =>
                    eprintln!("{} task added: {}", self.prefix, description),
            }
        }
    }

    /// Counts how many of each event type were fired during its lifetime.
    ///
    /// Call `summary()` to retrieve the counts, or they are printed to stderr
    /// on `on_unload`.
    pub struct EventCounterPlugin {
        pub people_added: usize,
        pub events_added: usize,
        pub relationships_added: usize,
        pub tasks_added: usize,
    }

    impl EventCounterPlugin {
        pub fn new() -> Self {
            Self {
                people_added: 0,
                events_added: 0,
                relationships_added: 0,
                tasks_added: 0,
            }
        }

        /// Returns a human-readable summary string.
        pub fn summary(&self) -> String {
            format!(
                "EventCounter: {} people  {} events  {} relationships  {} tasks",
                self.people_added, self.events_added,
                self.relationships_added, self.tasks_added
            )
        }
    }

    impl Default for EventCounterPlugin {
        fn default() -> Self { Self::new() }
    }

    impl KinforgePlugin for EventCounterPlugin {
        fn id(&self) -> &str { "builtin.event_counter" }
        fn name(&self) -> &str { "Event Counter" }
        fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

        fn on_unload(&mut self) -> KinforgeResult<()> {
            eprintln!("{}", self.summary());
            Ok(())
        }

        fn on_event(&mut self, event: &PluginEvent) {
            match event {
                PluginEvent::PersonAdded { .. } => self.people_added += 1,
                PluginEvent::EventAdded { .. } => self.events_added += 1,
                PluginEvent::RelationshipAdded { .. } => self.relationships_added += 1,
                PluginEvent::TaskAdded { .. } => self.tasks_added += 1,
            }
        }
    }
}
