//! Reading a task aloud: its description and notes, rewritten by Claude for
//! listening and spoken by a local Kokoro server.
//!
//! The defaults are herdr-speak's (`~/Projects/herdr-speak`), copied in as
//! constants rather than read from its config or called: this is a fixed
//! feature, not a second front end to that plugin.
//!
//! `niritasks task speak <uuid>` only decides, and returns at once: a speech
//! already playing is stopped, otherwise `task speak <uuid> --here` is started
//! detached in a process group of its own and does the slow part. The group is
//! what makes a stop whole — `claude`, `curl` and `ffplay` are all in it.

use crate::{notify, project, task};
use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

/// The rewrite's system prompt. It lives here rather than with herdr-speak's
/// plan prompt because a task is short and has labelled notes, which a plan
/// does not.
const PROMPT: &str = include_str!("speak_prompt.md");

/// The model the rewrite runs on: fast, and plenty for rephrasing a few lines.
const REWRITE_MODEL: &str = "haiku";
/// How long the rewrite gets before it is killed.
const REWRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// Kokoro's health check and speech endpoint, local only.
const HEALTH_URL: &str = "http://127.0.0.1:8880/health";
const SPEECH_URL: &str = "http://127.0.0.1:8880/v1/audio/speech";
/// The container herdr-speak's README runs, so the two share one server.
const CONTAINER: &str = "kokoro-tts";
/// Kokoro's GPU image; [`kokoro_image`] picks the tag.
const IMAGE: &str = "ghcr.io/remsky/kokoro-fastapi-gpu";
/// How long a just-started container gets to report healthy.
const START_TIMEOUT: Duration = Duration::from_secs(120);
const START_POLL: Duration = Duration::from_secs(1);

const MODEL: &str = "kokoro";
const VOICE: &str = "af_bella";
const SPEED: f64 = 1.0;

/// herdr-speak's default player: raw 24 kHz mono 16-bit PCM on stdin, no
/// window, gone when the audio ends.
const PLAYER: [&str; 13] = [
    "ffplay", "-nodisp", "-autoexit", "-loglevel", "quiet", "-f", "s16le", "-ar", "24000", "-ch_layout", "mono",
    "-i", "-",
];

/// The rewrite's input: the description, then one line per note — the shape
/// the prompt tells Claude to expect. A note's own line breaks are folded, or
/// it would read as several notes.
pub fn rewrite_input(description: &str, notes: &[&str]) -> String {
    let mut input = format!("{description}\n");
    for note in notes {
        input.push_str(&note.split_whitespace().collect::<Vec<_>>().join(" "));
        input.push('\n');
    }
    input
}

/// The rewrite's argv, program first: a bare Haiku with no tools, no saved
/// session, and none of the user's settings, MCP servers or slash commands,
/// so all it can do is rephrase the text on stdin.
fn rewrite_command(prompt: &str) -> Vec<String> {
    [
        "claude",
        "-p",
        "--model",
        REWRITE_MODEL,
        "--no-session-persistence",
        "--tools",
        "",
        "--disable-slash-commands",
        "--strict-mcp-config",
        "--setting-sources",
        "",
        "--system-prompt",
        prompt,
    ]
    .map(String::from)
    .to_vec()
}

/// The environment variables the rewrite drops, out of `keys`: every
/// `HERDR_*`, which a press from inside a herdr pane would hand down and
/// which would point the rewrite's Claude at that pane.
fn dropped_vars(keys: impl IntoIterator<Item = String>) -> Vec<String> {
    keys.into_iter().filter(|k| k.starts_with("HERDR_")).collect()
}

/// Whether a `/health` reply says Kokoro is ready: `{"status": "healthy"}`.
/// Anything else, including no JSON at all from a proxy or a half-started
/// server, is not ready.
fn healthy(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("status")?.as_str().map(|s| s == "healthy"))
        .unwrap_or(false)
}

/// The image to run, from `nvidia-smi --query-gpu=compute_cap` output, one
/// capability per GPU. Blackwell cards (10 and up) need the CUDA 12.8 build,
/// older ones the default; the newest card decides.
fn kokoro_image(compute_caps: &str) -> Result<String> {
    let newest = compute_caps
        .split_whitespace()
        .filter_map(|c| c.parse::<f64>().ok())
        .fold(None, |max: Option<f64>, c| Some(max.map_or(c, |m| m.max(c))));
    let Some(newest) = newest else {
        bail!("No NVIDIA GPU found to pick a Kokoro image (nvidia-smi printed {:?}).", compute_caps.trim());
    };
    let tag = if newest >= 10.0 { "latest-cu128" } else { "latest" };
    Ok(format!("{IMAGE}:{tag}"))
}

