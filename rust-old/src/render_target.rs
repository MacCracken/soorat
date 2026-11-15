//! Offscreen render targets.
//!
//! Re-exports [`mabda::render_target`] types which provide render targets with
//! optional MSAA and depth attachments via [`RenderTargetBuilder`].

pub use mabda::render_target::{RenderTarget, RenderTargetBuilder};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_target_size() {
        // Verify the type exists and is accessible
        let _size = std::mem::size_of::<RenderTarget>();
    }

    #[test]
    fn builder_type_exists() {
        let _size = std::mem::size_of::<RenderTargetBuilder>();
    }
}
