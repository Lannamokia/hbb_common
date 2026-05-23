// vhd-machine-auth-bridge §1.2b — sanity check that the Build_Prereq_Vars
// gate's compile-time injection actually flows into `RS_PUB_KEY` /
// `RENDEZVOUS_SERVERS` / `RELAY_SERVER_DEFAULT` when `secret.sec` is present
// at the workspace root.
//
// We don't assert specific values (operators rotate them) — just that, when
// the workspace-root `secret.sec` exists, the consts are not the legacy
// hard-coded defaults.  When `secret.sec` is *absent*, the legacy defaults
// apply and this test silently skips.

use hbb_common::config;

const LEGACY_RS_PUB_KEY: &str = "OeVuKk5nlHiXp+APNn0Y3pC1Iwpwn44JGqrQCsWqmBw=";
const LEGACY_RENDEZVOUS_SERVER: &str = "rs-ny.rustdesk.com";

fn workspace_secret_sec() -> Option<std::path::PathBuf> {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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

#[test]
fn rs_pub_key_is_overridden_when_secret_sec_present() {
    if workspace_secret_sec().is_none() {
        eprintln!("secret.sec absent — legacy defaults apply, test skipped");
        return;
    }
    // The build script must have emitted RUSTDESK_RS_PUB_KEY.  We don't
    // reveal the actual value (PII / secret hygiene) — only assert it's
    // *different* from the legacy default.
    assert_ne!(
        config::RS_PUB_KEY, LEGACY_RS_PUB_KEY,
        "secret.sec is present at workspace root, so RS_PUB_KEY must be the \
         injected HBBS_KEY, not the legacy hard-coded value"
    );
}

#[test]
fn rendezvous_server_default_is_overridden_when_secret_sec_present() {
    if workspace_secret_sec().is_none() {
        eprintln!("secret.sec absent — legacy defaults apply, test skipped");
        return;
    }
    assert_eq!(config::RENDEZVOUS_SERVERS.len(), 1);
    assert_ne!(
        config::RENDEZVOUS_SERVERS[0], LEGACY_RENDEZVOUS_SERVER,
        "secret.sec is present so RENDEZVOUS_SERVERS[0] must be HBBS_HOST"
    );
}

#[test]
fn relay_server_default_is_set_when_secret_sec_present() {
    if workspace_secret_sec().is_none() {
        eprintln!("secret.sec absent — legacy defaults apply, test skipped");
        return;
    }
    assert!(
        !config::RELAY_SERVER_DEFAULT.is_empty(),
        "secret.sec is present so RELAY_SERVER_DEFAULT must be HBBR_HOST"
    );
}
