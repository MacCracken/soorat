//! Frame profiling — CPU timing and GPU timestamp queries.
//!
//! Re-exported from [`mabda`] — the shared GPU foundation.

pub use mabda::profiler::{FrameProfiler, GpuTimestamps, PassTiming, ProfileScope};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiler_default() {
        let p = FrameProfiler::new();
        assert_eq!(p.frame_count, 0);
        assert!((p.fps - 60.0).abs() < 0.1);
    }

    #[test]
    fn profiler_begin_end() {
        let mut p = FrameProfiler::new();
        p.begin_frame();
        let ms = p.end_frame();
        assert!(ms >= 0.0);
        assert_eq!(p.frame_count, 1);
    }

    #[test]
    fn profiler_fps_updates() {
        let mut p = FrameProfiler::new();
        for _ in 0..100 {
            p.begin_frame();
            p.end_frame();
        }
        assert!(p.fps > 0.0);
        assert_eq!(p.frame_count, 100);
    }

    #[test]
    fn profiler_reset() {
        let mut p = FrameProfiler::new();
        p.begin_frame();
        p.end_frame();
        p.reset();
        assert_eq!(p.frame_count, 0);
    }

    #[test]
    fn profiler_end_without_begin() {
        let mut p = FrameProfiler::new();
        let ms = p.end_frame();
        assert_eq!(ms, 0.0);
    }

    #[test]
    fn profiler_record_pass() {
        let mut p = FrameProfiler::new();
        p.begin_frame();
        p.record_pass("shadow", 0.5);
        p.record_pass("pbr", 2.0);
        p.record_pass("post", 0.3);
        assert_eq!(p.pass_times.len(), 3);
        assert!((p.total_pass_time_ms() - 2.8).abs() < 0.001);
    }

    #[test]
    fn profiler_begin_clears_passes() {
        let mut p = FrameProfiler::new();
        p.record_pass("test", 1.0);
        p.begin_frame();
        assert!(p.pass_times.is_empty());
    }

    #[test]
    fn pass_timing_fields() {
        let t = PassTiming {
            label: "shadow".into(),
            duration_ms: 1.5,
        };
        assert_eq!(t.label, "shadow");
        assert_eq!(t.duration_ms, 1.5);
    }
}
