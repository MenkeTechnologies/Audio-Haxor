//! Generate an empty Ableton Live Set for testing using the embedded template

use app_lib::als_generator::generate_empty_als;
use std::path::Path;

fn main() {
    let output_path = Path::new("/Users/wizard/Desktop/Empty_Project.als");

    match generate_empty_als(output_path) {
        Ok(()) => {
            println!("Generated: {}", output_path.display());
            println!("Open in Ableton Live to verify.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
