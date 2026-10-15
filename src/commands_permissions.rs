//! The `/permissions` report (#869 slice, Day 228): a pure renderer over the
//! startup policy AND the deny entries `/cd` appended since launch.
//!
//! Before this, `/permissions` read only the launch-time `PermissionConfig` and
//! `DirectoryRestrictions`, so after `/cd` into a project whose `.yoyo.toml`
//! carries `deny` / `[directories] deny`, the tools refused those commands and
//! directories while the one surface whose job is "what protects me?" printed
//! `Directory restrictions: none (full filesystem access)`. The "none" lines may
//! now print only when the startup list AND the `/cd` list are both empty.
//!
//! Volume is decided by severity, not by adjacency: a `/cd` entry is live policy,
//! so it prints in the same red `✗` style as a startup entry, never dimmed. Each
//! `/cd` entry carries its source label so startup policy and `/cd` policy can be
//! told apart. `/cd` entries come from a repository-authored `.yoyo.toml`, so they
//! are sanitized for display; startup entries are rendered exactly as before.

use crate::format::{DIM, GREEN, RED, RESET, YELLOW};

/// Label on a `/cd`-appended bash deny pattern, the command-side twin of
/// [`crate::commands_file_fence::CD_DIR_SOURCE_NOTE`].
pub(crate) const CD_BASH_DENY_SOURCE_NOTE: &str = "(a deny entry applied by /cd)";

/// Label on a `/cd`-applied `[directories] allow` entry (Day 228). It says the
/// two things a user cannot infer from a green tick under "Allowed": the entry
/// is a SECOND fence (a path must pass the launch list too), and it lasts only
/// until the next `/cd`, which replaces it.
pub(crate) const CD_DIR_ALLOW_SOURCE_NOTE: &str =
    "(an allow entry applied by /cd; also required, replaced by the next /cd)";

/// Heading over launch-project allow entries `/cd` suspended (Day 229).
pub(crate) const CD_SUSPENDED_ALLOW_HEADING: &str =
    "Suspended allow patterns (launch project's .yoyo.toml; not applied outside its tree):";

/// The full `/permissions` text. `cd_bash_deny` / `cd_dir_deny` are the lists
/// `/cd` has appended (`cd_deny::cd_added_deny_handle`,
/// `cd_config_note::cd_denied_dirs_handle`), and `cd_dir_allow` is the allow
/// list of the latest `/cd` (`cd_config_note::cd_allowed_dirs_handle`, Day
/// 228); empty slices render the pre-#869 text byte-identically.
pub(crate) fn permissions_report(
    auto_approve: bool,
    permissions: &crate::cli::PermissionConfig,
    dir_restrictions: &crate::cli::DirectoryRestrictions,
    cd_bash_deny: &[String],
    cd_dir_deny: &[String],
    cd_dir_allow: &[String],
    cd_suspended_allow: &[String],
) -> String {
    let mut out = String::new();
    let mut line = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    line(format!("{DIM}  Security Configuration:\n"));

    // Auto-approve status
    if auto_approve {
        line(format!(
            "    {YELLOW}⚠ Auto-approve: ON{RESET}{DIM} (--yes flag active)"
        ));
        line(format!(
            "      All tool operations run without confirmation{RESET}"
        ));
    } else {
        line(format!("    {GREEN}✓ Confirmation: required{RESET}"));
        line(format!(
            "    {DIM}  Tools will prompt before write/edit/bash operations{RESET}"
        ));
    }
    line(String::new());

    // Bash command permissions
    if permissions.is_empty() && cd_bash_deny.is_empty() {
        line("    Command patterns: none configured".to_string());
    } else {
        // Day 229 (#869): an entry `/cd` suspended is not live policy, so it is
        // not listed under the ✓ heading — it gets its own block saying why.
        let (live, suspended): (Vec<&String>, Vec<&String>) = permissions
            .allow
            .iter()
            .partition(|p| !cd_suspended_allow.contains(p));
        if !live.is_empty() {
            line(format!("    {GREEN}Allow patterns:{RESET}"));
            for pat in live {
                line(format!("      {GREEN}✓{RESET} {pat}"));
            }
        }
        if !suspended.is_empty() {
            line(format!("    {DIM}{CD_SUSPENDED_ALLOW_HEADING}{RESET}"));
            for pat in suspended {
                line(format!("      - {pat}"));
            }
        }
        if !permissions.deny.is_empty() || !cd_bash_deny.is_empty() {
            line(format!("    {RED}Deny patterns:{RESET}"));
            for pat in &permissions.deny {
                line(format!("      {RED}✗{RESET} {pat}"));
            }
            for pat in cd_bash_deny {
                let pat = crate::cli::sanitize_for_display(pat);
                line(format!(
                    "      {RED}✗{RESET} {pat} {CD_BASH_DENY_SOURCE_NOTE}"
                ));
            }
        }
    }
    line(String::new());

    // Directory restrictions
    if dir_restrictions.is_empty() && cd_dir_deny.is_empty() && cd_dir_allow.is_empty() {
        line("    Directory restrictions: none (full filesystem access)".to_string());
    } else {
        if !dir_restrictions.allow.is_empty() || !cd_dir_allow.is_empty() {
            line(format!("    {GREEN}Allowed directories:{RESET}"));
            for dir in &dir_restrictions.allow {
                line(format!("      {GREEN}✓{RESET} {dir}"));
            }
            for dir in cd_dir_allow {
                let dir = crate::cli::sanitize_for_display(dir);
                line(format!(
                    "      {GREEN}✓{RESET} {dir} {CD_DIR_ALLOW_SOURCE_NOTE}"
                ));
            }
        }
        if !dir_restrictions.deny.is_empty() || !cd_dir_deny.is_empty() {
            line(format!("    {RED}Denied directories:{RESET}"));
            for dir in &dir_restrictions.deny {
                line(format!("      {RED}✗{RESET} {dir}"));
            }
            let note = crate::commands_file_fence::CD_DIR_SOURCE_NOTE;
            for dir in cd_dir_deny {
                let dir = crate::cli::sanitize_for_display(dir);
                line(format!("      {RED}✗{RESET} {dir} {note}"));
            }
        }
    }
    line(String::new());

    // Quick reference
    line(format!(
        "    {DIM}Configure with: --allow <pat>, --deny <pat>, --allow-dir <d>, --deny-dir <d>"
    ));
    line(format!(
        "    Or in .yoyo.toml: allow = [...], deny = [...]{RESET}\n"
    ));
    out
}

