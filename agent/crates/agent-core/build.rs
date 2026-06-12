fn main() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(current_dir) = std::env::current_dir() {
            let src_manifest = current_dir.join("agent-core.exe.manifest");

            // Copy the manifest file to the workspace `target` directory
            // relative to the workspace root, so it has a space-free path: "target/agent-core.exe.manifest"
            let target_dir = current_dir.parent().unwrap().parent().unwrap().join("target");
            let dest_manifest = target_dir.join("agent-core.exe.manifest");

            let _ = std::fs::create_dir_all(&target_dir);
            if src_manifest.exists() {
                let _ = std::fs::copy(&src_manifest, &dest_manifest);
            }

            // Tell the MSVC linker to natively embed the manifest using the relative target path
            println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
            println!("cargo:rustc-link-arg=/MANIFESTINPUT:target/agent-core.exe.manifest");
        }
    }
}
