//! Renders one fixed scene for a number of frames and reports how long the
//! CPU and each GPU pass took: the render benchmark.
//!
//! ```text
//! render_bench <helmet|shadows|instances|crowd> [--frames N] [--warmup N]
//!              [--model file.glb] [--json out.json] [--fps N] [--animation N] [--dump DIR]
//! ```
//!
//! `helmet` and `crowd` load a glTF file (`--model`, else
//! `assets/models/DamagedHelmet.glb` and `assets/models/CesiumMan.glb`);
//! `shadows` and `instances` are procedural. It renders offscreen on a
//! headless device (no window, no vsync) at 1920 x 1080 unless
//! `MERIDIAN_WINDOW=<width>x<height>` says otherwise; `--fps` is the scene
//! clock (default 60) and `--dump DIR` writes every measured frame as a
//! `.ppm` image, for encoding video.
//!
//! Design notes: docs/notes/meridian-examples.md (`examples/render_bench`).

mod app;
mod report;
mod scenes;

use scenes::SceneKind;

/// What the command line asked for.
struct Options {
    kind: SceneKind,
    frames: usize,
    warmup: usize,
    model: Option<String>,
    json: Option<String>,
    fps: f32,
    animation: usize,
    dump: Option<String>,
}

fn main() {
    meridian_examples::app_main::install_diagnostics("render_bench");
    let options = parse(std::env::args().skip(1)).unwrap_or_else(|message| {
        say(&mut std::io::stderr(), &message);
        std::process::exit(2)
    });
    let (width, height) = meridian_examples::app_main::window_size_or((1920, 1080));
    match app::run(&options, width, height) {
        Ok(report) => finish(&report, options.json.as_deref()),
        Err(message) => {
            say(&mut std::io::stderr(), &message);
            std::process::exit(1)
        }
    }
}

/// Prints the table, then the JSON as one line on stdout (and into `json`).
fn finish(report: &report::Report, json: Option<&str>) {
    say(&mut std::io::stderr(), &report.table());
    let line = report.json();
    say(&mut std::io::stdout(), &line);
    if let Some(path) = json
        && let Err(err) = std::fs::write(path, format!("{line}\n"))
    {
        say(
            &mut std::io::stderr(),
            &format!("cannot write {path}: {err}"),
        );
        std::process::exit(1);
    }
}

/// Writes `text` and a newline to `out`; a closed pipe is not worth failing over.
fn say(out: &mut impl std::io::Write, text: &str) {
    drop(writeln!(out, "{}", text.trim_end()));
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Options, String> {
    let usage = "usage: render_bench <helmet|shadows|instances|crowd> [--frames N] [--warmup N] [--model file.glb] [--json out.json] [--fps N] [--animation N] [--dump DIR]";
    let name = args.next().ok_or(usage)?;
    let kind = SceneKind::parse(&name).ok_or(usage)?;
    let mut options = Options {
        kind,
        frames: 300,
        warmup: 30,
        model: None,
        json: None,
        fps: 60.0,
        animation: 0,
        dump: None,
    };
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(usage)?;
        match flag.as_str() {
            "--frames" => options.frames = value.parse().map_err(|_| usage)?,
            "--warmup" => options.warmup = value.parse().map_err(|_| usage)?,
            "--model" => options.model = Some(value),
            "--json" => options.json = Some(value),
            "--fps" => options.fps = value.parse().map_err(|_| usage)?,
            "--animation" => options.animation = value.parse().map_err(|_| usage)?,
            "--dump" => options.dump = Some(value),
            _ => return Err(usage.to_string()),
        }
    }
    Ok(options)
}
