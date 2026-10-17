//! #1041: connect an MCP stdio server whose tool names collide with a yoyo
//! builtin (or with a tool an earlier server already registered) under a
//! prefix, instead of skipping the whole server.
//!
//! Before this module, the collision guard in
//! `agent_builder::connect_external_servers` refused such a server outright,
//! so the flagship `@modelcontextprotocol/server-filesystem` (which exposes
//! `read_file` and `write_file`) contributed ZERO tools. Now its tools reach
//! the model as `<prefix>__<tool>` via yoagent's
//! `McpToolAdapter::from_client_with_prefix`, while every call still goes to
//! the server under the server's own tool name.
//!
//! Decisions, stated where the code is:
//! - **Prefix only on collision.** A server whose names collide with nothing
//!   gets [`McpConnectPlan::Plain`] and connects exactly as before
//!   (`with_mcp_server_stdio`); prefixing is not a rename of working servers.
//! - **The whole server is prefixed**, not just the colliding names: one
//!   consistent namespace per server.
//! - **Re-checked after prefixing.** If a prefixed name is still taken (two
//!   servers sharing a prefix), the server is skipped with the existing skip
//!   message: there is no second prefix scheme.
//! - `BUILTIN_TOOL_NAMES` stays the single authority: callers pass it in,
//!   nothing here copies it.

use std::collections::HashMap;
use std::sync::Arc;

use yoagent::agent::Agent;
use yoagent::mcp::{McpClient, McpToolAdapter};
use yoagent::{AgentTool, ToolSource};

use crate::agent_builder::{detect_mcp_collisions, record_failed_server};
use crate::format::{GREEN, RED, RESET, YELLOW};

/// Longest prefix yoyo uses. Providers cap tool names (Anthropic at 64
/// characters), and the prefix is only a disambiguator, so a long
/// `[mcp_servers.<name>]` is cut rather than allowed to push every tool name
/// of that server past the cap.
const MCP_PREFIX_MAX_CHARS: usize = 32;

/// Pure: turn a server name into a prefix the provider will accept in a tool
/// name: every char outside `[A-Za-z0-9_-]` becomes `_`, the result is capped
/// at [`MCP_PREFIX_MAX_CHARS`] chars (char-wise, never a byte index), and an
/// empty result becomes `mcp`.
pub(crate) fn sanitize_mcp_prefix(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(MCP_PREFIX_MAX_CHARS)
        .collect();
    if s.is_empty() {
        "mcp".to_string()
    } else {
        s
    }
}

/// The name the model sees for `tool` under `prefix` — the exact format
/// `McpToolAdapter::with_prefix` produces.
pub(crate) fn prefixed_tool_name(prefix: &str, tool: &str) -> String {
    format!("{prefix}__{tool}")
}

/// What to do with one MCP server, decided from its pre-flight tool listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpConnectPlan {
    /// Nothing collides: connect exactly as before.
    Plain,
    /// Some names collide: connect every tool as `<prefix>__<tool>`.
    /// `colliding` are the server's own names that forced the prefix.
    Prefixed {
        prefix: String,
        colliding: Vec<String>,
    },
    /// Even the prefixed names collide: skip the server. Carries the
    /// prefixed names that are still taken.
    Skip(Vec<String>),
}

/// Pure: decide how to connect a server exposing `tool_names`, given the
/// builtin names and the names earlier servers in this session registered.
pub(crate) fn plan_mcp_connect(
    tool_names: &[String],
    builtins: &[&str],
    already_registered: &[String],
    prefix: &str,
) -> McpConnectPlan {
    // One detector for both checks: `detect_mcp_collisions` over the builtins
    // plus every name an earlier server registered.
    let mut taken: Vec<&str> = builtins.to_vec();
    taken.extend(already_registered.iter().map(String::as_str));
    let colliding = detect_mcp_collisions(tool_names, &taken);
    if colliding.is_empty() {
        return McpConnectPlan::Plain;
    }
    let prefixed: Vec<String> = tool_names
        .iter()
        .map(|n| prefixed_tool_name(prefix, n))
        .collect();
    let still = detect_mcp_collisions(&prefixed, &taken);
    if still.is_empty() {
        McpConnectPlan::Prefixed {
            prefix: prefix.to_string(),
            colliding,
        }
    } else {
        McpConnectPlan::Skip(still)
    }
}

