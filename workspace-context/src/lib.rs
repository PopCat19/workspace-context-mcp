use serde::Deserialize;
use zed::settings::ContextServerSettings;
use zed_extension_api::{self as zed, Command, ContextServerId, Project, Result, serde_json};

struct WorkspaceContextExtension;

#[derive(Debug, Deserialize)]
struct WorkspaceContextServerSettings {
    server_path: String,
    args: Option<Vec<String>>,
    env: Option<Vec<(String, String)>>,
}

impl zed::Extension for WorkspaceContextExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        let ctx_server_settings = ContextServerSettings::for_project("workspace-context", project)?;
        let Some(settings) = ctx_server_settings.settings else {
            return Err("missing settings for workspace-context".into());
        };
        let settings: WorkspaceContextServerSettings =
            serde_json::from_value(settings).map_err(|e| e.to_string())?;

        if settings.server_path.is_empty() {
            return Err("missing server_path in workspace-context settings".into());
        }

        Ok(Command {
            command: settings.server_path,
            args: settings.args.unwrap_or_default(),
            env: settings.env.unwrap_or_default(),
        })
    }
}

zed::register_extension!(WorkspaceContextExtension);
