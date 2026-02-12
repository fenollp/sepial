use std::{collections::VecDeque, env, fmt, fmt::Display, io};

use anyhow::bail;
use serial2_tokio::SerialPort;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

const SEPIAL_PORT: &str = "SEPIAL_PORT";
const SEPIAL_BAUD: &str = "SEPIAL_BAUD";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
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

    let mut state =
        State { reqs: [Req::Heartbeat, Req::IHandleDialogs].into(), ..State::default() };

    let mut pos = 0;
    let mut raw = [0u8; 512];
    let mut sig = signal(SignalKind::interrupt())?;
    loop {
        print!("  Reading... ");
        select! {
            _ = sig.recv() => break,
            r = port.read(&mut raw[pos..]) => {
                match r {
                    Ok(0) if pos == 0 => break,
                    Ok(0) => {
                        handle(&port, &mut state, &raw[..pos]).await?;
                        break;
                    }
                    Ok(n) => {
                        println!("{n} bytes.");
                        pos += n;

                        let mut start = 0;
                        while let Some(i) = raw[start..pos].iter().position(|&c| c == b'\n') {
                            handle(&port, &mut state, &raw[start..(start + i)]).await?;
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Req {
    Heartbeat,
    IHandleDialogs,
    PenUp(u8 /* 0..180 */, u16),
    PenDown(u8 /* 0..180 */, u16),
    MotorsEngage,
    MotorsDisengage,
    FindHome,
}
impl Display for Req {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Heartbeat => write!(f, "M400"),
            Self::IHandleDialogs => write!(f, "M876 P1"),
            Self::PenUp(angle, ms) | Self::PenDown(angle, ms) => {
                write!(f, "M280 P0 S{angle} T{ms}")
            }
            Self::MotorsEngage => write!(f, "M17"),
            Self::MotorsDisengage => write!(f, "M18"),
            Self::FindHome => write!(f, "G28 X Y"),
        }
    }
}

#[derive(Debug, Default)]
struct State {
    ready: Option<bool>,
    reqs: VecDeque<Req>,
    last_line_number_sent: u32,
}
impl State {
    async fn send(&mut self, port: &SerialPort) -> io::Result<Option<Req>> {
        if self.ready.is_some_and(|ready| ready)
            && let Some(req) = self.reqs.pop_front()
        {
            self.ready = Some(false);
            self.last_line_number_sent += 1;
            println!(">> #{} {req:?}: {req}", self.last_line_number_sent);
            port.write_all(format!("{req}\n").as_bytes()).await?;
            return Ok(Some(req));
        }
        Ok(None)
    }
}

async fn handle(port: &SerialPort, state: &mut State, line: &[u8]) -> anyhow::Result<()> {
    let Ok(line) = str::from_utf8(line) else {
        println!("> GARBAGE! Check baud rate? {line:?}");
        return Ok(());
    };
    println!("> {line:?}");

    match line {
        "//action:notification Polargraph Ready." => {
            println!("Ready!");
            assert!(state.ready.is_none(), "State is already initialized: {state:?}");
            state.ready = Some(true);
        }
        "ok" => {
            println!("   #{} ack'd", state.last_line_number_sent);
            state.ready = Some(true);
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

    if let Some(req) = state.send(port).await? {
        const PEN_UP: Req = Req::PenUp(90, 250);
        const PEN_DOWN: Req = Req::PenDown(25, 150);
        if req == Req::IHandleDialogs {
            state.reqs.extend([PEN_UP, Req::MotorsDisengage, Req::FindHome].into_iter());
        }
    }

    Ok(())
}