/// `docker run` for a container that does not exist yet: herdr-speak's
/// README command, so either tool can start the server the other uses.
/// Bound to localhost only, and restarted with docker after a reboot.
fn docker_run_command(image: &str) -> Vec<String> {
    [
        "docker",
        "run",
        "-d",
        "--name",
        CONTAINER,
        "--restart",
        "unless-stopped",
        "--gpus",
        "all",
        "-p",
        "127.0.0.1:8880:8880",
        image,
    ]
    .map(String::from)
    .to_vec()
}

/// The JSON body for Kokoro's speech endpoint: streamed raw PCM, which the
/// player starts on before the whole reply has arrived.
fn speech_body(input: &str) -> String {
    serde_json::json!({
        "model": MODEL,
        "voice": VOICE,
        "speed": SPEED,
        "response_format": "pcm",
        "stream": true,
        "input": input,
    })
    .to_string()
}

/// The curl argv that streams the speech to stdout. The body goes on stdin,
/// so a long task is not on a command line `ps` shows; `--fail` makes a
/// refused request exit non-zero instead of piping an error page into the
/// player; `--no-buffer` hands audio on as it comes.
fn speech_command() -> Vec<String> {
    [
        "curl",
        "-sS",
        "--fail",
        "--no-buffer",
        "-H",
        "Content-Type: application/json",
        "--data-binary",
        "@-",
        SPEECH_URL,
    ]
    .map(String::from)
    .to_vec()
}

/// The pid file, under `runtime` (`$XDG_RUNTIME_DIR`), so it is per-user and
/// gone after a reboot, when no speech can still be playing. The temp dir
/// stands in without one, as for the daemon's socket.
fn pid_file(runtime: Option<OsString>) -> PathBuf {
    runtime
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("niri-tasks")
        .join("speak.pid")
}

/// Whether `/proc/<pid>/cmdline` contents (NUL-separated arguments) are a
/// background speech: `… speak … --here`. A pid file outlives a crash, and
/// its pid may since belong to anything; only this may be killed.
fn is_speaker(cmdline: &[u8]) -> bool {
    let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
    args.contains(&b"speak".as_slice()) && args.contains(&b"--here".as_slice())
}

/// The argv that stops the speech whose group leader is `pid`. The negative
/// pid names the whole process group, so `claude`, `curl` and `ffplay` stop
/// with it, wherever it had got to.
fn kill_command(pid: u32) -> Vec<String> {
    vec!["kill".into(), "-TERM".into(), "--".into(), format!("-{pid}")]
}

/// The arguments `toggle` starts the background speech with: this same
/// command, with `--here` to say it is the one that speaks.
pub fn worker_args(uuid: &str) -> Vec<String> {
    vec!["task".into(), "speak".into(), uuid.into(), "--here".into()]
}

