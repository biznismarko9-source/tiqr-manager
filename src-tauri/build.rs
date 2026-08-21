use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Embeds the Google service-account credentials (Settings -> Integrations
    // -> Google Sheets sync) into the compiled binary at build time, so the
    // key never has to exist as a loose file next to the installed app, and
    // never has to be a tracked file in this repository - which is public
    // (see tauri.conf.json's unauthenticated updater endpoint), so anything
    // committed here is world-readable forever, even after a later commit
    // "removes" it.
    //
    // The real value is injected only by GitHub Actions, from the repository
    // secret GOOGLE_SERVICE_ACCOUNT_JSON (see
    // .github/workflows/build-windows.yml - both the quick unsigned test
    // build and the real signed release set it the same way). A plain local
    // `cargo build`/`cargo test`/`npm run tauri dev` never has it set, and
    // that is fine on purpose: this script falls back to an empty string
    // rather than failing, and google_sheets::embedded_service_account()
    // treats an empty embed as "sync isn't configured in this build" rather
    // than panicking - see that function's doc comment.
    let key_json = env::var("GOOGLE_SERVICE_ACCOUNT_JSON").unwrap_or_default();
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is always set by Cargo while running a build script");
    let dest = Path::new(&out_dir).join("google_service_account.json");
    fs::write(&dest, key_json).expect("failed to write the embedded Google service account file");
    // Re-embed whenever the secret itself changes, not just when source
    // files change (a plain `cargo build` wouldn't otherwise notice).
    println!("cargo:rerun-if-env-changed=GOOGLE_SERVICE_ACCOUNT_JSON");

    // 2.0.5: same embed-at-build-time mechanism as above, for the OAuth
    // client "Sign in with Google" uses (google_oauth.rs) - a completely
    // separate credential from the service account, registered separately in
    // Cloud Console (OAuth client ID, type "Desktop app"), injected from two
    // more GitHub Actions repository secrets: GOOGLE_OAUTH_CLIENT_ID and
    // GOOGLE_OAUTH_CLIENT_SECRET. Google documents a Desktop app's
    // client_secret as not needing to be kept confidential the way a service
    // account private key does (PKCE is what actually secures this flow -
    // see google_oauth.rs's module doc comment) - it is still never
    // committed to this repository, for the same "public repo, anything
    // committed is world-readable forever" reason as the service account
    // above. Two plain-text files, not JSON: each holds exactly one opaque
    // value end to end, so there is nothing to escape and no need to pull
    // serde_json into this build script's own dependency graph just for it.
    let oauth_client_id = env::var("GOOGLE_OAUTH_CLIENT_ID").unwrap_or_default();
    let oauth_client_secret = env::var("GOOGLE_OAUTH_CLIENT_SECRET").unwrap_or_default();
    fs::write(Path::new(&out_dir).join("google_oauth_client_id.txt"), oauth_client_id)
        .expect("failed to write the embedded Google OAuth client ID file");
    fs::write(Path::new(&out_dir).join("google_oauth_client_secret.txt"), oauth_client_secret)
        .expect("failed to write the embedded Google OAuth client secret file");
    println!("cargo:rerun-if-env-changed=GOOGLE_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_OAUTH_CLIENT_SECRET");

    tauri_build::build()
}
