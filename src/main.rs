use serial2_tokio::SerialPort;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Hello, world!");

    println!("Available ports: {:?}", SerialPort::available_ports()?);

    // On Windows, use something like "COM1" or "COM15".
    //
    // python3 -m serial.tools.miniterm -e /dev/ttyACM0 250000
    //
    let port_name = "/dev/ttyACM0";
    let baud_rate = 250000;

    print!("Connecting to {port_name} at {baud_rate}...");
    let port = SerialPort::open(port_name, baud_rate)?;
    println!(" ok!");

    // start
    // Marlin bugfix-2.1.x
    // echo: Last Updated: 2023-01-27 | Author: (Marginally Clever, Makelangelo 5 Huge)
    // echo: Compiled: Nov  3 2023
    // echo: Free Memory: 4012  PlannerBufferBytes: 1152
    // //action:notification Polargraph Ready.
    // //action:prompt_end
    // echo:SD card ok

    let mut buffer = [0u8; 512];
    loop {
        print!("  Reading...");
        match port.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                println!(" {n} bytes.");
                let mut bufstr = str::from_utf8(&buffer[..n])?;

                while let Some((lhs, rst)) = bufstr.split_once('\n') {
                    println!("> {lhs}");
                    bufstr = rst;
                }

                // port.write_all(&buffer[..n]).await?;
                // println!("Wrote {} bytes", &buffer[..n].len());
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                print!(".");
                continue;
            }
            Err(e) => return Err(e)?,
        }
    }

    println!("Exited.");
    Ok(())
}
