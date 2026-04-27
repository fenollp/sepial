use std::{
    collections::VecDeque,
    env,
    fmt::{self, Display},
    io,
};

use anyhow::{Result, anyhow, bail};
use dialoguer::Confirm;
use serial2_tokio::SerialPort;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
    select,
    signal::unix::{SignalKind, signal},
};
use tracing::{debug, error, info, trace};
use tracing_indicatif::{IndicatifLayer, suspend_tracing_indicatif};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const SEPIAL_BAUD: &str = "SEPIAL_BAUD";
const SEPIAL_GCODE: &str = "SEPIAL_GCODE";
const SEPIAL_PORT: &str = "SEPIAL_PORT";

// start
// Marlin bugfix-2.1.x
// echo: Last Updated: 2023-01-27 | Author: (Marginally Clever, Makelangelo 5 Huge)
// echo: Compiled: Nov  3 2023
// echo: Free Memory: 4012  PlannerBufferBytes: 1152
// //action:notification Polargraph Ready.
// //action:prompt_end
// echo:SD card ok

const PEN_UP: Req = Req::Pen(90, 250);
#[expect(dead_code)]
const PEN_DOWN: Req = Req::Pen(25, 150);

#[derive(Clone, Debug)]
enum Req {
    Heartbeat,
    PromptsSupported,
    PromptAnswerContinue,
    Pen(u8 /* 0..180 */, u16),
    MotorsEngage,
    MotorsDisengage,
    FindHome,
    Raw(String),   // passthrough
    EmergencyStop, // https://marlinfw.org/docs/gcode/M112.html
    Die,           // not an actual Marlin command
}
impl Display for Req {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Heartbeat => write!(f, "M400"),
            Self::PromptsSupported => write!(f, "M876 P1"),
            Self::PromptAnswerContinue => write!(f, "M876 S0"),
            Self::Pen(angle, ms) => write!(f, "M280 P0 S{angle} T{ms}"),
            Self::MotorsEngage => write!(f, "M17"),
            Self::MotorsDisengage => write!(f, "M18"),
            Self::FindHome => write!(f, "G28 X Y"),
            Self::Raw(line) => write!(f, "{line}"),
            Self::EmergencyStop => write!(f, "M112"),
            Self::Die => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
struct State {
    ready: Option<bool>,
    reqs: VecDeque<Req>,
    last_line_number_sent: u32,
    gcode: Option<BufReader<File>>,
}
impl State {
    fn new(gcode: BufReader<File>) -> Self {
        Self {
            gcode: Some(gcode),
            reqs: [Req::Heartbeat, Req::PromptsSupported, PEN_UP, Req::MotorsEngage, Req::FindHome]
                .into(),
            ..Self::default()
        }
    }

    async fn send(&mut self, port: &SerialPort) -> Result<Option<Req>> {
        if self.ready.is_some_and(|ready| ready)
            && let Some(req) = self.reqs.pop_front()
        {
            if matches!(req, Req::Die) {
                return Ok(Some(req));
            }
            self.ready = Some(false);
            self.last_line_number_sent += 1;
            debug!(">> #{} {req:?}: {req}", self.last_line_number_sent);
            port.write_all(format!("{req}\n").as_bytes()).await?;
            return Ok(Some(req));
        }
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        env::set_var("RUST_LOG", env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_owned()))
    };

    let indicatif_layer = IndicatifLayer::new();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
        .with(EnvFilter::from_default_env())
        .init();

    info!("Available ports: {:?}", SerialPort::available_ports()?);

    let port_name = env::var(SEPIAL_PORT).unwrap_or("/dev/ttyACM0".to_owned());
    let baud_rate = env::var(SEPIAL_BAUD).map(|x| x.parse().unwrap()).unwrap_or(250000);

    info!("Connecting to {port_name} at {baud_rate}... ");
    let port = SerialPort::open(&port_name, baud_rate)
        .map_err(|e| anyhow!("Could not talk Serial with {port_name} @ {baud_rate}: {e}"))?;
    info!("ok!");

