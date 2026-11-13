//! Optional Instruments intervals; IDs cross FFI, and a scope always closes its interval.
unsafe extern "C" {
    fn lw_frame_interval_begin(frame: u64) -> u64;
    fn lw_frame_interval_end(interval: u64, wall: u64, cpu: u64);
}

pub struct FrameInterval {
    id: u64,
    wall: u64,
    cpu: u64,
}
impl FrameInterval {
    pub fn begin(frame: u64) -> Self {
        Self {
            id: unsafe { lw_frame_interval_begin(frame) },
            wall: 0,
            cpu: 0,
        }
    }
    pub fn finish(mut self, wall_us: u64, cpu_us: u64) {
        self.wall = wall_us;
        self.cpu = cpu_us;
    }
}
impl Drop for FrameInterval {
    fn drop(&mut self) {
        unsafe { lw_frame_interval_end(self.id, self.wall, self.cpu) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_frame_intervals_allocate_no_warm_rust_storage() {
        drop(FrameInterval::begin(0));
        mem_sys::local_reset();
        for frame in 1..101 {
            drop(FrameInterval::begin(frame));
        }
        assert_eq!(mem_sys::local_alloc_count(), 0);
    }
}