/// Run `command` with `input` on stdin, killing it once `limit` has passed.
///
/// stdin, stdout and stderr each get a thread, so a long reply cannot fill a
/// pipe and stall the child while this waits on it. The error messages are
/// whole sentences because `main` shows only the outermost one.
fn run_with_timeout(mut command: Command, input: &str, limit: Duration) -> Result<Output> {
    let name = command.get_program().to_string_lossy().into_owned();
    let mut child = match command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!("{name} is not installed."),
        Err(e) => bail!("Could not run {name}: {e}"),
    };
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let input = input.to_owned();
    // A child that exits without reading gives a broken pipe; its exit
    // status, not this, is what reports the failure.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let reader = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    };
    let stdout = reader(Box::new(child.stdout.take().expect("stdout is piped")));
    let stderr = reader(Box::new(child.stderr.take().expect("stderr is piped")));

    let deadline = Instant::now() + limit;
    let status = loop {
        let polled = child
            .try_wait()
            .map_err(|e| anyhow::anyhow!("Could not check whether {name} had finished: {e}"))?;
        if let Some(status) = polled {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("{name} took longer than {}s.", limit.as_secs_f32());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = writer.join();
    Ok(Output {
        status,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

/// The end of a program's stderr, for an error message: the last lines are
/// where the reason is, and a notification has room for little else.
fn tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let text = text.trim();
    let start = text.char_indices().rev().nth(299).map_or(0, |(i, _)| i);
    text[start..].to_string()
}

/// What a failed `claude` says: stderr, or stdout when stderr is empty,
/// because `claude -p` can print its auth and billing errors to stdout.
fn failure_reason(stdout: &[u8], stderr: &[u8]) -> String {
    if stderr.trim_ascii().is_empty() {
        tail(stdout)
    } else {
        tail(stderr)
    }
}

/// Which of the two programs to blame for a failed speech, if either. curl
/// comes first, since a refused request is its failure, unless a signal killed
/// it while the player had failed: a player that dies early closes the pipe
/// and curl gets SIGPIPE, so the player is the cause.
fn blame(curl: ExitStatus, player: ExitStatus) -> Option<Culprit> {
    use std::os::unix::process::ExitStatusExt;
    if !curl.success() && !(curl.signal().is_some() && !player.success()) {
        Some(Culprit::Curl)
    } else if !player.success() {
        Some(Culprit::Player)
    } else {
        None
    }
}

/// The program a failed speech is reported against.
#[derive(Debug, PartialEq)]
enum Culprit {
    Curl,
    Player,
}

/// Whether Kokoro answers its health check as ready. A curl that fails or is
/// missing counts as not ready; starting the server then reports the real
/// problem.
fn kokoro_up() -> bool {
    Command::new("curl")
        .args(["-sS", "--max-time", "3", HEALTH_URL])
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success() && healthy(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false)
}

/// The GPUs' compute capabilities, for picking the image.
fn compute_caps() -> Result<String> {
    let out = Command::new("nvidia-smi")
        .args(["--query-gpu=compute_cap", "--format=csv,noheader"])
        .output();
    match out {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
        Ok(o) => bail!("No NVIDIA GPU found to pick a Kokoro image (nvidia-smi: {}).", tail(&o.stderr)),
        Err(e) => bail!("No NVIDIA GPU found to pick a Kokoro image (nvidia-smi: {e})."),
    }
}

/// Make Kokoro ready, as herdr-speak does: start the container if there is
/// one, run a new one if not, then wait for it to say it is healthy.
fn ensure_kokoro() -> Result<()> {
    if kokoro_up() {
        return Ok(());
    }
    anyhow::ensure!(
        project::on_path("docker"),
        "Kokoro isn't running, and docker is not installed to start it."
    );
    let exists = Command::new("docker")
        .args(["container", "inspect", CONTAINER])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    let argv = if exists {
        vec!["docker".to_string(), "start".to_string(), CONTAINER.to_string()]
    } else {
        docker_run_command(&kokoro_image(&compute_caps()?)?)
    };
    notify::tasks("Starting Kokoro — this can take a minute.");
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|e| anyhow::anyhow!("Could not run docker to start Kokoro: {e}"))?;
    if !out.status.success() {
        bail!("{} failed: {}", argv[..2].join(" "), tail(&out.stderr));
    }
    let deadline = Instant::now() + START_TIMEOUT;
    while Instant::now() < deadline {
        if kokoro_up() {
            return Ok(());
        }
        std::thread::sleep(START_POLL);
    }
    bail!("Kokoro didn't become healthy within {} seconds.", START_TIMEOUT.as_secs());
}

/// Have Claude turn the task into something to listen to.
fn rewrite(input: &str) -> Result<String> {
    let argv = rewrite_command(PROMPT);
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    for key in dropped_vars(std::env::vars_os().filter_map(|(k, _)| k.into_string().ok())) {
        command.env_remove(key);
    }
    // Thinking only delays the first word.
    command.env("MAX_THINKING_TOKENS", "0");
    let out = run_with_timeout(command, input, REWRITE_TIMEOUT)?;
    if !out.status.success() {
        bail!("claude failed to rewrite the task for speaking ({}): {}", out.status, failure_reason(&out.stdout, &out.stderr));
    }
    let script = String::from_utf8_lossy(&out.stdout).trim().to_string();
    anyhow::ensure!(!script.is_empty(), "claude gave back nothing to say.");
    Ok(script)
}

/// Speak `script`: Kokoro's streamed PCM from curl, straight into ffplay.
/// curl is blamed first (see `blame`): a refused request is its failure,
/// whatever the player made of the empty stream.
fn play(script: &str) -> Result<()> {
    let argv = speech_command();
    let mut curl = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Could not run curl: {e}"))?;
    {
        // curl reads the whole body before it sends anything, so writing it
        // all here cannot wait on the player.
        let mut stdin = curl.stdin.take().expect("stdin is piped");
        stdin
            .write_all(speech_body(script).as_bytes())
            .map_err(|e| anyhow::anyhow!("Could not hand curl the speech request: {e}"))?;
    }
    let audio = curl.stdout.take().expect("stdout is piped");
    let player = Command::new(PLAYER[0])
        .args(&PLAYER[1..])
        .stdin(audio)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| anyhow::anyhow!("Could not run ffplay: {e}"))?;
    let curl = curl
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("Could not wait for curl to finish: {e}"))?;
    match blame(curl.status, player) {
        Some(Culprit::Curl) => {
            bail!("curl failed to fetch the speech from Kokoro ({}): {}", curl.status, tail(&curl.stderr))
        }
        Some(Culprit::Player) => bail!("ffplay failed to play the speech ({player})."),
        None => Ok(()),
    }
}

