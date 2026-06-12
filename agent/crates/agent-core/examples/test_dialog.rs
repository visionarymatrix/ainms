use anyhow::Result;

fn main() -> Result<()> {
    // Initialize tracing so we can see log outputs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Testing Alert Dialog System ===");

    // 1. Test notification (notify)
    println!("\n1. Testing notify dialog...");
    agent_core::dialog::notify("Notification Test", "This is a test notification message. Press OK to close.")?;
    println!("Notification closed.");

    // 2. Test binary question (ask)
    println!("\n2. Testing ask dialog...");
    match agent_core::dialog::ask("Question Test", "Do you like coding in Rust?")? {
        agent_core::dialog::DialogAnswer::Yes => println!("User selected: Yes"),
        agent_core::dialog::DialogAnswer::No => println!("User selected: No"),
    }

    // 3. Test input prompt (prompt)
    println!("\n3. Testing prompt dialog...");
    let prompt_res = agent_core::dialog::prompt("Prompt Test", "What is your favorite programming language?")?;
    match prompt_res.text {
        Some(text) => println!("User entered: '{}'", text),
        None => println!("User dismissed/cancelled the prompt."),
    }

    println!("\n=== Dialog tests completed! ===");
    Ok(())
}
