use serde::Deserialize;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct LspConfig {
    #[serde(default = "default_true")]
    pub markdown: bool,
    #[serde(default = "default_true")]
    pub document_color: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            markdown: true,
            document_color: true,
        }
    }
}

impl LspConfig {
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut config = LspConfig::default();

        if let Some(obj) = value.as_object() {
            if let Some(settings) = obj.get("settings")
                && let Some(settings_obj) = settings.as_object()
            {
                if let Some(v) = settings_obj.get("markdown").and_then(|v| v.as_bool()) {
                    config.markdown = v;
                }
                if let Some(v) = settings_obj.get("documentColor").and_then(|v| v.as_bool()) {
                    config.document_color = v;
                }
                return config;
            }

            if let Some(v) = obj.get("markdown").and_then(|v| v.as_bool()) {
                config.markdown = v;
            }
            if let Some(v) = obj.get("documentColor").and_then(|v| v.as_bool()) {
                config.document_color = v;
            }
        }

        config
    }
}