/// Pure: the one stderr line saying a server was connected under a prefix,
/// naming the server, the names that forced it and the prefix. `server_cmd`
/// is repository-authored (a project `.yoyo.toml` or a flag), so it goes
/// through `cli::sanitize_for_display` like `collision_guard_skipped_message`.
/// Glyph-free when `plain` (no em dash).
pub(crate) fn prefix_applied_message(
    server_cmd: &str,
    prefix: &str,
    colliding: &[String],
    plain: bool,
) -> String {
    let shown = crate::cli::sanitize_for_display(server_cmd);
    let names = colliding
        .iter()
        .map(|n| format!("'{}'", crate::cli::sanitize_for_display(n)))
        .collect::<Vec<_>>()
        .join(", ");
    let dash = if plain { "-" } else { "\u{2014}" };
    format!(
        "mcp: '{shown}' exposes {names}, already used by yoyo {dash} connected with every tool of this server renamed to '{prefix}__<tool>'"
    )
}

/// A fixed set of prefixed MCP tools, handed to the agent as a
/// [`ToolSource`] because yoagent's `Agent` has no "append one tool" builder
/// (`with_tools` replaces the list).
struct PrefixedMcpTools(Vec<Arc<dyn AgentTool>>);

#[async_trait::async_trait]
impl ToolSource for PrefixedMcpTools {
    async fn tools(&self) -> Vec<Arc<dyn AgentTool>> {
        self.0.clone()
    }
}

/// One prefixed connection to make.
pub(crate) struct PrefixedConnect<'a> {
    pub command: &'a str,
    pub args: &'a [&'a str],
    pub env: Option<HashMap<String, String>>,
    /// What the user called the server (flag text or config name).
    pub label: &'a str,
    /// The id recorded for the model-facing failure note.
    pub failure_id: &'a str,
    pub prefix: &'a str,
    pub colliding: &'a [String],
}

