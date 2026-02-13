use std::{collections::VecDeque, env, fmt, fmt::Display, io};

use anyhow::{Result, bail};
use serial2_tokio::SerialPort;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

const SEPIAL_PORT: &str = "SEPIAL_PORT";
const SEPIAL_BAUD: &str = "SEPIAL_BAUD";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Available ports: {:?}", SerialPort::available_ports()?);

    // On Windows, use something like "COM1" or "COM15".
    //
    // python3 -m serial.tools.miniterm -e /dev/ttyACM0 250000
    //
    let port_name = env::var(SEPIAL_PORT).unwrap_or("/dev/ttyACM0".to_owned());
    let baud_rate = env::var(SEPIAL_BAUD).map(|x| x.parse().unwrap()).unwrap_or(250000);

    print!("Connecting to {port_name} at {baud_rate}... ");
    let port = SerialPort::open(port_name, baud_rate)?;
    println!("ok!");

    // start
    // Marlin bugfix-2.1.x
    // echo: Last Updated: 2023-01-27 | Author: (Marginally Clever, Makelangelo 5 Huge)
    // echo: Compiled: Nov  3 2023
    // echo: Free Memory: 4012  PlannerBufferBytes: 1152
    // //action:notification Polargraph Ready.
    // //action:prompt_end
    // echo:SD card ok

    let mut state = State {
        reqs: [Req::Heartbeat, Req::PromptsSupported, PEN_UP, Req::MotorsEngage, Req::FindHome]
            .into(),
        ..State::default()
    };

    let mut pos = 0;
    let mut raw = [0u8; 512];
    let mut sig = signal(SignalKind::interrupt())?;
    loop {
        print!("  Reading... ");
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
                        println!("{n} bytes.");
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
                        print!("{}", if pos == 0 { "." } else { "!" });
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }

    println!("Exited.");
    Ok(())
}

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
    started_drawing: bool,
}
impl State {
    async fn send(&mut self, port: &SerialPort) -> Result<Option<Req>> {
        if self.ready.is_some_and(|ready| ready)
            && let Some(req) = self.reqs.pop_front()
        {
            if matches!(req, Req::Die) {
                return Ok(Some(req));
            }
            self.ready = Some(false);
            self.last_line_number_sent += 1;
            println!(">> #{} {req:?}: {req}", self.last_line_number_sent);
            port.write_all(format!("{req}\n").as_bytes()).await?;
            return Ok(Some(req));
        }
        Ok(None)
    }
}

async fn handle(port: &SerialPort, state: &mut State, line: &[u8]) -> Result<bool> {
    let Ok(line) = str::from_utf8(line) else { bail!("> GARBAGE! Check baud rate? {line:?}") };
    println!("> {line:?}");

    match line {
        "//action:notification Polargraph Ready." => {
            println!("Ready!");
            state.ready = Some(true);
        }
        "ok" => {
            println!("   #{} ack'd", state.last_line_number_sent);
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
            println!("HACK");
            state.ready = Some(true);
            state.reqs.push_front(Req::PromptAnswerContinue);
        }
        _ if line.starts_with("Error:") => {
            if line.contains("Printer halted") {
                bail!("Fatal: {line}")
            }
            println!("Stopping! Should we be continuing?");
            state.ready = Some(false);
        }
        _ => {}
    }

    if state.ready.is_some_and(|ready| ready) && state.reqs.is_empty() && !state.started_drawing {
        print!("  Loading... ");
        let mut count = 0;
        let mut lines = io::stdin().lines();
        while let Some(Ok(line)) = lines.next() {
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
        println!("{count} GCODE lines!");
        state.started_drawing = true;
        state.reqs.extend([PEN_UP, Req::FindHome, Req::MotorsDisengage, Req::Die].into_iter());
        if count != 0 {
            println!("  Drawing!");
        }
    }

    if let Some(req) = state.send(port).await? {
        return Ok(matches!(req, Req::Die));
    }
    Ok(false)
}
