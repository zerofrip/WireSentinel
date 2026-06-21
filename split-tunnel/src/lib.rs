//! Global split-tunnel template management and resolution.

mod resolution;
mod templates;

pub use resolution::TemplateResolver;
pub use shared_types::ResolvedTemplate;
pub use templates::SplitTunnelTemplateManager;
