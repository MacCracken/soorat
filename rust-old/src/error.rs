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
            other => Self::Other(format!("{other:#}").into()),
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

    #[test]
    fn from_gpu_error_adapter_not_found() {
        let gpu_err = mabda::GpuError::AdapterNotFound;
        let render_err: RenderError = gpu_err.into();
        assert!(
            matches!(render_err, RenderError::AdapterNotFound),
            "AdapterNotFound should map to AdapterNotFound"
        );
    }

    #[test]
    fn from_gpu_error_surface_config() {
        let gpu_err = mabda::GpuError::SurfaceConfig("test config".into());
        let render_err: RenderError = gpu_err.into();
        match render_err {
            RenderError::SurfaceConfig(msg) => assert_eq!(msg, "test config"),
            other => panic!("expected SurfaceConfig, got {other:?}"),
        }
    }

    #[test]
    fn from_gpu_error_shader() {
        let gpu_err = mabda::GpuError::Shader("bad shader".into());
        let render_err: RenderError = gpu_err.into();
        match render_err {
            RenderError::Shader(msg) => assert_eq!(msg, "bad shader"),
            other => panic!("expected Shader, got {other:?}"),
        }
    }

    #[test]
    fn from_gpu_error_pipeline() {
        let gpu_err = mabda::GpuError::Pipeline("pipeline fail".into());
        let render_err: RenderError = gpu_err.into();
        match render_err {
            RenderError::Pipeline(msg) => assert_eq!(msg, "pipeline fail"),
            other => panic!("expected Pipeline, got {other:?}"),
        }
    }

    #[test]
    fn from_gpu_error_other_variants_fallback() {
        // Variants not explicitly matched should fall through to Other
        let gpu_err = mabda::GpuError::SurfaceTimeout;
        let render_err: RenderError = gpu_err.into();
        assert!(
            matches!(render_err, RenderError::Other(_)),
            "unmatched GpuError variants should map to Other"
        );

        let gpu_err = mabda::GpuError::SurfaceOutdated;
        let render_err: RenderError = gpu_err.into();
        assert!(matches!(render_err, RenderError::Other(_)));

        let gpu_err = mabda::GpuError::SurfaceLost;
        let render_err: RenderError = gpu_err.into();
        assert!(matches!(render_err, RenderError::Other(_)));

        let gpu_err = mabda::GpuError::ReadbackChannel;
        let render_err: RenderError = gpu_err.into();
        assert!(matches!(render_err, RenderError::Other(_)));
    }
}
