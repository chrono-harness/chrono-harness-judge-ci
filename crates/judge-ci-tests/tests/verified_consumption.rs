//! Boundary proofs exercise the same executable preparation predicate.
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verified_consumption.rs"]
mod policy;
use vstd::prelude::*;

// Installer policy rendered for this host from Copier v0.1.42.
#[cfg(any(test, verus_keep_ghost))]
mod installed_selection {
    use vstd::prelude::*;

    verus! {
        /// A mise setup installs tools first and refreshes the rules only when selected.
    pub fn setup_sequence(commands: &[String], instructions: bool) -> (accepted: bool)
        ensures accepted == (commands.len() == (if instructions { 2usize } else { 1usize })
            && commands@[0]@ == "sh .chrono-harness/install-tools.sh"@
            && (!instructions || commands@[1]@ == "mise exec -- chrono-instructions sync-rules --host-root ."@)),
    {
        if commands.len() != (if instructions { 2usize } else { 1usize }) { return false; }
        commands[0] == "sh .chrono-harness/install-tools.sh".to_string() && (!instructions
            || commands[1] == "mise exec -- chrono-instructions sync-rules --host-root .".to_string())
    }

    pub fn mise_manifest() -> (s: &'static str) {
        include_str!("../../../.config/mise/conf.d/chrono-harness.toml")
    }

    pub fn tool_installer() -> (s: &'static str) { include_str!("../../../.chrono-harness/install-tools.sh") }

    /// The template renders these same rows into both mise platform tables.
        pub fn installed_asset(linux: bool) -> (fields: (&'static str, usize))
            ensures linux ==> fields.0@ == "linux-x86_64"@ && fields.1 == 1,
                !linux ==> fields.0@ == "macos-aarch64"@ && fields.1 == 2,
        {
            if linux {
                ("linux-x86_64", 1)
            } else {
                ("macos-aarch64", 2)
            }
        }

        /// mise reports file/key presence and applies the selected Rust-regex expression.
        #[derive(Clone, Copy)]
        pub enum InstalledValue { Pin, Use, Reject }

        pub fn installed_value(present: bool, matches: bool) -> (action: InstalledValue)
            ensures !present ==> action == InstalledValue::Pin,
                present && matches ==> action == InstalledValue::Use,
                present && !matches ==> action == InstalledValue::Reject,
        {
            if !present { InstalledValue::Pin } else if matches { InstalledValue::Use }
            else { InstalledValue::Reject }
        }

        pub fn installed_pattern(is_version: bool) -> (pattern: &'static str)
            ensures is_version ==> pattern@ == "^[0-9]+[.][0-9]+[.][0-9]+(-[A-Za-z0-9-]+([.][A-Za-z0-9-]+)*)?$"@,
                !is_version ==> pattern@ == "^sha256:[a-f0-9]{64}$"@,
        {
            if is_version { "^[0-9]+[.][0-9]+[.][0-9]+(-[A-Za-z0-9-]+([.][A-Za-z0-9-]+)*)?$" } else { "^sha256:[a-f0-9]{64}$" }
        }

        pub fn installed_config() -> (s: &'static str) {
            include_str!("../../../.config/mise/conf.d/selected-chrono-harness.toml")
        }
            }

    #[cfg(all(test, not(verus_keep_ghost)))]
    mod native {
        use super::*;

        fn setup_matches(source: &str, instructions: bool) -> bool {
            let Ok(doc) = source.parse::<toml::Table>() else {
                return false;
            };
            let instruction_tool = doc
                .get("tools")
                .and_then(toml::Value::as_table)
                .is_some_and(|tools| tools.contains_key("http:chrono-instructions"));
            let Some(setup) = doc
                .get("tasks")
                .and_then(|tasks| tasks.get("harness-setup"))
                .and_then(toml::Value::as_table)
            else {
                return false;
            };
            if instruction_tool != instructions
                || setup.keys().any(|key| key != "run" && key != "description")
            {
                return false;
            }
            let commands: Vec<String> = match setup.get("run") {
                Some(toml::Value::String(one)) => vec![one.clone()],
                Some(toml::Value::Array(steps)) => {
                    let Some(all) = steps
                        .iter()
                        .map(|value| value.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()
                    else {
                        return false;
                    };
                    all
                }
                _ => return false,
            };
            setup_sequence(&commands, instructions)
        }

        #[test]
        fn every_installed_organization_tool_can_follow_the_selected_channel() {
            use std::collections::BTreeSet;

            let manifest = mise_manifest().parse::<toml::Table>().unwrap();
            let expected: BTreeSet<_> = manifest["tools"]
                .as_table()
                .unwrap()
                .keys()
                .filter_map(|name| name.strip_prefix("http:chrono-"))
                .collect();
            let (_, rows) = tool_installer().split_once("done <<'TOOLS'\n").unwrap();
            let (rows, _) = rows.split_once("\nTOOLS\n").unwrap();
            let mut selected = BTreeSet::new();
            for row in rows.lines() {
                let (command, _) = row.split_once(' ').unwrap();
                assert!(
                    selected.insert(command.strip_prefix("chrono-").unwrap()),
                    "{command}"
                );
            }
            assert_eq!(selected, expected);

            let projection = installed_config().parse::<toml::Table>().unwrap();
            let projected: BTreeSet<_> = projection["tools"]
                .as_table()
                .unwrap()
                .iter()
                .map(|(key, tool)| {
                    let command = tool["bin"].as_str().unwrap();
                    assert_eq!(key.strip_prefix("http:"), Some(command));
                    command.strip_prefix("chrono-").unwrap()
                })
                .collect();
            assert_eq!(projected, expected);
        }

        #[test]
        fn mise_installs_the_selected_tools_before_refreshing_instructions() {
            let original = mise_manifest();
            let selected = true;
            assert!(setup_matches(original, selected));
            assert!(!setup_matches(
                &original.replace(
                    "sh .chrono-harness/install-tools.sh",
                    "sh .chrono-harness/install-tools.sh --dry-run"
                ),
                selected
            ));
            assert!(!setup_matches(
                &original.replace(
                    "[tasks.harness-setup]",
                    "[tasks.harness-setup]\ndepends = ['other']"
                ),
                selected
            ));
            let install_only =
                "[tools]\n[tasks.harness-setup]\nrun = 'sh .chrono-harness/install-tools.sh'";
            assert!(setup_matches(install_only, false));
            assert!(!setup_matches(install_only, true));
            let reverse = "[tools.'http:chrono-instructions']\n[tasks.harness-setup]\nrun=['mise exec -- chrono-instructions sync-rules --host-root .','sh .chrono-harness/install-tools.sh']";
            assert!(!setup_matches(reverse, true));
            let extra = "[tools]\n[tasks.harness-setup]\nrun = ['sh .chrono-harness/install-tools.sh', 'another-command']";
            assert!(!setup_matches(extra, false));
        }

        fn when_selected(condition: &str, installed: &str, pinned: &str) -> String {
            format!("{{% if {condition} %}}{installed}{{% else %}}{pinned}{{% endif %}}")
        }

        // Reconstruct the actions of the host's actual mise expressions for every
        // outcome of its built-in file, TOML lookup and regex operations.
        fn installed_field_expression(is_version: bool, command: &str, index: usize) -> String {
            let (value, lookup, field) = if is_version {
                ("version", format!(r#"data["{command}"][0]"#), "version")
            } else {
                (
                    "hash",
                    format!(r#"read_toml(path=vars.chrono_selection)["{command}"][{index}]"#),
                    "checksum",
                )
            };
            let action = |present, matches| match installed_value(present, matches) {
                InstalledValue::Pin if is_version => String::new(),
                InstalledValue::Pin => r#"{{ PLATFORM["checksum"] }}"#.to_owned(),
                InstalledValue::Use => format!("{{{{ {value} }}}}"),
                InstalledValue::Reject => {
                    format!(r#"{{{{ throw(message="Invalid installed release {field}") }}}}"#)
                }
            };
            let assign = format!("{{% set {value} = {lookup} %}}");
            let pattern = installed_pattern(is_version);
            let check = format!(r#"{{% if {value} is matching(pat="{pattern}") %}}"#);
            let yes = action(true, true);
            let no = action(true, false);
            let fallback = action(false, false);
            let end = "{% endif %}";
            let otherwise = "{% else %}";
            let inner = format!("{assign}{check}{yes}{otherwise}{no}{end}");
            if is_version {
                let file = "{% if vars.chrono_selection is exists %}{% set data = read_toml(path=vars.chrono_selection) %}{% endif %}";
                let has = format!(r#"{{% if data is defined and "{command}" in data %}}"#);
                format!("{file}{has}{inner}{end}{fallback}")
            } else {
                when_selected("SELECTED", &inner, &fallback)
            }
        }

        #[test]
        fn installed_assets_use_the_verified_platforms() {
            let document: toml::Value = toml::from_str(installed_config()).unwrap();
            assert_eq!(
                document["vars"]["chrono_selection"].as_str(),
                Some("{{ config_root }}/.chrono-harness/state/tool-selection/versions.toml")
            );
            assert_eq!(
                document["vars"]["chrono_pins"].as_str(),
                Some("{{ config_root }}/.config/mise/conf.d/chrono-harness.toml")
            );
            let entries = document["tools"].as_table().unwrap();
            assert_eq!(entries.len(), 21);
            for (key, tool) in entries {
                let command = key.strip_prefix("http:").unwrap();
                let repository = match command {
                    "chrono-cache" => "chrono-harness/chrono-harness-cache",
                    "chrono-ci" => "chrono-harness/chrono-harness-ci",
                    "chrono-diagnostics" => "chrono-harness/chrono-harness-diagnostics",
                    "chrono-distribution" => "chrono-harness/chrono-harness-distribution",
                    "chrono-harness" => "chrono-harness/chrono-harness-runner",
                    "chrono-host-bootstrap" => "chrono-harness/chrono-harness-host-bootstrap",
                    "chrono-host-release" => "chrono-harness/chrono-harness-host-release",
                    "chrono-inputs" => "chrono-harness/chrono-harness-inputs",
                    "chrono-instructions" => "chrono-harness/chrono-harness-instructions",
                    "chrono-judge-ci" => "chrono-harness/chrono-harness-judge-ci",
                    "chrono-judge-cost" => "chrono-harness/chrono-harness-judge-cost",
                    "chrono-judge-filemap" => "chrono-harness/chrono-harness-judge-filemap",
                    "chrono-judge-mixed" => "chrono-harness/chrono-harness-judge-mixed",
                    "chrono-judge-projects" => "chrono-harness/chrono-harness-judge-projects",
                    "chrono-judge-registration" => {
                        "chrono-harness/chrono-harness-judge-registration"
                    }
                    "chrono-judge-routes" => "chrono-harness/chrono-harness-judge-routes",
                    "chrono-judge-workflow" => "chrono-harness/chrono-harness-judge-workflow",
                    "chrono-pr" => "chrono-harness/chrono-harness-pr",
                    "chrono-test-git" => "chrono-harness/chrono-harness-test-git",
                    "chrono-test-git-profile" => "chrono-harness/chrono-harness-test-git",
                    "chrono-worktree" => "chrono-harness/chrono-harness-worktree",
                    _ => panic!("unexpected selected tool {command}"),
                };
                let variable = format!("selected_{}", command.replace('-', "_"));
                assert_eq!(
                    document["vars"][&variable].as_str(),
                    Some(installed_field_expression(true, command, 0).as_str())
                );
                let selected = format!("vars.{variable}");
                let value = format!("{{{{ {selected} }}}}");
                let pin_tool =
                    format!(r#"read_toml(path=vars.chrono_pins)["tools"]["http:{command}"]"#);
                let pinned_version = format!(r#"{{{{ {pin_tool}["version"] }}}}"#);
                let version = when_selected(&selected, &value, &pinned_version);
                assert_eq!(tool["version"].as_str(), Some(version.as_str()));
                let platforms = tool["platforms"].as_table().unwrap();
                assert_eq!(platforms.len(), 2);
                for (name, linux) in [("linux-x64", true), ("macos-arm64", false)] {
                    let (suffix, index) = installed_asset(linux);
                    let pin = format!(r#"{pin_tool}["platforms"]["{name}"]"#);
                    let asset = format!(
                        "https://github.com/{repository}/releases/download/v{value}/{command}-{suffix}"
                    );
                    let pinned_url = format!(r#"{{{{ {pin}["url"] }}}}"#);
                    let url = when_selected(&selected, &asset, &pinned_url);
                    let checksum = installed_field_expression(false, command, index)
                        .replace("SELECTED", &selected)
                        .replace("PLATFORM", &pin);
                    assert_eq!(platforms[name]["url"].as_str(), Some(url.as_str()));
                    assert_eq!(
                        platforms[name]["checksum"].as_str(),
                        Some(checksum.as_str())
                    );
                }
            }
        }
    }
}

// Cargo builds this same release obligation as an executable for the publishing workflow.
mod release_observation {
    #[cfg(not(verus_keep_ghost))]
    use vstd::prelude::verus;
    #[cfg(verus_keep_ghost)]
    use vstd::prelude::*;

    verus! {
    pub fn query_diagnostic() -> (value: &'static str)
        ensures value@ == "E_REQUIREMENTS: invalid relative path"@,
    { "E_REQUIREMENTS: invalid relative path" }

    pub fn judge_diagnostic() -> (value: &'static str)
        ensures value@ == "E_REQUEST:"@,
    { "E_REQUEST:" }

    pub fn accepts(query_exit: i32, query_match: bool, judge_exit: i32, judge_match: bool) -> (accepted: bool)
        ensures accepted == (query_exit == 2 && query_match && judge_exit == 2 && judge_match),
    {
        query_exit == 2 && query_match && judge_exit == 2 && judge_match
    }
    }

    #[cfg(not(verus_keep_ghost))]
    fn grep(mode: &str, pattern: &str, path: &str) -> Result<bool, String> {
        // grep interprets its documented matching mode and reports the original matching lines.
        std::process::Command::new("grep")
            .args([mode, pattern, path])
            .status()
            .map(|result| result.success())
            .map_err(|error| error.to_string())
    }

    #[cfg(not(verus_keep_ghost))]
    pub(super) fn evaluate_observations() -> Result<(), String> {
        let arguments: Vec<_> = std::env::args().skip(1).collect();
        let [query_exit, query_error, judge_exit, judge_error] = arguments.as_slice() else {
            return Err(
                "usage: release_observation QUERY_EXIT QUERY_ERROR JUDGE_EXIT JUDGE_ERROR".into(),
            );
        };
        let query_exit = query_exit.parse::<i32>().map_err(|e| e.to_string())?;
        let judge_exit = judge_exit.parse::<i32>().map_err(|e| e.to_string())?;
        let query_match = grep("-F", query_diagnostic(), query_error)?;
        let judge_match = grep("-F", judge_diagnostic(), judge_error)?;
        let admitted = accepts(query_exit, query_match, judge_exit, judge_match);
        if admitted {
            Ok(())
        } else {
            Err(format!("unexpected release observation for {arguments:?}"))
        }
    }
}

#[cfg(all(not(verus_keep_ghost), not(test)))]
fn main() -> Result<(), String> {
    release_observation::evaluate_observations()
}

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_accepts_the_expected_observations_only() {
    let accepted = release_observation::accepts(2, true, 2, true);
    assert(accepted);
    let refused_1 = release_observation::accepts(0, true, 2, true);
    assert(!refused_1);
    let refused_2 = release_observation::accepts(2, false, 2, true);
    assert(!refused_2);
    let refused_3 = release_observation::accepts(2, true, 0, true);
    assert(!refused_3);
    let refused_4 = release_observation::accepts(2, true, 2, false);
    assert(!refused_4);
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn preparation_requires_exact_command_and_prerequisite() {
    proof {
        reveal_strlit("python"); reveal_strlit("bootstrap"); reveal_strlit(".");
        reveal_strlit("profile"); reveal_strlit("projects"); reveal_strlit("verify");
        reveal_strlit("chrono-bootstrap/v1"); reveal_strlit("other"); reveal_strlit("extra");
        assert("verify"@.len() != "other"@.len());
        assert("profile"@.len() != "other"@.len());
        assert("chrono-bootstrap/v1"@.len() != "other"@.len());
        assert("."@.len() != "other"@.len());
        assert("python"@.len() != "other"@.len());
        assert("projects"@.len() != "other"@.len());
    }
    let command = vec!["python", "bootstrap", ".", "profile"];
    let prerequisites = vec!["verify"];
    let accepted = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(accepted);
    let missing: Vec<&str> = Vec::new();
    let absent = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &missing, "verify", "python", "python", "projects", "projects");
    assert(!absent);
    let duplicate = vec!["verify", "verify"];
    let duplicated = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &duplicate, "verify", "python", "python", "projects", "projects");
    assert(!duplicated);
    let wrong = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "other", "python", "python", "projects", "projects");
    assert(!wrong);
    let changed = policy::matches_preparation(&command, "python", "bootstrap", "other",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!changed);
    let schema = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "other", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!schema);
    let root = vec!["python", "bootstrap", "other", "profile"];
    let wrong_root = policy::matches_preparation(&root, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!wrong_root);
    let extra = vec!["python", "bootstrap", ".", "profile", "extra"];
    let extra_arg = policy::matches_preparation(&extra, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "projects", "projects");
    assert(!extra_arg);
    let executor = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "other", "python", "projects", "projects");
    assert(!executor);
    let registry = policy::matches_preparation(&command, "python", "bootstrap", "profile",
        "chrono-bootstrap/v1", &prerequisites, "verify", "python", "python", "other", "projects");
    assert(!registry);
}

}
