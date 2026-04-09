use super::*;

#[test]
fn get_existing_topic() {
    assert!(get("zig").is_some());
    assert!(get("run").is_some());
    assert!(get("workflow").is_some());
    assert!(get("describe").is_some());
    assert!(get("validate").is_some());
    assert!(get("zug").is_some());
    assert!(get("patterns").is_some());
    assert!(get("variables").is_some());
    assert!(get("conditions").is_some());
}

#[test]
fn get_unknown_topic_returns_none() {
    assert!(get("nonexistent").is_none());
    assert!(get("").is_none());
}

#[test]
fn all_pages_are_nonempty() {
    for (topic, _) in TOPICS {
        let content = get(topic).unwrap_or_else(|| panic!("missing manpage for '{topic}'"));
        assert!(!content.is_empty(), "manpage for '{topic}' is empty");
    }
}

#[test]
fn all_pages_start_with_heading() {
    for (topic, _) in TOPICS {
        let content = get(topic).unwrap();
        assert!(
            content.starts_with('#'),
            "manpage for '{topic}' should start with a markdown heading"
        );
    }
}

#[test]
fn list_topics_contains_all_entries() {
    let listing = list_topics();
    for (topic, description) in TOPICS {
        assert!(
            listing.contains(topic),
            "listing should contain topic '{topic}'"
        );
        assert!(
            listing.contains(description),
            "listing should contain description '{description}'"
        );
    }
}

#[test]
fn list_topics_shows_usage() {
    let listing = list_topics();
    assert!(listing.contains("zig man <topic>"));
}

#[test]
fn topics_list_matches_get() {
    for (topic, _) in TOPICS {
        assert!(
            get(topic).is_some(),
            "TOPICS lists '{topic}' but get() returns None"
        );
    }
}
