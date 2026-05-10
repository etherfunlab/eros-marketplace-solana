pub mod init_registries;
pub mod initialize;

pub use init_registries::InitRegistries;
pub use initialize::Initialize;

// Re-export handlers under namespaced aliases to avoid the ambiguous-glob-reexport warning.
pub use init_registries::handler as init_registries_handler;
pub use initialize::handler as initialize_handler;
