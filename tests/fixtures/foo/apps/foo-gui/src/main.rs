mod app;
mod capture;
mod cli;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match cli::Cli::parse(args.iter().cloned()) {
        Ok(cli) => cli,
        Err(reason) => {
            if args.iter().any(|a| a.starts_with("--capture=")) {
                fail_capture(&reason);
            }
            eprintln!("{reason}");
            std::process::exit(2);
        }
    };
    let capturing = cli.capture.is_some();
    let title = format!("Foo {}", env!("BK_VERSION_FULL"));
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size(if capturing {
                capture::WINDOW
            } else {
                [480.0, 240.0]
            }),
        ..Default::default()
    };
    let outcome = capture::Outcome::default();
    let capture_outcome = outcome.clone();
    let window = eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            // The same pixels on every machine: a zoomed display would change the screenshot.
            cc.egui_ctx.set_zoom_factor(1.0);
            let mut app = app::App::new(cli.screen);
            if let Some(path) = cli.capture {
                app = app.with_capture(capture::Run::new(path, capture_outcome));
            }
            Ok(Box::new(app))
        }),
    );
    if capturing {
        if let Err(reason) = capture::finish(window.map_err(|e| e.to_string()), &outcome) {
            fail_capture(&reason);
        }
        return Ok(());
    }
    window
}

/// `cargo bk manual capture` reads this line, so every capture failure uses it.
fn fail_capture(reason: &str) -> ! {
    eprintln!("capture failed: {reason}");
    std::process::exit(1);
}