/// The pid of the speech playing now, if one is.
fn speaking(path: &Path) -> Option<u32> {
    let pid: u32 = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    is_speaker(&cmdline).then_some(pid)
}

/// Removes the pid file when the background speech ends, however it ends —
/// but only while the file still names this process, so it never deletes the
/// pid of a speech started after it.
struct PidFile {
    path: PathBuf,
}

impl Drop for PidFile {
    fn drop(&mut self) {
        let ours = std::fs::read_to_string(&self.path)
            .is_ok_and(|s| s.trim() == std::process::id().to_string());
        if ours {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// `niritasks task speak <uuid>`: stop the speech playing now, or start one
/// for `uuid` in the background and return at once.
///
/// The background process is started in its own process group, so a stop
/// takes everything it started with it. Its pid is written here, not by the
/// process itself, so a second press straight after the first already finds
/// it. What it needs is checked first, so a missing program fails on the
/// press rather than as a later notification.
pub fn toggle(uuid: &str, description: &str) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let path = pid_file(std::env::var_os("XDG_RUNTIME_DIR"));
    if let Some(pid) = speaking(&path) {
        let argv = kill_command(pid);
        let status = Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map_err(|e| anyhow::anyhow!("Could not stop the speech: {e}"))?;
        // The speech may have ended between the check and the kill, which
        // is as good as stopped.
        anyhow::ensure!(
            status.success() || !Path::new(&format!("/proc/{pid}")).exists(),
            "Could not stop the speech (kill {status})."
        );
        let _ = std::fs::remove_file(&path);
        notify::tasks("Stopped speaking.");
        return Ok(());
    }

    for program in ["claude", "curl", PLAYER[0]] {
        anyhow::ensure!(project::on_path(program), "{program} is not installed.");
    }
    let exe = std::env::current_exe().context("could not find the niritasks binary")?;
    // The directory is made before the speech starts, so the likeliest write
    // failure is found while there is nothing to clean up.
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("Could not create the speech's pid directory {}: {e}", dir.display()))?;
    }
    let child = Command::new(exe)
        .args(worker_args(uuid))
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Could not start speaking: {e}"))?;
    if let Err(e) = std::fs::write(&path, child.id().to_string()) {
        // Without its pid on file a second press could not find this speech
        // to stop it, and would start another on top, so it is stopped now.
        let argv = kill_command(child.id());
        let _ = Command::new(&argv[0]).args(&argv[1..]).status();
        bail!("Could not write the speech's pid file {}: {e}", path.display());
    }
    notify::tasks(&format!("Speaking: {description}"));
    Ok(())
}

