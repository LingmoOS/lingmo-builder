mod app;
mod commands;

fn main() {
    if let Err(e) = app::run() {
        eprintln!("Error: {}", e);

        // Print error chain
        let mut source: Option<&dyn std::error::Error> = e.source();
        while let Some(err) = source {
            eprintln!("  caused by: {}", err);
            source = err.source();
        }

        std::process::exit(1);
    }
}
