use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use uuid::Uuid;

use shared::{BrewResponse, ReadingResponse};

pub struct NotebookRegistry {
    processes: Mutex<HashMap<Uuid, (u16, Child)>>,
}

impl NotebookRegistry {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for NotebookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum NotebookError {
    Io(std::io::Error),
    Lock,
    MarimoNotFound,
}

impl std::fmt::Display for NotebookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotebookError::Io(e) => write!(f, "IO error: {e}"),
            NotebookError::Lock => write!(f, "Lock poisoned"),
            NotebookError::MarimoNotFound => {
                write!(f, "marimo not found — install it with: pip install marimo")
            }
        }
    }
}

impl From<std::io::Error> for NotebookError {
    fn from(e: std::io::Error) -> Self {
        NotebookError::Io(e)
    }
}

/// Ensure a marimo server is running for the given brew and return its port.
/// If a process for this brew already exists and is still alive, reuse it.
/// Otherwise spawn a new one on the next available port starting from `port_base`.
pub fn ensure_notebook_server(
    registry: &NotebookRegistry,
    brew: &BrewResponse,
    readings: &[ReadingResponse],
    notebook_dir: &str,
    port_base: u16,
    marimo_host: &str,
) -> Result<u16, NotebookError> {
    let mut processes = registry.processes.lock().map_err(|_| NotebookError::Lock)?;

    let brew_id = brew.id;

    if let Some((port, child)) = processes.get_mut(&brew_id) {
        match child.try_wait() {
            Ok(None) => {
                return Ok(*port);
            }
            _ => {
                processes.remove(&brew_id);
            }
        }
    }

    let notebook_path = ensure_notebook_file(notebook_dir, brew, readings)?;

    let port = find_free_port(&processes, port_base);

    let (marimo_bin, extra_args) = find_marimo_binary().ok_or(NotebookError::MarimoNotFound)?;

    let port_str = port.to_string();
    let path_str = notebook_path.to_str().unwrap_or("").to_string();

    let mut cmd = Command::new(&marimo_bin);
    cmd.args(&extra_args);
    cmd.args([
        "edit",
        "--no-token",
        "--headless",
        "--host",
        marimo_host,
        "--port",
        &port_str,
        &path_str,
    ]);
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    let child = cmd.spawn()?;

    tracing::info!(
        brew_id = %brew_id,
        brew_name = %brew.name,
        port = port,
        path = %notebook_path.display(),
        "Started marimo notebook server"
    );

    processes.insert(brew_id, (port, child));
    Ok(port)
}

fn find_free_port(processes: &HashMap<Uuid, (u16, Child)>, base: u16) -> u16 {
    let used: std::collections::HashSet<u16> = processes.values().map(|(p, _)| *p).collect();
    let mut port = base;
    while used.contains(&port) {
        port += 1;
    }
    port
}

/// Returns `(binary, leading_args)` so that `Command::new(binary).args(leading_args).args(["edit", ...])` works.
fn find_marimo_binary() -> Option<(String, Vec<String>)> {
    if which_exists("marimo") {
        return Some(("marimo".to_string(), vec![]));
    }
    for python in &["python3", "python"] {
        if which_exists(python) {
            let ok = Command::new(python)
                .args(["-c", "import marimo"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Some((
                    python.to_string(),
                    vec!["-m".to_string(), "marimo".to_string()],
                ));
            }
        }
    }
    None
}

fn which_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Write (or refresh) the marimo notebook `.py` file for a brew.
fn ensure_notebook_file(
    notebook_dir: &str,
    brew: &BrewResponse,
    readings: &[ReadingResponse],
) -> Result<PathBuf, NotebookError> {
    let dir = Path::new(notebook_dir);
    std::fs::create_dir_all(dir)?;

    let filename = format!("{}.py", brew.id);
    let path = dir.join(&filename);

    let content = generate_notebook(brew, readings);
    std::fs::write(&path, content)?;

    Ok(path)
}

fn escape_py_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn py_optional_float(v: Option<f64>) -> String {
    match v {
        Some(f) => format!("{f}"),
        None => "None".to_string(),
    }
}

fn py_optional_str(v: Option<&str>) -> String {
    match v {
        Some(s) => format!("\"{}\"", escape_py_str(s)),
        None => "None".to_string(),
    }
}

fn generate_notebook(brew: &BrewResponse, readings: &[ReadingResponse]) -> String {
    static TEMPLATE: &str = include_str!("notebook_template.py");

    let brew_name = escape_py_str(&brew.name);
    let brew_id = brew.id.to_string();
    let brew_status = format!("{:?}", brew.status);
    let style = match &brew.style {
        Some(s) => escape_py_str(s),
        None => "—".to_string(),
    };
    let og = py_optional_float(brew.og);
    let fg = py_optional_float(brew.fg);
    let target_fg = py_optional_float(brew.target_fg);
    let live_abv = py_optional_float(brew.live_abv);
    let final_abv = py_optional_float(brew.final_abv);
    let start_date = py_optional_str(brew.start_date.as_ref().map(|d| d.to_rfc3339()).as_deref());
    let end_date = py_optional_str(brew.end_date.as_ref().map(|d| d.to_rfc3339()).as_deref());
    let notes = py_optional_str(brew.notes.as_deref());

    let readings_data: String = readings
        .iter()
        .map(|r| {
            format!(
                "        {{\"recorded_at\": \"{}\", \"temperature_f\": {:.4}, \"gravity\": {:.3}, \"rssi\": {}}},\n",
                r.recorded_at,
                r.temperature_f,
                r.gravity,
                r.rssi.map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
            )
        })
        .collect();

    TEMPLATE
        .replace("__BREW_NAME__", &brew_name)
        .replace("__BREW_ID__", &brew_id)
        .replace("__BREW_STATUS__", &brew_status)
        .replace("__BREW_STYLE__", &style)
        .replace("__BREW_OG__", &og)
        .replace("__BREW_FG__", &fg)
        .replace("__BREW_TARGET_FG__", &target_fg)
        .replace("__BREW_LIVE_ABV__", &live_abv)
        .replace("__BREW_FINAL_ABV__", &final_abv)
        .replace("__BREW_START_DATE__", &start_date)
        .replace("__BREW_END_DATE__", &end_date)
        .replace("__BREW_NOTES__", &notes)
        .replace("__READINGS_DATA__", &readings_data)
}
