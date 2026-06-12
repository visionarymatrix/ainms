use agent_collectors::installed_apps::scan_installed_apps;

fn main() {
    let apps = scan_installed_apps();
    println!("Found {} desktop apps:\n", apps.len());
    for app in &apps {
        println!("  {} | {} | {} | {:?}", app.app_name, app.display_name, app.publisher, app.install_path);
    }
}