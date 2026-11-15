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
    let workload = perf::Workload::from_args(&args)?;
    let reference = reference::from_args(&args)?;
    let smoke = args.iter().any(|arg| arg == "--smoke-test");
    let trace_frames = args.iter().any(|arg| arg == "--trace-frames");
    let mut app: Option<Box<App>> = None;
    let (width, height) = if cfg!(target_os = "ios") {
        (390, 778)
    } else {
        (1440, 872)
    };
    pxui_sdl::app::run(
        "LocalWave",
        width,
        height,
        Box::new(move |window, signal| {
            if signal == Signal::Initialize {
                app = Some(app::open(window, workload, smoke, trace_frames, reference.clone())?);
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

fn main() {
    if let Err(error) = run() {
        eprintln!("LocalWave: {error}");
        std::process::exit(1);
    }
}
