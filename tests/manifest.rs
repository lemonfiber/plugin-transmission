//! The manifest, as the core at the pinned commit reads it.

#[test]
fn the_pinned_core_refuses_nothing_in_the_manifest() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml"))
        .unwrap_or_default();
    let read: Result<lemonfiber_plugin::Manifest, _> = toml::from_str(&text);
    let refused: Vec<String> = match read {
        Ok(manifest) => lemonfiber_plugin::refusals(&manifest, &[])
            .into_iter()
            .map(|violation| format!("{}: {}", violation.location, violation.message))
            .collect(),
        Err(why) => vec![why.to_string()],
    };
    assert!(refused.is_empty(), "{refused:#?}");
}

#[test]
fn transmission_reads_its_rpc_account_from_files_holding_only_the_minted_secrets() {
    let root = env!("CARGO_MANIFEST_DIR");
    let manifest: toml::Table = std::fs::read_to_string(format!("{root}/plugin.toml"))
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default();
    let env = manifest
        .get("service")
        .and_then(toml::Value::as_array)
        .and_then(|services| {
            services
                .iter()
                .find(|one| one.get("id").and_then(toml::Value::as_str) == Some("transmission"))
        })
        .and_then(|service| service.get("env"))
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();
    let mut read: Vec<(String, Option<String>, Option<String>)> = env
        .iter()
        .map(|(variable, value)| {
            let file = value.as_str().map(str::to_owned);
            let held = file.as_deref().and_then(|file| {
                std::fs::read_to_string(format!(
                    "{root}/config/transmission/{}",
                    file.trim_start_matches("/config/")
                ))
                .ok()
            });
            (variable.clone(), file, held)
        })
        .collect();
    read.sort();
    assert_eq!(
        read,
        [
            (
                "FILE__PASS".to_owned(),
                Some("/config/rpc-password".to_owned()),
                Some("{{secret:upstream-password}}".to_owned())
            ),
            (
                "FILE__USER".to_owned(),
                Some("/config/rpc-username".to_owned()),
                Some("{{secret:upstream-username}}".to_owned())
            ),
        ]
    );
}
