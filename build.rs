use std::path::PathBuf;

fn main() {
    let out_dir = format!("{}/protos", std::env::var("OUT_DIR").unwrap());

    std::fs::create_dir_all(&out_dir).unwrap();

    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(out_dir)
        .inputs(["protos/rendezvous.proto", "protos/message.proto"])
        .include("protos")
        .customize(protobuf_codegen::Customize::default().tokio_bytes(true))
        .run()
        .expect("Codegen failed.");

    inject_build_prereq_vars();
}

// vhd-machine-auth-bridge §1.2b: emit `cargo:rustc-env=RUSTDESK_RS_PUB_KEY` /
// `RUSTDESK_RENDEZVOUS_SERVER` / `RUSTDESK_RELAY_SERVER` so `src/config.rs`
// can pick them up via `option_env!` at compile time.  Cargo scopes
// `cargo:rustc-env` to the *current* crate, so this must run in
// `hbb_common`'s build script — not the workspace root's.
//
// Lenient on absence: when no env / no secret.sec is supplied, this function
// emits nothing and the legacy `OeVuKk...` / `rs-ny.rustdesk.com` defaults in
// `config.rs` apply unchanged.  The strict gate that fails the build on
// invalid input lives in the workspace root's `build.rs`; here we only
// re-resolve the same validated inputs to hand them to `rustc`.
fn inject_build_prereq_vars() {
    use build_support::{parse_secret_sec, resolve_build_prereq_vars, BuildPrereqInputs};

    println!("cargo:rerun-if-env-changed=HBBS_KEY");
    println!("cargo:rerun-if-env-changed=HBBS_HOST");
    println!("cargo:rerun-if-env-changed=HBBR_HOST");

    let hbbs_key_env = std::env::var("HBBS_KEY").ok();
    let hbbs_host_env = std::env::var("HBBS_HOST").ok();
    let hbbr_host_env = std::env::var("HBBR_HOST").ok();

    // Walk up from CARGO_MANIFEST_DIR to locate secret.sec at the workspace
    // root (CARGO_WORKSPACE_DIR is not stable, so we discover it by walking
    // up — cheap, deterministic, and friendly to vendored / sub-tree builds).
    let secret_sec_path = find_secret_sec();
    if let Some(ref p) = secret_sec_path {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let sec_map = secret_sec_path
        .as_deref()
        .map(parse_secret_sec)
        .unwrap_or_default();

    let any_env_set =
        hbbs_key_env.is_some() || hbbs_host_env.is_some() || hbbr_host_env.is_some();
    let secret_sec_present = secret_sec_path.is_some();
    if !(any_env_set || secret_sec_present) {
        // No injection attempt — keep legacy defaults from config.rs.
        return;
    }

    let inputs = BuildPrereqInputs {
        hbbs_key_env: hbbs_key_env.as_deref(),
        hbbs_host_env: hbbs_host_env.as_deref(),
        hbbr_host_env: hbbr_host_env.as_deref(),
        sec_map: &sec_map,
    };
    let values = match resolve_build_prereq_vars(&inputs) {
        Ok(v) => v,
        Err(_) => {
            // The workspace-root `build.rs` already runs the same resolver
            // and exits non-zero on the same `Err`, so we silently bail out
            // here — the operator gets a single, unified error surface.
            // hbb_common is built before the root crate, so root's gate
            // hasn't fired yet at this point; if the user is building
            // hbb_common standalone (e.g. `cargo test -p hbb_common`) the
            // most useful behavior is to keep the legacy defaults rather
            // than fail the package's tests over an unrelated injection
            // problem.  The strict gate is at the workspace level.
            return;
        }
    };

    // Emit env vars consumed by `option_env!` in src/config.rs.
    println!(
        "cargo:rustc-env=RUSTDESK_RS_PUB_KEY={}",
        values.hbbs_key_b64_canonical
    );
    println!(
        "cargo:rustc-env=RUSTDESK_RENDEZVOUS_SERVER={}",
        values.hbbs_host
    );
    println!("cargo:rustc-env=RUSTDESK_RELAY_SERVER={}", values.hbbr_host);
}

fn find_secret_sec() -> Option<PathBuf> {
    let mut dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    loop {
        let candidate = dir.join("secret.sec");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}
