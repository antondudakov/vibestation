//! What the developer types, and the name they get for it.

use vibestation::naming;

#[test]
fn a_line_of_prompt_input_becomes_a_session_name() {
    let table = [
        (
            "VBSN-1 initialize the project",
            "ada/VBSN-1-initialize-the-project",
        ),
        ("VBSN-1", "ada/VBSN-1"),
        ("just a description", "ada/just-a-description"),
        ("fix VBSN-1 later", "ada/fix-vbsn-1-later"),
        (
            "vbsn-1 lowercase is not a ticket",
            "ada/vbsn-1-lowercase-is-not-a-ticket",
        ),
        ("VBSN-1   Spaced   Out  ", "ada/VBSN-1-spaced-out"),
        (
            "VBSN-1 punctuation: it's fine!",
            "ada/VBSN-1-punctuation-it-s-fine",
        ),
        ("  ", "ada"),
        (
            "VBSN-42 a description far longer than fifty characters will ever need to be",
            "ada/VBSN-42-a-description-far-longer-than-fifty-characters",
        ),
    ];

    for (said, expected) in table {
        assert_eq!(naming::suggest("ada", said), expected, "input {said:?}");
    }
}

#[test]
fn work_that_already_has_a_branch_is_named_after_it() {
    assert_eq!(
        naming::from_branch("ada", "ada/VBSN-1-init"),
        "ada/VBSN-1-init",
        "a branch this tool cut already carries the prefix"
    );
    assert_eq!(naming::from_branch("ada", "hotfix"), "ada/hotfix");
    assert_eq!(naming::from_branch("", "hotfix"), "hotfix");
}

#[test]
fn characters_tmux_forbids_are_replaced_and_slashes_survive() {
    assert_eq!(
        naming::from_branch("ada", "release/v1.2:rc"),
        "ada/release/v1-2-rc",
        "dots and colons become hyphens; slashes carry the convention"
    );
    assert_eq!(naming::sanitize("a.b:c/d"), "a-b-c/d");
}
