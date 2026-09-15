use std::fs;
use zed_extension_api::{
    self as zed, settings::LspSettings, Architecture, DownloadedFileType, LanguageServerId, Os,
    Result,
};

struct PascalExtension;

impl zed::Extension for PascalExtension {
    fn new() -> Self {
        PascalExtension
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let (os, arch) = zed::current_platform();

        let binary_name = if os == Os::Windows {
            "pasls-release/pasls-win64-x86_64.exe"
        } else {
            "pasls-release/pasls-linux-x86_64"
        };

        let mut command = binary_name.to_string();
        let mut args = vec![];
        let mut env = vec![
            ("PP".to_string(), "/usr/bin/fpc".to_string()),
            ("FPCDIR".to_string(), "/usr/share/fpcsrc".to_string()),
            (
                "LAZARUSDIR".to_string(),
                "$HOME/.lazarus/Lazarus".to_string(),
            ),
            ("FPCTARGET".to_string(), "linux".to_string()),
            ("FPCTARGETCPU".to_string(), "x86_64".to_string()),
        ];

        let mut user_provided_binary = false;

        if let Ok(lsp_settings) = LspSettings::for_worktree(language_server_id.as_ref(), worktree) {
            if let Some(binary) = lsp_settings.binary {
                if let Some(path) = binary.path {
                    command = path;
                    user_provided_binary = true;
                }
                if let Some(binary_args) = binary.arguments {
                    args = binary_args;
                }

                if let Some(binary_env) = binary.env {
                    for (key, value) in binary_env {
                        if let Some(existing) = env.iter_mut().find(|item| item.0 == key) {
                            existing.1 = value;
                        } else {
                            env.push((key, value));
                        }
                    }
                }
            }
        }

        if !user_provided_binary {
            if !fs::metadata(&command).map_or(false, |stat| stat.is_file()) {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Downloading,
                );

                let release = zed::latest_github_release(
                    "oisanjaya/pascal-language-server",
                    zed::GithubReleaseOptions {
                        require_assets: true,
                        pre_release: false,
                    },
                )
                .map_err(|e| format!("Failed to fetch GitHub release info: {}", e))?;

				let asset_name_suffix = match (os, arch) {
                    (Os::Linux, Architecture::X8664) => "linux-x86_64.tar.gz",
                    (Os::Windows, Architecture::X8664) => "win64-x86_64.zip",
                    _ => return Err(format!("Unsupported platform: {:?} {:?}", os, arch)),
                };
                
                let asset = release
					.assets
					.iter()
					.find(|asset| asset.name.contains(asset_name_suffix))
					.ok_or_else(|| format!("No asset found matching {}", asset_name_suffix))?;

                zed::download_file(
                    &asset.download_url,
                    "pasls-release",
                    if os == Os::Windows {
                        DownloadedFileType::Zip
                    } else {
                        DownloadedFileType::GzipTar
                    },
                )
                .map_err(|e| format!("Failed to download pasls: {}", e))?;

                if os != Os::Windows {
                    zed::make_file_executable(&command)
                        .map_err(|e| format!("Failed to set permissions: {}", e))?;
                }

                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
            }
        }

        Ok(zed::Command { command, args, env })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        // Fetch the settings for `pasls` in the current workspace
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.initialization_options.clone());

        Ok(settings)
    }
}

zed::register_extension!(PascalExtension);
