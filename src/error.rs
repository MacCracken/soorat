//! Error types for soorat.

use thiserror::Error;

/// Errors produced by soorat.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    #[error("GPU adapter not found")]
    AdapterNotFound,

    #[error("GPU device request failed: {0}")]
    DeviceRequest(String),

    #[error("surface configuration failed: {0}")]
    SurfaceConfig(String),

    #[error("surface texture acquisition failed: {0}")]
    SurfaceTexture(String),

    #[error("shader compilation failed: {0}")]
    Shader(String),

    #[error("pipeline creation failed: {0}")]
    Pipeline(String),

    #[error("texture load failed: {0}")]
    Texture(String),

    #[error("model load failed: {0}")]
    Model(String),

    #[error("window creation failed: {0}")]
    Window(String),

    #[error("screenshot failed: {0}")]
    Screenshot(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, RenderError>;

impl From<mabda::GpuError> for RenderError {
    fn from(e: mabda::GpuError) -> Self {
        match e {
            mabda::GpuError::AdapterNotFound => Self::AdapterNotFound,
            mabda::GpuError::DeviceRequest(inner) => Self::DeviceRequest(inner.to_string()),
            mabda::GpuError::SurfaceConfig(msg) => Self::SurfaceConfig(msg),
            mabda::GpuError::Shader(msg) => Self::Shader(msg),
            mabda::GpuError::Pipeline(msg) => Self::Pipeline(msg),
            other => Self::Other(other.to_string().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = RenderError::AdapterNotFound;
        assert_eq!(err.to_string(), "GPU adapter not found");
    }

    #[test]
    fn error_variants() {
        let errors = vec![
            RenderError::AdapterNotFound,
            RenderError::DeviceRequest("test".into()),
            RenderError::SurfaceConfig("test".into()),
            RenderError::SurfaceTexture("test".into()),
            RenderError::Shader("test".into()),
            RenderError::Pipeline("test".into()),
            RenderError::Texture("test".into()),
            RenderError::Model("test".into()),
            RenderError::Window("test".into()),
        ];
        for err in &errors {
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn error_other_variant() {
        let inner: Box<dyn std::error::Error + Send + Sync> = "custom error".into();
        let err = RenderError::Other(inner);
        assert!(err.to_string().contains("custom error"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<RenderError>();
        assert_sync::<RenderError>();
    }
}
