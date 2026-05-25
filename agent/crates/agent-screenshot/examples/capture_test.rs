use std::io::Write;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let commander = agent_screenshot::ScreenshotCommander::new();
    match commander.capture().await {
        Ok(data) => {
            let path = "screenshot_test.png";
            let mut file = std::fs::File::create(path).expect("Failed to create file");
            file.write_all(&data).expect("Failed to write PNG data");
            println!("Screenshot captured! {} bytes written to {}", data.len(), path);

            // Verify it's a valid PNG by checking the header
            if data.len() >= 8 && &data[0..4] == b"\x89PNG" {
                println!("Verified: File is a valid PNG");
            } else {
                println!("WARNING: File does not appear to be a valid PNG");
            }

            // Verify nonzero pixel data (screen shouldn't be all black/transparent)
            let nonzero_pixels: usize = data[8..].iter().filter(|&&b| b != 0).count();
            println!("Non-zero bytes in image data: {}/{}", nonzero_pixels, data.len() - 8);
        }
        Err(e) => {
            eprintln!("Failed to capture screenshot: {}", e);
            std::process::exit(1);
        }
    }
}