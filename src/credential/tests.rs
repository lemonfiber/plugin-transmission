use std::path::PathBuf;

use lemonfiber_contract::adapter::UPSTREAM_FILE;

use super::{Credential, Holding, Upstream};

fn directory(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "transmission-adapter-credential-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn a_password() -> String {
    ('d'..='q').collect()
}

#[test]
fn the_file_lemonfiber_wrote_is_read_as_the_credential() {
    let dir = directory("read");
    let password = a_password();
    let _ = std::fs::write(
        dir.join(UPSTREAM_FILE),
        format!(r#"{{"username":"admin","password":"{password}"}}"#),
    );
    assert_eq!(
        Upstream::in_directory(&dir).credential(),
        Ok(Credential {
            username: "admin".to_owned(),
            password,
        })
    );
}

#[test]
fn a_missing_file_is_refused_by_name() {
    let dir = directory("missing");
    let refused = Upstream::in_directory(&dir).credential();
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.starts_with("upstream.json could not be read")),
        "{refused:?}"
    );
}

#[test]
fn a_file_that_is_not_an_object_of_strings_is_refused_without_quoting_it() {
    let dir = directory("unreadable");
    let password = a_password();
    for written in [
        format!("password={password}"),
        format!(r#"{{"username":"admin","password":["{password}"]}}"#),
    ] {
        let _ = std::fs::write(dir.join(UPSTREAM_FILE), written);
        let refused = Upstream::in_directory(&dir).credential();
        assert_eq!(
            refused,
            Err("upstream.json is not an object of named strings".to_owned())
        );
    }
}

#[test]
fn an_empty_or_missing_part_is_no_credential_and_is_named() {
    let dir = directory("empty");
    for (written, missing) in [
        (r#"{"username":"admin","password":""}"#, "password"),
        (r#"{"password":"x"}"#, "username"),
    ] {
        let _ = std::fs::write(dir.join(UPSTREAM_FILE), written);
        assert_eq!(
            Upstream::in_directory(&dir).credential(),
            Err(format!("upstream.json names no {missing}"))
        );
    }
}

#[test]
fn a_credential_never_prints_its_password() {
    let password = a_password();
    let held = Credential {
        username: "admin".to_owned(),
        password: password.clone(),
    };
    assert!(!format!("{held:?}").contains(&password));
}

#[test]
fn a_file_that_cannot_be_read_is_refused_by_name() {
    let dir = directory("directory");
    let _ = std::fs::create_dir_all(dir.join(UPSTREAM_FILE));
    let refused = Upstream::in_directory(&dir).credential();
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.starts_with("upstream.json could not be")),
        "{refused:?}"
    );
}