/// A snapshot of a `/cd` deny handle.
pub(crate) fn snapshot(list: &crate::cd_deny::DenyAdditions) -> Vec<String> {
    crate::sync_util::lock_or_recover(list).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{DirectoryRestrictions, PermissionConfig};

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    /// HEAD's text (pre-Day-228 `handle_permissions`), transcribed line for line
    /// from its `println!`s, for the regression cases where `/cd` added nothing.
    fn head_text(
        auto_approve: bool,
        permissions: &PermissionConfig,
        dirs: &DirectoryRestrictions,
    ) -> String {
        let mut o = String::new();
        o += &format!("{DIM}  Security Configuration:\n\n");
        if auto_approve {
            o += &format!("    {YELLOW}⚠ Auto-approve: ON{RESET}{DIM} (--yes flag active)\n");
            o += &format!("      All tool operations run without confirmation{RESET}\n");
        } else {
            o += &format!("    {GREEN}✓ Confirmation: required{RESET}\n");
            o +=
                &format!("    {DIM}  Tools will prompt before write/edit/bash operations{RESET}\n");
        }
        o += "\n";
        if permissions.is_empty() {
            o += "    Command patterns: none configured\n";
        } else {
            if !permissions.allow.is_empty() {
                o += &format!("    {GREEN}Allow patterns:{RESET}\n");
                for p in &permissions.allow {
                    o += &format!("      {GREEN}✓{RESET} {p}\n");
                }
            }
            if !permissions.deny.is_empty() {
                o += &format!("    {RED}Deny patterns:{RESET}\n");
                for p in &permissions.deny {
                    o += &format!("      {RED}✗{RESET} {p}\n");
                }
            }
        }
        o += "\n";
        if dirs.is_empty() {
            o += "    Directory restrictions: none (full filesystem access)\n";
        } else {
            if !dirs.allow.is_empty() {
                o += &format!("    {GREEN}Allowed directories:{RESET}\n");
                for d in &dirs.allow {
                    o += &format!("      {GREEN}✓{RESET} {d}\n");
                }
            }
            if !dirs.deny.is_empty() {
                o += &format!("    {RED}Denied directories:{RESET}\n");
                for d in &dirs.deny {
                    o += &format!("      {RED}✗{RESET} {d}\n");
                }
            }
        }
        o += "\n";
        o += &format!(
            "    {DIM}Configure with: --allow <pat>, --deny <pat>, --allow-dir <d>, --deny-dir <d>\n"
        );
        o += &format!("    Or in .yoyo.toml: allow = [...], deny = [...]{RESET}\n\n");
        o
    }

    fn deny_line(entry: &str, note: &str) -> String {
        format!("      {RED}✗{RESET} {entry} {note}\n")
    }

    #[test]
    fn cd_dir_deny_is_shown_and_full_access_is_not_claimed() {
        let perms = PermissionConfig::default();
        let dirs = DirectoryRestrictions::default();
        let out = permissions_report(false, &perms, &dirs, &[], &s(&["/proj/secret"]), &[], &[]);
        assert!(
            !out.contains("full filesystem access"),
            "a live /cd fence must not be reported as full access:\n{out}"
        );
        let expected_block = format!(
            "    {RED}Denied directories:{RESET}\n{}\n",
            deny_line(
                "/proj/secret",
                crate::commands_file_fence::CD_DIR_SOURCE_NOTE
            )
        );
        assert!(out.contains(&expected_block), "got:\n{out}");
        // Everything else is HEAD's text: only the directory block changed.
        let head = head_text(false, &perms, &dirs);
        assert_eq!(
            out,
            head.replace(
                "    Directory restrictions: none (full filesystem access)\n",
                &expected_block[..expected_block.len() - 1]
            )
        );
    }

    #[test]
    fn cd_bash_deny_is_shown_and_none_configured_is_not_claimed() {
        let perms = PermissionConfig::default();
        let dirs = DirectoryRestrictions::default();
        let out = permissions_report(false, &perms, &dirs, &s(&["curl *"]), &[], &[], &[]);
        assert!(!out.contains("none configured"), "got:\n{out}");
        let head = head_text(false, &perms, &dirs);
        let block = format!(
            "    {RED}Deny patterns:{RESET}\n{}",
            deny_line("curl *", CD_BASH_DENY_SOURCE_NOTE)
        );
        assert_eq!(
            out,
            head.replace("    Command patterns: none configured\n", &block)
        );
    }

    #[test]
    fn both_lists_empty_is_byte_identical_to_head() {
        let perms = PermissionConfig::default();
        let dirs = DirectoryRestrictions::default();
        for auto in [false, true] {
            let out = permissions_report(auto, &perms, &dirs, &[], &[], &[], &[]);
            assert_eq!(out, head_text(auto, &perms, &dirs));
            // Anti-vacuous: HEAD's text really carries the claim this slice guards.
            assert!(out.contains("Directory restrictions: none (full filesystem access)"));
            assert!(out.contains("Command patterns: none configured"));
        }
    }

    #[test]
    fn startup_policy_only_is_byte_identical_to_head() {
        let perms = PermissionConfig {
            allow: s(&["cargo *"]),
            deny: s(&["rm -rf *"]),
        };
        let dirs = DirectoryRestrictions {
            allow: s(&["/project"]),
            deny: s(&["/etc", "/usr"]),
        };
        for auto in [false, true] {
            let out = permissions_report(auto, &perms, &dirs, &[], &[], &[], &[]);
            assert_eq!(out, head_text(auto, &perms, &dirs));
        }
    }

    #[test]
    fn both_present_each_shown_with_its_own_label() {
        let perms = PermissionConfig {
            allow: vec![],
            deny: s(&["rm *"]),
        };
        let dirs = DirectoryRestrictions {
            allow: vec![],
            deny: s(&["/etc"]),
        };
        let out = permissions_report(
            false,
            &perms,
            &dirs,
            &s(&["curl *"]),
            &s(&["/p/secret"]),
            &[],
            &[],
        );
        let note = crate::commands_file_fence::CD_DIR_SOURCE_NOTE;
        let dir_block = format!(
            "    {RED}Denied directories:{RESET}\n      {RED}✗{RESET} /etc\n{}",
            deny_line("/p/secret", note)
        );
        let cmd_block = format!(
            "    {RED}Deny patterns:{RESET}\n      {RED}✗{RESET} rm *\n{}",
            deny_line("curl *", CD_BASH_DENY_SOURCE_NOTE)
        );
        assert!(out.contains(&dir_block), "got:\n{out}");
        assert!(out.contains(&cmd_block), "got:\n{out}");
        // The startup entries carry no /cd label.
        assert!(!out.contains(&format!("/etc {note}")));
        assert!(!out.contains(&format!("rm * {CD_BASH_DENY_SOURCE_NOTE}")));
        assert!(!out.contains("full filesystem access"));
    }

    #[test]
    fn cd_entries_are_sanitized_for_display() {
        let hostile = "/p/se\x1b[2Jcret".to_string();
        assert!(hostile.as_bytes().contains(&0x1b), "fixture must carry ESC");
        let out = permissions_report(
            false,
            &PermissionConfig::default(),
            &DirectoryRestrictions::default(),
            std::slice::from_ref(&hostile),
            std::slice::from_ref(&hostile),
            std::slice::from_ref(&hostile),
            &[],
        );
        assert!(
            !out.contains(&hostile),
            "raw control bytes reached the report"
        );
    }

    #[test]
    fn cd_dir_allow_is_listed_with_its_label_and_full_access_is_not_claimed() {
        let perms = PermissionConfig::default();
        let dirs = DirectoryRestrictions::default();
        let out = permissions_report(false, &perms, &dirs, &[], &[], &s(&["/proj/src"]), &[]);
        // The exact line a user reads.
        let line =
            format!("      {GREEN}✓{RESET} /proj/src (an allow entry applied by /cd; also required, replaced by the next /cd)\n");
        assert!(
            out.contains(&format!("    {GREEN}Allowed directories:{RESET}\n{line}")),
            "got:\n{out}"
        );
        assert!(!out.contains("full filesystem access"), "got:\n{out}");
        // Only the directory block differs from HEAD's text.
        let head = head_text(false, &perms, &dirs);
        assert_eq!(
            out,
            head.replace(
                "    Directory restrictions: none (full filesystem access)\n",
                &format!("    {GREEN}Allowed directories:{RESET}\n{line}")
            )
        );
        // With a launch allow list too, the launch entry stays unlabelled and first.
        let launch = DirectoryRestrictions {
            allow: s(&["/proj"]),
            deny: vec![],
        };
        let out = permissions_report(false, &perms, &launch, &[], &[], &s(&["/proj/src"]), &[]);
        assert!(
            out.contains(&format!(
                "    {GREEN}Allowed directories:{RESET}\n      {GREEN}✓{RESET} /proj\n{line}"
            )),
            "got:\n{out}"
        );
    }

    /// Day 229 (#869): a /cd-suspended allow entry leaves the ✓ list and is
    /// shown as suspended; an unsuspended neighbour stays exactly where it was.
    #[test]
    fn suspended_allow_entry_is_not_listed_as_live() {
        let perms = PermissionConfig {
            allow: s(&["cargo *", "ls *"]),
            deny: vec![],
        };
        let dirs = DirectoryRestrictions::default();
        let out = permissions_report(false, &perms, &dirs, &[], &[], &[], &s(&["cargo *"]));
        let head = head_text(false, &perms, &dirs);
        assert_eq!(
            out,
            head.replace(
                &format!("      {GREEN}✓{RESET} cargo *\n      {GREEN}✓{RESET} ls *\n"),
                &format!(
                    "      {GREEN}✓{RESET} ls *\n    {DIM}{CD_SUSPENDED_ALLOW_HEADING}{RESET}\n      - cargo *\n"
                )
            )
        );
        assert!(!out.contains(&format!("✓{RESET} cargo *")), "got:\n{out}");
        // Every entry suspended: no empty ✓ heading.
        let out = permissions_report(
            false,
            &perms,
            &dirs,
            &[],
            &[],
            &[],
            &s(&["cargo *", "ls *"]),
        );
        assert!(!out.contains("Allow patterns:"), "got:\n{out}");
        assert!(out.contains("      - ls *\n"), "got:\n{out}");
    }

    #[test]
    fn snapshot_reads_a_local_handle() {
        let h: crate::cd_deny::DenyAdditions = Default::default();
        assert!(snapshot(&h).is_empty());
        crate::cd_deny::append_deny_with(&h, &s(&["/x"]));
        assert_eq!(snapshot(&h), s(&["/x"]));
    }
}
