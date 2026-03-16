/// WASM sandbox configuration: resource limits, allowed syscalls.
///
/// This is a placeholder for Phase 7 implementation.

pub struct SandboxConfig {
    pub max_memory_bytes: usize,
    pub max_execution_time_secs: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024, // 256MB
            max_execution_time_secs: 30,
        }
    }
}
