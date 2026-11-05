use std::borrow::Cow;

use rmcp::schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Deserialize;

/// The eight memory commands, serialized as Anthropic's string enum.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    View,
    Create,
    StrReplace,
    Insert,
    Delete,
    Rename,
    History,
    Restore,
}

impl Command {
    pub const ALL: [Command; 8] = [
        Command::View,
        Command::Create,
        Command::StrReplace,
        Command::Insert,
        Command::Delete,
        Command::Rename,
        Command::History,
        Command::Restore,
    ];

    /// `"str_replace"` style name, as used in the tool schema and error messages.
    pub fn as_str(&self) -> &'static str {
        match self {
            Command::View => "view",
            Command::Create => "create",
            Command::StrReplace => "str_replace",
            Command::Insert => "insert",
            Command::Delete => "delete",
            Command::Rename => "rename",
            Command::History => "history",
            Command::Restore => "restore",
        }
    }
}

/// The `command` field as supplied by the model: one of the eight supported
/// commands, or an unrecognized string.
///
/// The JSON schema advertises only the eight-value enum (it delegates to
/// [`Command`]); unrecognized values are surfaced to the dispatch layer as
/// [`CommandArg::Unknown`] so they can be reported as tool-level error
/// results rather than protocol errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandArg {
    Known(Command),
    Unknown(String),
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum KnownCommand {
    View,
    Create,
    StrReplace,
    Insert,
    Delete,
    Rename,
    History,
    Restore,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CommandInput {
    Known(KnownCommand),
    Unknown(String),
}

impl<'de> Deserialize<'de> for CommandArg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match CommandInput::deserialize(deserializer)? {
            CommandInput::Known(KnownCommand::View) => CommandArg::Known(Command::View),
            CommandInput::Known(KnownCommand::Create) => CommandArg::Known(Command::Create),
            CommandInput::Known(KnownCommand::StrReplace) => CommandArg::Known(Command::StrReplace),
            CommandInput::Known(KnownCommand::Insert) => CommandArg::Known(Command::Insert),
            CommandInput::Known(KnownCommand::Delete) => CommandArg::Known(Command::Delete),
            CommandInput::Known(KnownCommand::Rename) => CommandArg::Known(Command::Rename),
            CommandInput::Known(KnownCommand::History) => CommandArg::Known(Command::History),
            CommandInput::Known(KnownCommand::Restore) => CommandArg::Known(Command::Restore),
            CommandInput::Unknown(value) => CommandArg::Unknown(value),
        })
    }
}

impl JsonSchema for CommandArg {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "CommandArg".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "memorija::CommandArg".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Command::json_schema(generator)
    }
}

/// The unified `memory` tool input: the flat Anthropic interface with exactly
/// two required fields (`command`, `path`) and eight optional per-command
/// fields.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct MemoryArgs {
    pub command: CommandArg,
    pub path: String,
    pub file_text: Option<String>,
    pub old_str: Option<String>,
    pub new_str: Option<String>,
    pub insert_line: Option<i64>,
    pub insert_text: Option<String>,
    pub new_path: Option<String>,
    pub view_range: Option<[i64; 2]>,
    /// A past version number, honored by `view` and required by `restore`.
    pub version: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::schemars::schema_for;
    use serde_json::json;

    /// Structurally, Anthropic's reference schema for the `memory` tool
    /// (memory_20250818, as implemented in the Anthropic SDK helpers),
    /// extended additively with the versioning commands: an object with
    /// exactly these ten properties, `command` restricted to the eight-value
    /// string enum, `path` a string, the eight per-command fields optional,
    /// and `required` exactly ["command", "path"].
    fn reference() -> serde_json::Value {
        json!({
        "type": "object",
        "properties": {
            "command": { "type": "string", "enum": [
                "view", "create", "str_replace", "insert", "delete", "rename",
                "history", "restore"
            ]},
            "path": { "type": "string" },
            "file_text": { "type": "string" },
            "old_str": { "type": "string" },
            "new_str": { "type": "string" },
            "insert_line": { "type": "integer" },
            "insert_text": { "type": "string" },
            "new_path": { "type": "string" },
            "view_range": { "type": "array", "items": { "type": "integer" } },
            "version": { "type": "integer" }
        },
        "required": ["command", "path"]
        })
    }

    fn generated() -> serde_json::Value {
        serde_json::to_value(schema_for!(MemoryArgs)).unwrap()
    }

    /// The command enum lists all eight commands, in tool-schema order.
    #[test]
    fn command_enum_lists_all_eight_commands() {
        let names: Vec<&str> = Command::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(
            names,
            [
                "view",
                "create",
                "str_replace",
                "insert",
                "delete",
                "rename",
                "history",
                "restore"
            ]
        );
    }

    /// The schema diff recorded during implementation (design D2): the
    /// generated schema matches the reference on every checked aspect. The
    /// only known divergences are schemars metadata additions that carry no
    /// semantic weight ($schema/$ref-less inline object, `type` placement)
    /// and per-field `description`s, which this project writes itself.
    #[test]
    fn schema_matches_anthropic_reference() {
        let schema = generated();

        let required = schema["required"].as_array().unwrap();
        let required: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(required, ["command", "path"], "required must be exactly [command, path]");

        let command = &schema["properties"]["command"];
        let enum_values: Vec<&str> = command["enum"]
            .as_array()
            .expect("command must be a string enum")
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            enum_values,
            [
                "view", "create", "str_replace", "insert", "delete", "rename", "history",
                "restore"
            ]
        );

        let properties = schema["properties"].as_object().unwrap();
        let mut names: Vec<&str> = properties.keys().map(String::as_str).collect();
        names.sort_unstable();
        let reference = reference();
        let mut reference_names: Vec<&str> = reference["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        reference_names.sort_unstable();
        assert_eq!(names, reference_names, "property set must match the reference");

        for optional in [
            "file_text",
            "old_str",
            "new_str",
            "insert_line",
            "insert_text",
            "new_path",
            "view_range",
            "version",
        ] {
            assert!(
                !required.contains(&optional),
                "{optional} must be optional"
            );
        }
    }
}
