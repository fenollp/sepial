use std::io;

use serial2_tokio::SerialPort;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

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
                        handle(&raw[..pos]);
                        break;
                    }
                    Ok(n) => {
                        println!("{n} bytes.");
                        pos += n;

                        let mut start = 0;
                        while let Some(i) = raw[start..pos].iter().position(|&c| c == b'\n') {
                            handle(&raw[start..(start + i)]);
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

fn handle(line: &[u8]) {
    if let Ok(line) = str::from_utf8(line) {
        println!("> {line:?}");
        return;
    }
    println!("> GARBAGE");
}
