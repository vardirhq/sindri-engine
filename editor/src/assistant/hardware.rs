//! How much memory a model could have on this machine.
//!
//! Best-effort throughout: every question is asked of a tool or file the
//! platform already has, and a question nobody could answer is `None` rather
//! than zero — an undetectable GPU is common, and "could not tell" must not read
//! as "nothing fits".

/// Memory available to a model, in gigabytes.
///
/// Video memory where a discrete GPU can be read; on Apple silicon, the unified
/// memory the GPU shares; otherwise what the system has spare, for a model that
/// will run on the CPU.
pub fn available_memory() -> Option<f32> {
    video_memory()
        .or_else(unified_memory)
        .or_else(system_memory)
}

/// Video memory, asked of whichever vendor tool is present.
///
/// Both vendors, because asking only NVIDIA means every Radeon machine falls
/// silently through to system memory and gets told it can run less than it can.
fn video_memory() -> Option<f32> {
    nvidia_memory().or_else(amd_memory)
}

fn nvidia_memory() -> Option<f32> {
    let text = run(
        "nvidia-smi",
        &["--query-gpu=memory.total", "--format=csv,noheader,nounits"],
    )?;
    let megabytes: f32 = text.lines().next()?.trim().parse().ok()?;
    Some(megabytes / 1024.0)
}

/// The largest AMD card, in gigabytes.
///
/// `rocm-smi` reports in bytes and one line per card; the largest is taken
/// rather than the sum, because a model runs on one card unless it has been
/// deliberately split across several.
fn amd_memory() -> Option<f32> {
    let text = run("rocm-smi", &["--showmeminfo", "vram", "--csv"])?;
    text.lines()
        .filter_map(|line| line.rsplit(',').next())
        .filter_map(|field| field.trim().parse::<f32>().ok())
        .map(|bytes| bytes / 1_073_741_824.0)
        .filter(|gigabytes| *gigabytes > 0.0)
        .max_by(f32::total_cmp)
}

/// On a Mac with Apple silicon the GPU works from system memory, and about
/// three quarters of it is what the system lets the GPU hold.
fn unified_memory() -> Option<f32> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return None;
    }
    let bytes: f32 = run("sysctl", &["-n", "hw.memsize"])?.trim().parse().ok()?;
    Some(bytes / 1_073_741_824.0 * 0.75)
}

/// What the system has spare, for a model running on its CPU.
fn system_memory() -> Option<f32> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = meminfo
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?;
    let kilobytes: f32 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kilobytes / 1_048_576.0)
}

/// One command, or nothing when it is absent or unhappy.
fn run(program: &str, arguments: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine that can answer none of the questions answers `None` or a
    /// positive figure, never zero or a negative one.
    #[test]
    fn memory_is_either_unknown_or_positive() {
        if let Some(gigabytes) = available_memory() {
            assert!(gigabytes > 0.0, "{gigabytes}");
        }
    }
}
