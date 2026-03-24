//! GPU frame profiling — timing queries for render passes.

use std::time::Instant;

/// Frame profiler — tracks CPU-side frame timing and optional GPU query results.
#[derive(Debug, Clone)]
pub struct FrameProfiler {
    frame_start: Option<Instant>,
    /// CPU-side frame time in milliseconds.
    pub cpu_frame_ms: f64,
    /// Rolling average of frame time (exponential moving average).
    pub avg_frame_ms: f64,
    /// Frames per second (computed from avg_frame_ms).
    pub fps: f64,
    /// Total frames counted.
    pub frame_count: u64,
    alpha: f64,
}

impl FrameProfiler {
    /// Create a new profiler with the given smoothing factor (0.0–1.0).
    /// Lower alpha = smoother. Default 0.05.
    pub fn new() -> Self {
        Self {
            frame_start: None,
            cpu_frame_ms: 0.0,
            avg_frame_ms: 16.67, // assume 60fps initially
            fps: 60.0,
            frame_count: 0,
            alpha: 0.05,
        }
    }

    /// Call at the start of each frame.
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }

    /// Call at the end of each frame. Returns the frame time in ms.
    pub fn end_frame(&mut self) -> f64 {
        let elapsed = self
            .frame_start
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        self.cpu_frame_ms = elapsed;
        self.avg_frame_ms = self.avg_frame_ms * (1.0 - self.alpha) + elapsed * self.alpha;
        self.fps = if self.avg_frame_ms > 0.0 {
            1000.0 / self.avg_frame_ms
        } else {
            0.0
        };
        self.frame_count += 1;
        self.frame_start = None;

        elapsed
    }

    /// Reset all counters.
    pub fn reset(&mut self) {
        self.cpu_frame_ms = 0.0;
        self.avg_frame_ms = 16.67;
        self.fps = 60.0;
        self.frame_count = 0;
    }
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self::new()
    }
}

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
}
