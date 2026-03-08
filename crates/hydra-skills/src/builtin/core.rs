//! Built-in skills — file, http, shell operations.

use crate::definition::*;

/// Load all built-in skills
pub fn builtin_skills() -> Vec<SkillDefinition> {
    vec![
        file_read_skill(),
        file_write_skill(),
        http_get_skill(),
        shell_exec_skill(),
    ]
}

fn file_read_skill() -> SkillDefinition {
    SkillDefinition {
        id: "builtin-file-read".into(),
        name: "file_read".into(),
        version: "1.0.0".into(),
        description: "Read contents of a file".into(),
        triggers: vec![
            SkillTrigger::Pattern("read * file".into()),
            SkillTrigger::Intent("file_read".into()),
            SkillTrigger::Tool("builtin.file_read".into()),
        ],
        parameters: vec![SkillParam {
            name: "path".into(),
            param_type: ParamType::Path,
            required: true,
            description: "File path to read".into(),
            default: None,
            constraints: vec![],
        }],
        outputs: vec![SkillOutput {
            name: "content".into(),
            output_type: ParamType::String,
            description: "File contents".into(),
        }],
        requirements: vec![Requirement::FileSystem],
        source: SkillSource::Builtin,
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::Low,
        metadata: SkillMetadata {
            author: "hydra".into(),
            idempotent: true,
            ..Default::default()
        },
    }
}

fn file_write_skill() -> SkillDefinition {
    SkillDefinition {
        id: "builtin-file-write".into(),
        name: "file_write".into(),
        version: "1.0.0".into(),
        description: "Write contents to a file".into(),
        triggers: vec![
            SkillTrigger::Pattern("write * to *".into()),
            SkillTrigger::Intent("file_write".into()),
            SkillTrigger::Tool("builtin.file_write".into()),
        ],
        parameters: vec![
            SkillParam {
                name: "path".into(),
                param_type: ParamType::Path,
                required: true,
                description: "File path to write".into(),
                default: None,
                constraints: vec![],
            },
            SkillParam {
                name: "content".into(),
                param_type: ParamType::String,
                required: true,
                description: "Content to write".into(),
                default: None,
                constraints: vec![],
            },
        ],
        outputs: vec![SkillOutput {
            name: "bytes_written".into(),
            output_type: ParamType::Number,
            description: "Number of bytes written".into(),
        }],
        requirements: vec![Requirement::FileSystem],
        source: SkillSource::Builtin,
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::Medium,
        metadata: SkillMetadata {
            author: "hydra".into(),
            reversible: true,
            ..Default::default()
        },
    }
}

fn http_get_skill() -> SkillDefinition {
    SkillDefinition {
        id: "builtin-http-get".into(),
        name: "http_get".into(),
        version: "1.0.0".into(),
        description: "Make an HTTP GET request".into(),
        triggers: vec![
            SkillTrigger::Pattern("fetch *".into()),
            SkillTrigger::Intent("http_get".into()),
            SkillTrigger::Tool("builtin.http_get".into()),
        ],
        parameters: vec![SkillParam {
            name: "url".into(),
            param_type: ParamType::String,
            required: true,
            description: "URL to fetch".into(),
            default: None,
            constraints: vec![],
        }],
        outputs: vec![
            SkillOutput {
                name: "status".into(),
                output_type: ParamType::Number,
                description: "HTTP status code".into(),
            },
            SkillOutput {
                name: "body".into(),
                output_type: ParamType::String,
                description: "Response body".into(),
            },
        ],
        requirements: vec![Requirement::Network],
        source: SkillSource::Builtin,
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::Low,
        metadata: SkillMetadata {
            author: "hydra".into(),
            idempotent: true,
            cacheable: true,
            ..Default::default()
        },
    }
}

fn shell_exec_skill() -> SkillDefinition {
    SkillDefinition {
        id: "builtin-shell-exec".into(),
        name: "shell_exec".into(),
        version: "1.0.0".into(),
        description: "Execute a shell command".into(),
        triggers: vec![
            SkillTrigger::Pattern("run *".into()),
            SkillTrigger::Intent("shell_exec".into()),
            SkillTrigger::Tool("builtin.shell_exec".into()),
        ],
        parameters: vec![SkillParam {
            name: "command".into(),
            param_type: ParamType::String,
            required: true,
            description: "Command to execute".into(),
            default: None,
            constraints: vec![],
        }],
        outputs: vec![
            SkillOutput {
                name: "stdout".into(),
                output_type: ParamType::String,
                description: "Standard output".into(),
            },
            SkillOutput {
                name: "exit_code".into(),
                output_type: ParamType::Number,
                description: "Exit code".into(),
            },
        ],
        requirements: vec![Requirement::Permission("shell".into())],
        source: SkillSource::Builtin,
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::High,
        metadata: SkillMetadata {
            author: "hydra".into(),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_skills_load() {
        let skills = builtin_skills();
        assert_eq!(skills.len(), 4);
        assert!(skills.iter().all(|s| s.source == SkillSource::Builtin));
        assert!(skills.iter().all(|s| !s.triggers.is_empty()));

        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
        assert!(names.contains(&"http_get"));
        assert!(names.contains(&"shell_exec"));
    }

    #[test]
    fn test_builtin_risk_levels() {
        let skills = builtin_skills();
        let shell = skills.iter().find(|s| s.name == "shell_exec").unwrap();
        assert_eq!(shell.risk_level, RiskLevel::High);

        let read = skills.iter().find(|s| s.name == "file_read").unwrap();
        assert_eq!(read.risk_level, RiskLevel::Low);
    }
}
