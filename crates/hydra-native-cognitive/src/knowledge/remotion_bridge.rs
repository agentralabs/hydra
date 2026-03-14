//! Remotion Bridge — generates Remotion video projects from scene specs.
//! Each scene = React component (from Forge sister).
//! Shell: npx remotion render → MP4.

/// A video specification.
#[derive(Debug, Clone)]
pub struct VideoSpec {
    pub title: String,
    pub scenes: Vec<SceneSpec>,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub format: VideoFormat,
}

/// A single scene in the video.
#[derive(Debug, Clone)]
pub struct SceneSpec {
    pub name: String,
    pub duration_frames: u32,
    pub content: SceneContent,
    pub transition: Option<String>,
}

/// What the scene shows.
#[derive(Debug, Clone)]
pub enum SceneContent {
    Title { text: String, subtitle: Option<String> },
    Code { language: String, code: String, highlight_lines: Vec<u32> },
    Bullets { heading: String, items: Vec<String> },
    Image { path: String, caption: Option<String> },
    Custom { component_code: String },
}

/// Output format.
#[derive(Debug, Clone)]
pub enum VideoFormat {
    Landscape,  // 1920x1080
    Portrait,   // 1080x1920
    Square,     // 1080x1080
}

impl VideoSpec {
    pub fn new(title: &str, format: VideoFormat) -> Self {
        let (w, h) = match format {
            VideoFormat::Landscape => (1920, 1080),
            VideoFormat::Portrait => (1080, 1920),
            VideoFormat::Square => (1080, 1080),
        };
        Self { title: title.into(), scenes: Vec::new(), fps: 30, width: w, height: h, format }
    }

    pub fn add_scene(&mut self, scene: SceneSpec) {
        self.scenes.push(scene);
    }

    pub fn total_frames(&self) -> u32 {
        self.scenes.iter().map(|s| s.duration_frames).sum()
    }

    pub fn duration_secs(&self) -> f64 {
        self.total_frames() as f64 / self.fps as f64
    }
}

/// Generate the Remotion project index file content.
pub fn generate_composition(spec: &VideoSpec) -> String {
    let mut code = String::from("import { Composition } from 'remotion';\n");
    for (i, scene) in spec.scenes.iter().enumerate() {
        code.push_str(&format!("import {{ Scene{} }} from './Scene{}';\n", i, i));
    }
    code.push_str("\nexport const RemotionVideo = () => {\n  return (\n    <>\n");
    code.push_str(&format!(
        "      <Composition id=\"{}\" component={{MainVideo}} \
         durationInFrames={{{}}} fps={{{}}} width={{{}}} height={{{}}}/>\n",
        spec.title.replace(' ', "-"), spec.total_frames(), spec.fps, spec.width, spec.height,
    ));
    code.push_str("    </>\n  );\n};\n");
    code
}

/// Generate a React component for a single scene.
pub fn generate_scene_component(index: usize, scene: &SceneSpec) -> String {
    let body = match &scene.content {
        SceneContent::Title { text, subtitle } => {
            let sub = subtitle.as_deref().unwrap_or("");
            format!(
                "  return (\n    <div style={{{{display:'flex',flexDirection:'column',\
                 alignItems:'center',justifyContent:'center',height:'100%',\
                 background:'#1a1a2e',color:'white'}}}}>\n\
                 <h1 style={{{{fontSize:64}}}}>{}</h1>\n\
                 <p style={{{{fontSize:32,opacity:0.7}}}}>{}</p>\n    </div>\n  );\n",
                text, sub
            )
        }
        SceneContent::Code { language, code, .. } => {
            format!(
                "  return (\n    <div style={{{{padding:40,background:'#0d1117',\
                 color:'#c9d1d9',fontFamily:'monospace',fontSize:20,height:'100%'}}}}>\n\
                 <pre><code>{}</code></pre>\n    </div>\n  );\n",
                code.replace('<', "&lt;").replace('>', "&gt;")
            )
        }
        SceneContent::Bullets { heading, items } => {
            let bullets: String = items.iter()
                .map(|i| format!("<li style={{{{marginBottom:12}}}}>{}</li>", i))
                .collect();
            format!(
                "  return (\n    <div style={{{{padding:60,background:'#1a1a2e',color:'white'}}}}>\n\
                 <h2 style={{{{fontSize:48}}}}>{}</h2>\n<ul style={{{{fontSize:28}}}}>{}</ul>\n\
                 </div>\n  );\n",
                heading, bullets
            )
        }
        _ => "  return <div style={{background:'#1a1a2e',height:'100%'}}/>;\n".into(),
    };

    format!(
        "import {{ useCurrentFrame }} from 'remotion';\n\n\
         export const Scene{} = () => {{\n  const frame = useCurrentFrame();\n{}}}\n",
        index, body
    )
}

/// Build the shell command to render the video.
pub fn render_command(project_dir: &str, output_path: &str) -> String {
    format!(
        "cd {} && npx remotion render src/index.tsx main-video {} --codec h264",
        project_dir, output_path,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_spec() {
        let mut spec = VideoSpec::new("Demo", VideoFormat::Landscape);
        spec.add_scene(SceneSpec {
            name: "intro".into(),
            duration_frames: 90,
            content: SceneContent::Title { text: "Hello".into(), subtitle: Some("World".into()) },
            transition: None,
        });
        assert_eq!(spec.total_frames(), 90);
        assert!((spec.duration_secs() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_generate_scene() {
        let scene = SceneSpec {
            name: "code".into(),
            duration_frames: 150,
            content: SceneContent::Code {
                language: "rust".into(),
                code: "fn main() {}".into(),
                highlight_lines: vec![1],
            },
            transition: None,
        };
        let component = generate_scene_component(0, &scene);
        assert!(component.contains("Scene0"));
        assert!(component.contains("fn main()"));
    }

    #[test]
    fn test_render_command() {
        let cmd = render_command("/tmp/project", "/tmp/output.mp4");
        assert!(cmd.contains("npx remotion render"));
    }
}
