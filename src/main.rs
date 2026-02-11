use serial2_tokio::SerialPort;
use std::io;
use tokio::select;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Available ports: {:?}", SerialPort::available_ports()?);

    // On Windows, use something like "COM1" or "COM15".
    //
    // python3 -m serial.tools.miniterm -e /dev/ttyACM0 250000
    //
    let port_name = "/dev/ttyACM0";
    let baud_rate = 250000;

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

    let mut raw = [0u8; 512];
    let mut rest = String::new();
    let mut sig = signal(SignalKind::interrupt())?;
    loop {
        print!("  Reading... ");
        fn handle(line: &str) {
            println!("> {line:?}");
        }
        select! {
            _ = sig.recv() => break,
            r = port.read(&mut raw) => {
                match r {
                    Ok(0) => {
                        handle(&rest);
                        break;
                    }
                    Ok(n) => {
                        println!("{n} bytes.");
                        let chunk = String::from_utf8_lossy(&raw[..n]);
                        rest.push_str(&chunk);

                        while let Some(i) = rest.find('\n') {
                            let line: String = rest.drain(..=i).collect();
                            handle(line.trim_end_matches('\n'));
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        print!(".");
                        continue;
                    }
                    Err(e) => return Err(e)?,
                }
            }
        }
    }

    println!("Exited.");
    Ok(())
}