    let Ok(gcode) = env::var(SEPIAL_GCODE) else {
        bail!("Provide a gcode file with ${SEPIAL_GCODE}=")
    };
    let gcode = BufReader::new(File::open(gcode).await?);
    let mut state = State::new(gcode);

    let mut pos = 0;
    let mut raw = [0u8; 512];
    let mut sig = signal(SignalKind::interrupt())?;
    loop {
        trace!("  Reading... ");
        select! {
            _ = sig.recv() => {
                state.reqs.push_front(Req::EmergencyStop);
                state.reqs.push_front(Req::EmergencyStop);
                state.reqs.push_front(Req::EmergencyStop);
            }

            r = port.read(&mut raw[pos..]) => {
                match r {
                    Ok(0) if pos == 0 => break,
                    Ok(0) => {
                        let _ = handle(&port, &mut state, &raw[..pos]).await?;
                        break;
                    }
                    Ok(n) => {
                        trace!("{n} bytes.");
                        pos += n;

                        let mut start = 0;
                        while let Some(i) = raw[start..pos].iter().position(|&c| c == b'\n') {
                            if handle(&port, &mut state, &raw[start..(start + i)]).await? {
                                return Ok(())
                            }
                            start += i + 1;
                        }
                        assert!(start <= pos);
                        if start > 0 {
                            raw.copy_within(start..pos, 0); // Move the rest back up
                            pos -= start;
                        }
                        assert_ne!(pos, raw.len(), "Line too long? ({pos}) {raw:?}");
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        trace!("{}", if pos == 0 { "." } else { "!" });
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }

    info!("Exited.");
    Ok(())
}

async fn handle(port: &SerialPort, state: &mut State, line: &[u8]) -> Result<bool> {
    let Ok(line) = str::from_utf8(line) else { bail!("> GARBAGE! Check baud rate? {line:?}") };
    debug!("> {line:?}");

    match line {
        "//action:notification Polargraph Ready." => {
            info!("Ready!");
            state.ready = Some(true);
        }
        "ok" => {
            trace!("   #{} ack'd", state.last_line_number_sent);
            state.ready = Some(true);
        }
        "//action:prompt_show" => {
            // Since we support prompts, we're expected to reply something here:
            // >> #11 Raw("M0 Ready black and click"): M0 Ready black and click
            // > "//action:notification Ready black and click\r"
            // > "//action:prompt_end"
            // > "//action:prompt_begin Ready black and click"
            // > "//action:prompt_button Continue"
            // > "//action:prompt_show"
            // > "echo:busy: paused for user"
            //... let's try Continue and just hope!
            suspend_tracing_indicatif(|| {
                let _ = Confirm::new().with_prompt("Load a pen then press").interact();
                // TODO? Exit when returns false
            });
            state.ready = Some(true);
            state.reqs.push_front(Req::PromptAnswerContinue);
        }
        _ if line.starts_with("Error:") => {
            if line.contains("Printer halted") {
                bail!("Fatal: {line}")
            }
            error!("Stopping! Should we be continuing?");
            state.ready = Some(false);
        }
        _ => {}
    }

    if state.ready.is_some_and(|ready| ready) && state.reqs.is_empty() {
        info!("  Loading... ");
        let mut count = 0;
        if let Some(gcode) = state.gcode.take() {
            let mut lines = gcode.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.is_empty() || line.trim().starts_with(';') {
                    continue;
                }
                if line == format!("{}", Req::FindHome) && count == 0 {
                    // At this point we're already home
                    continue;
                }
                count += 1;
                state.reqs.push_back(Req::Raw(line));
            }
        }
        info!("{count} GCODE lines!");
        state.reqs.extend([PEN_UP, Req::FindHome, Req::MotorsDisengage, Req::Die]);
        if count != 0 {
            info!("  Drawing!");
        }
    }

    if let Some(req) = state.send(port).await? {
        return Ok(matches!(req, Req::Die));
    }
    Ok(false)
}
