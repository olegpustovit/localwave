//! Entry point; application state and performance qualification have separate owners.
mod app;
mod events;
mod gpu;
mod perf;
mod reference;
mod smoke;
use app::App;
use pxui_sdl::app::{Schedule, Signal};

extern "C" fn wake() {
    pxui_sdl::wake();
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    known_options(&args)?;
    let workload = perf::Workload::from_args(&args)?;
    let reference = reference::from_args(&args)?;
    let energy = perf::energy::Scenario::from_args(&args)?;
    let smoke = args.iter().any(|arg| arg == "--smoke-test");
    if energy.is_some() && (workload.is_some() || reference.is_some() || smoke) {
        return Err("Energy runs require their own fixture".into());
    }
    let mut reference_capture = reference
        .as_ref()
        .and_then(|_| std::env::var_os("LOCALWAVE_REFERENCE_CAPTURE"))
        .map(std::path::PathBuf::from);
    let trace_frames = args.iter().any(|arg| arg == "--trace-frames");
    let mut app: Option<Box<App>> = None;
    let (width, height) = if cfg!(target_os = "ios") {
        (390, 778)
    } else {
        (1440, 872)
    };
    pxui_sdl::app::run_with_density(
        "LocalWave",
        width,
        height,
        reference_capture.is_none(),
        Box::new(move |window, signal| {
            if signal == Signal::Initialize {
                app = Some(app::open(
                    window, workload, energy, smoke, trace_frames, reference.clone(), reference_capture.take(),
                )?);
                return Ok(Schedule::NextFrame);
            }
            if signal == Signal::Finish {
                if let Some(mut app) = app.take() {
                    app.frame(window, signal)?;
                }
                return Ok(Schedule::Exit);
            }
            app.as_mut()
                .ok_or("LocalWave was not initialized")?
                .frame(window, signal)
        }),
    )
}

/// A mistyped or unsupported development flag must not fall through to the user's real library.
/// Single-dash arguments (`-psn_…`, `-NSDocumentRevisionsDebugMode`) belong to the system.
fn known_options(args: &[String]) -> Result<(), String> {
    let mut value = false;
    for arg in args.iter().skip(1) {
        if std::mem::take(&mut value) {
            continue;
        }
        match arg.as_str() {
            "--performance-check" | "--smoke-test" | "--trace-frames" => {}
            "--reference-screen" => value = true,
            // Validated by Workload::from_args.
            option if option.starts_with("--performance-") || option.starts_with("--energy-") => {}
            option if option.starts_with("--") => return Err(format!("Unknown option: {option}")),
            _ => {}
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("LocalWave: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_double_dash_options_are_rejected_before_any_library_opens() {
        let args = |list: &[&str]| list.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert!(super::known_options(&args(&["localwave"])).is_ok());
        assert!(super::known_options(&args(&["localwave", "-psn_0_1234", "-NSDocumentRevisionsDebugMode", "YES"])).is_ok());
        assert!(super::known_options(&args(&["localwave", "--performance-check", "--trace-frames"])).is_ok());
        assert!(super::known_options(&args(&["localwave", "--reference-screen", "--songs"])).is_ok());
        assert!(super::known_options(&args(&["localwave", "--smoke-tset"])).is_err());
        assert!(super::known_options(&args(&["localwave", "--snapshot", "x.png"])).is_err());
    }
}