/// `niritasks task speak <uuid> --here`: the background speech itself.
/// Kokoro is made ready before the rewrite, so a broken docker fails before
/// a claude call is spent.
pub fn run(uuid: &str) -> Result<()> {
    let _pid = PidFile { path: pid_file(std::env::var_os("XDG_RUNTIME_DIR")) };
    let t = task::get(uuid)?.with_context(|| format!("No task {uuid}."))?;
    ensure_kokoro()?;
    let notes: Vec<&str> = t.annotations.iter().map(|a| a.description.as_str()).collect();
    let script = rewrite(&rewrite_input(&t.description, &notes))?;
    play(&script)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    /// The description first, then each note on its own line, as the
    /// prompt tells Claude to expect.
    #[test]
    fn the_rewrite_reads_the_description_then_one_line_per_note() {
        assert_eq!(
            rewrite_input("Add a Speak action", &["Goal: listen to a task", "Decided: audio only"]),
            "Add a Speak action\nGoal: listen to a task\nDecided: audio only\n"
        );
        assert_eq!(rewrite_input("Just this", &[]), "Just this\n");
    }

    /// A note that spans lines would read as several notes, so its line
    /// breaks are folded to spaces.
    #[test]
    fn a_note_with_line_breaks_stays_one_line() {
        assert_eq!(rewrite_input("D", &["one\ntwo"]), "D\none two\n");
    }

    /// The spec's exact flags: no tools, no sessions saved, no settings, MCP
    /// or slash commands of the user's leaking into a rephrase.
    #[test]
    fn the_rewrite_runs_a_bare_haiku() {
        assert_eq!(
            rewrite_command("PROMPT"),
            strings(&[
                "claude",
                "-p",
                "--model",
                "haiku",
                "--no-session-persistence",
                "--tools",
                "",
                "--disable-slash-commands",
                "--strict-mcp-config",
                "--setting-sources",
                "",
                "--system-prompt",
                "PROMPT",
            ])
        );
    }

    /// The prompt is the file in the repo, not a copy that could drift.
    #[test]
    fn the_rewrite_uses_the_prompt_file() {
        assert!(rewrite_command(PROMPT).contains(&PROMPT.to_string()));
        assert!(PROMPT.contains("Output only the spoken script"));
    }

    /// Run from a herdr pane, the rewrite would otherwise inherit that pane's
    /// HERDR_* and could act on it.
    #[test]
    fn only_herdr_variables_are_dropped() {
        let keys = strings(&["HERDR_PANE_ID", "PATH", "HERDR_SESSION", "HOME", "XHERDR_X"]);
        assert_eq!(dropped_vars(keys), strings(&["HERDR_PANE_ID", "HERDR_SESSION"]));
    }

    #[test]
    fn kokoro_is_ready_only_when_it_says_healthy() {
        assert!(healthy(r#"{"status":"healthy"}"#));
        assert!(!healthy(r#"{"status":"starting"}"#));
        assert!(!healthy("Bad Gateway"));
        assert!(!healthy(""));
    }

    /// Blackwell (compute capability 10 and up) needs the CUDA 12.8 build;
    /// older cards the default. With several GPUs, the newest decides.
    #[test]
    fn the_image_follows_the_gpu() {
        assert_eq!(kokoro_image("12.0\n").unwrap(), "ghcr.io/remsky/kokoro-fastapi-gpu:latest-cu128");
        assert_eq!(kokoro_image("10.0\n").unwrap(), "ghcr.io/remsky/kokoro-fastapi-gpu:latest-cu128");
        assert_eq!(kokoro_image("8.9\n").unwrap(), "ghcr.io/remsky/kokoro-fastapi-gpu:latest");
        assert_eq!(kokoro_image("8.6\n12.0\n").unwrap(), "ghcr.io/remsky/kokoro-fastapi-gpu:latest-cu128");
    }

    #[test]
    fn no_gpu_is_an_error_naming_it() {
        let e = kokoro_image("").unwrap_err().to_string();
        assert!(e.contains("No NVIDIA GPU"), "{e}");
        assert!(kokoro_image("No devices were found").is_err());
    }

    /// herdr-speak's README command, so either tool can start the server the
    /// other uses.
    #[test]
    fn a_new_container_is_run_as_herdr_speak_runs_it() {
        assert_eq!(
            docker_run_command("img:tag"),
            strings(&[
                "docker",
                "run",
                "-d",
                "--name",
                "kokoro-tts",
                "--restart",
                "unless-stopped",
                "--gpus",
                "all",
                "-p",
                "127.0.0.1:8880:8880",
                "img:tag",
            ])
        );
    }

    #[test]
    fn the_speech_request_streams_pcm_in_af_bella() {
        let v: serde_json::Value = serde_json::from_str(&speech_body("Hello \"there\"")).unwrap();
        assert_eq!(v["model"], "kokoro");
        assert_eq!(v["voice"], "af_bella");
        assert_eq!(v["speed"], 1.0);
        assert_eq!(v["response_format"], "pcm");
        assert_eq!(v["stream"], true);
        assert_eq!(v["input"], "Hello \"there\"");
    }

    /// The body goes on stdin, not argv, so a long task is not in `ps`; a
    /// failed request exits non-zero rather than piping an error page into
    /// the player.
    #[test]
    fn curl_posts_the_body_from_stdin_and_fails_loudly() {
        assert_eq!(
            speech_command(),
            strings(&[
                "curl",
                "-sS",
                "--fail",
                "--no-buffer",
                "-H",
                "Content-Type: application/json",
                "--data-binary",
                "@-",
                "http://127.0.0.1:8880/v1/audio/speech",
            ])
        );
    }

    #[test]
    fn the_player_is_herdr_speaks_default() {
        assert_eq!(
            PLAYER.join(" "),
            "ffplay -nodisp -autoexit -loglevel quiet -f s16le -ar 24000 -ch_layout mono -i -"
        );
    }

    #[test]
    fn the_pid_file_lives_under_the_runtime_dir() {
        assert_eq!(
            pid_file(Some(OsString::from("/run/user/1000"))),
            PathBuf::from("/run/user/1000/niri-tasks/speak.pid")
        );
        assert_eq!(pid_file(None), std::env::temp_dir().join("niri-tasks/speak.pid"));
    }

    /// A pid left by a crash may have been reused by any process; only the
    /// background speech itself may be killed.
    #[test]
    fn only_a_background_speech_counts_as_speaking() {
        assert!(is_speaker(b"/usr/bin/niritasks\0task\0speak\0c53b6e3d\0--here\0"));
        assert!(!is_speaker(b"/usr/bin/niritasks\0task\0speak\0c53b6e3d\0"), "the toggle itself");
        assert!(!is_speaker(b"/usr/bin/firefox\0--here\0"));
        assert!(!is_speaker(b""));
    }

    /// The negative pid names the process group, so claude, curl and ffplay
    /// stop with it.
    #[test]
    fn stopping_signals_the_whole_group() {
        assert_eq!(kill_command(4242), strings(&["kill", "-TERM", "--", "-4242"]));
    }

    #[test]
    fn the_background_speech_is_task_speak_here() {
        assert_eq!(worker_args("c53b6e3d"), strings(&["task", "speak", "c53b6e3d", "--here"]));
    }

    /// stdin goes in, stdout and the exit status come back.
    #[test]
    fn a_timed_command_gets_its_input_and_returns_its_output() {
        let out = run_with_timeout(Command::new("cat"), "spoken\n", Duration::from_secs(5)).unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout, b"spoken\n");
    }

    /// A rewrite that hangs is killed, and the error says which program and
    /// how long it had.
    #[test]
    fn a_timed_command_that_runs_over_is_killed() {
        let mut sleep = Command::new("sleep");
        sleep.arg("5");
        let started = Instant::now();
        let e = run_with_timeout(sleep, "", Duration::from_millis(200)).unwrap_err().to_string();
        assert!(started.elapsed() < Duration::from_secs(3), "it waited for sleep to finish");
        assert!(e.contains("sleep took longer than"), "{e}");
    }

    /// A program that is not installed says so, rather than "No such file".
    #[test]
    fn a_missing_program_is_named_as_not_installed() {
        let e = run_with_timeout(Command::new("niritasks-no-such-program"), "", Duration::from_secs(1))
            .unwrap_err()
            .to_string();
        assert!(e.contains("niritasks-no-such-program is not installed"), "{e}");
    }

    fn exited(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    fn killed(signal: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(signal)
    }

    /// A player that dies early closes the pipe and curl dies of SIGPIPE;
    /// that is the player's failure, not curl's.
    #[test]
    fn a_curl_killed_by_a_signal_after_the_player_failed_blames_the_player() {
        assert_eq!(blame(killed(13), exited(1)), Some(Culprit::Player));
    }

    /// A refused request is curl's failure, whatever the player made of it.
    #[test]
    fn a_curl_that_exited_with_an_error_is_blamed_even_when_the_player_failed() {
        assert_eq!(blame(exited(22), exited(1)), Some(Culprit::Curl));
        assert_eq!(blame(exited(22), exited(0)), Some(Culprit::Curl));
    }

    #[test]
    fn a_failed_player_with_a_clean_curl_is_the_players() {
        assert_eq!(blame(exited(0), exited(1)), Some(Culprit::Player));
    }

    #[test]
    fn nothing_is_blamed_when_both_succeed() {
        assert_eq!(blame(exited(0), exited(0)), None);
    }

    /// claude -p can print its errors to stdout, leaving stderr empty.
    #[test]
    fn a_claude_failure_falls_back_to_stdout_when_stderr_is_blank() {
        assert_eq!(failure_reason(b"Invalid API key\n", b"  \n"), "Invalid API key");
        assert_eq!(failure_reason(b"ignored", b"boom\n"), "boom");
    }
}
