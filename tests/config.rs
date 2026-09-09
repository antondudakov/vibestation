//! First run, loading, and the ways a hand-edited config can be wrong.

use std::path::PathBuf;
use vibestation::config::{self, Config};
use vibestation::fake::{Answer, FakeHost};

const CONFIG: &str = "/home/dev/.vibestation/config.toml";

/// A machine with no config, whose git knows the developer's name.
fn fresh() -> FakeHost {
    FakeHost::new().succeeds("git config user.name", "Ada Lovelace\n")
}

#[test]
fn first_run_asks_one_question_and_writes_a_config_that_loads_back() {
    let host = fresh().answer(Answer::text("/home/dev/work"));

    let config = config::load_or_init(&host).unwrap();

    assert_eq!(
        host.prompts(),
        ["Where do your projects live? [/home/dev/code]"],
        "exactly one question, per constitution §5"
    );
    assert_eq!(
        config,
        Config {
            projects_dirs: vec![PathBuf::from("/home/dev/work")],
            username: "ada-lovelace".to_string(),
            ..Config::default()
        }
    );
    assert_eq!((config.scan_depth, config.fetch_before_branch), (10, true));

    let writes = host.writes();
    let (path, written) = &writes[0];
    assert_eq!(writes.len(), 1);
    assert_eq!(path, &PathBuf::from(CONFIG));
    for field in [
        "projects_dirs",
        "extra_projects",
        "username",
        "default_branch",
        "scan_depth",
        "fetch_before_branch",
    ] {
        assert!(written.contains(field), "{field} is missing from {written}");
    }
    assert!(
        written.lines().filter(|l| l.starts_with('#')).count() >= 7,
        "every field is introduced by a comment: {written}"
    );

    let reopened = FakeHost::new().file(CONFIG, written);
    assert_eq!(config::load_or_init(&reopened).unwrap(), config);
    assert!(reopened.prompts().is_empty(), "loading is silent");
    assert!(reopened.writes().is_empty(), "loading rewrites nothing");
}

#[test]
fn the_offered_directory_is_one_that_exists_and_a_tilde_is_expanded() {
    let host = fresh()
        .file("/home/dev/projects/thing/.git/HEAD", "")
        .answer(Answer::text("~/elsewhere  "));

    let config = config::load_or_init(&host).unwrap();

    assert_eq!(
        host.prompts(),
        ["Where do your projects live? [/home/dev/projects]"]
    );
    assert_eq!(config.projects_dirs, [PathBuf::from("/home/dev/elsewhere")]);
}

#[test]
fn the_username_falls_back_to_the_home_directory_name() {
    let host = FakeHost::new()
        .home("/home/ada")
        .fails("git config user.name", 1, "")
        .answer(Answer::text("/home/ada/code"));

    assert_eq!(config::load_or_init(&host).unwrap().username, "ada");
}

#[test]
fn omitted_fields_fall_back_to_defaults() {
    let host = FakeHost::new().file(CONFIG, "projects_dirs = [\"/src\"]\n");

    assert_eq!(
        config::load_or_init(&host).unwrap(),
        Config {
            projects_dirs: vec![PathBuf::from("/src")],
            ..Config::default()
        }
    );
}

#[test]
fn a_hand_written_config_is_taken_as_written() {
    let host = FakeHost::new().file(
        CONFIG,
        r#"
            projects_dirs = ["/src", "/work"]
            extra_projects = ["/opt/vendored"]
            username = "ada"
            default_branch = "develop"
            scan_depth = 3
            fetch_before_branch = false
        "#,
    );

    assert_eq!(
        config::load_or_init(&host).unwrap(),
        Config {
            projects_dirs: vec![PathBuf::from("/src"), PathBuf::from("/work")],
            extra_projects: vec![PathBuf::from("/opt/vendored")],
            username: "ada".to_string(),
            default_branch: Some("develop".to_string()),
            scan_depth: 3,
            fetch_before_branch: false,
        }
    );
}

#[test]
fn a_malformed_config_names_the_file() {
    let host = FakeHost::new().file(CONFIG, "projects_dirs = [\nscan_depth = nope\n");

    let error = config::load_or_init(&host).unwrap_err().to_string();

    assert!(error.starts_with(CONFIG), "{error}");
    assert_eq!(error.lines().count(), 1, "a message, not a parse trace");
}
