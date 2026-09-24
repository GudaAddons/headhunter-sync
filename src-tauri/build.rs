fn main() {
    println!("cargo:rerun-if-env-changed=HEADHUNTER_BUILD");
    println!("cargo:rerun-if-env-changed=HEADHUNTER_API_URL");

    if std::env::var("HEADHUNTER_BUILD").as_deref() == Ok("prod") {
        match std::env::var("HEADHUNTER_API_URL") {
            Ok(url) if url.starts_with("https://") => {}
            _ => panic!(
                "The production build needs HEADHUNTER_API_URL set to the website's https:// address. \
                 There is no production website yet; use `npm run build:local` for now."
            ),
        }
    }

    tauri_build::build()
}