/// Spawn a fresh client for `spec` and add its tools to `agent` under the
/// prefix. Unlike `with_mcp_server_stdio`, a failure here does NOT consume the
/// agent, so no earlier connection is lost; the failure is printed and
/// recorded for the model via `record_failed_server`, the same path as every
/// other connect failure (the model must not be handed an absence). On
/// success the final names join `registered` and `mcp_count` grows by one.
pub(crate) async fn connect_prefixed(
    agent: Agent,
    spec: PrefixedConnect<'_>,
    registered: &mut Vec<String>,
    mcp_count: &mut u32,
) -> Agent {
    let result = async {
        let client = McpClient::connect_stdio(spec.command, spec.args, spec.env).await?;
        let client = Arc::new(tokio::sync::Mutex::new(client));
        McpToolAdapter::from_client_with_prefix(client, spec.prefix).await
    }
    .await;
    match result {
        Ok(adapters) => {
            let plain = crate::format::is_plain_output();
            eprintln!(
                "{YELLOW}  {}{RESET}",
                prefix_applied_message(spec.label, spec.prefix, spec.colliding, plain)
            );
            let tools: Vec<Arc<dyn AgentTool>> = adapters
                .into_iter()
                .map(|a| Arc::new(a) as Arc<dyn AgentTool>)
                .collect();
            registered.extend(tools.iter().map(|t| t.name().to_string()));
            *mcp_count += 1;
            let shown = crate::cli::sanitize_for_display(spec.label);
            eprintln!(
                "{GREEN}  ✓ mcp: {shown} connected (prefix '{}'){RESET}",
                spec.prefix
            );
            agent.with_tool_source(PrefixedMcpTools(tools))
        }
        Err(e) => {
            let shown = crate::cli::sanitize_for_display(spec.label);
            let e = crate::cli::sanitize_for_display(&e.to_string());
            eprintln!("{RED}  ✗ mcp: failed to connect to '{shown}': {e}{RESET}");
            record_failed_server("mcp", spec.failure_id);
            agent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_builder::BUILTIN_TOOL_NAMES;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// The flagship reference server's tool set (the case #1041 is about).
    fn filesystem_server() -> Vec<String> {
        names(&[
            "read_file",
            "read_multiple_files",
            "write_file",
            "edit_file",
            "create_directory",
            "list_directory",
            "directory_tree",
            "move_file",
            "search_files",
            "get_file_info",
            "list_allowed_directories",
        ])
    }

    #[test]
    fn filesystem_server_is_prefixed_not_skipped() {
        let tools = filesystem_server();
        let plan = plan_mcp_connect(&tools, BUILTIN_TOOL_NAMES, &[], "fs");
        assert_eq!(
            plan,
            McpConnectPlan::Prefixed {
                prefix: "fs".to_string(),
                colliding: names(&["read_file", "write_file", "edit_file"]),
            }
        );
        for t in &tools {
            let final_name = prefixed_tool_name("fs", t);
            assert!(final_name.starts_with("fs__"));
            assert!(
                !BUILTIN_TOOL_NAMES.contains(&final_name.as_str()),
                "{final_name} collides with a builtin"
            );
        }
    }

    #[test]
    fn non_colliding_server_stays_plain() {
        let tools = names(&["get_weather", "get_forecast"]);
        assert_eq!(
            plan_mcp_connect(&tools, BUILTIN_TOOL_NAMES, &[], "weather"),
            McpConnectPlan::Plain
        );
    }

    #[test]
    fn collision_with_an_earlier_server_also_prefixes() {
        let tools = names(&["search_docs"]);
        let earlier = names(&["search_docs"]);
        assert_eq!(
            plan_mcp_connect(&tools, BUILTIN_TOOL_NAMES, &earlier, "docs2"),
            McpConnectPlan::Prefixed {
                prefix: "docs2".to_string(),
                colliding: names(&["search_docs"]),
            }
        );
    }

    #[test]
    fn second_server_with_same_prefix_is_skipped() {
        let tools = filesystem_server();
        let first = plan_mcp_connect(&tools, BUILTIN_TOOL_NAMES, &[], "fs");
        assert!(matches!(first, McpConnectPlan::Prefixed { .. }));
        let registered: Vec<String> = tools.iter().map(|t| prefixed_tool_name("fs", t)).collect();
        let second = plan_mcp_connect(&tools, BUILTIN_TOOL_NAMES, &registered, "fs");
        assert_eq!(second, McpConnectPlan::Skip(registered.clone()));
    }

    #[test]
    fn sanitize_mcp_prefix_cases() {
        assert_eq!(sanitize_mcp_prefix("my server!"), "my_server_");
        assert_eq!(sanitize_mcp_prefix(""), "mcp");
        assert_eq!(sanitize_mcp_prefix("fs"), "fs");
        assert_eq!(sanitize_mcp_prefix("a-b_c9"), "a-b_c9");
        // Multibyte: one `_` per char, no byte slicing, no panic.
        assert_eq!(sanitize_mcp_prefix("✓é"), "__");
        let long = "é".repeat(100);
        assert_eq!(
            sanitize_mcp_prefix(&long).chars().count(),
            MCP_PREFIX_MAX_CHARS
        );
    }

    #[test]
    fn prefix_message_names_server_collisions_and_prefix() {
        let out = prefix_applied_message(
            "npx server-filesystem /tmp",
            "mcp1",
            &names(&["read_file", "write_file"]),
            false,
        );
        assert_eq!(
            out,
            "mcp: 'npx server-filesystem /tmp' exposes 'read_file', 'write_file', already used by yoyo \u{2014} connected with every tool of this server renamed to 'mcp1__<tool>'"
        );
    }

    #[test]
    fn prefix_message_is_glyph_free_when_plain() {
        let out = prefix_applied_message("fs", "fs", &names(&["read_file"]), true);
        assert!(!out.contains('•') && !out.contains('—'), "{out}");
        // Anti-vacuous: the non-plain form really does carry the em dash.
        let loud = prefix_applied_message("fs", "fs", &names(&["read_file"]), false);
        assert!(loud.contains('—'));
    }

    #[test]
    fn prefix_message_escapes_control_bytes_in_server_cmd() {
        let hostile = "evil\x1b[2Jcmd";
        assert!(
            hostile.as_bytes().contains(&0x1b),
            "fixture must be hostile"
        );
        let out = prefix_applied_message(hostile, "fs", &names(&["read_file"]), true);
        assert!(!out.as_bytes().contains(&0x1b), "{out:?}");
    }
}
