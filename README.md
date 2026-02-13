# sepial ~ Plotting via the Serial port
Stream GCODE to my Makelangelo 5 HUGE via WiFi via an ESP32C6 plugged in USB to Serial

```shell
cargo install --locked --force --git https://github.com/fenollp/sepial.git --branch=main

SEPIAL_PORT=/dev/ttyACM0 SEPIAL_BAUD=250000 RUST_LOG=debug sepial <blobs/circle.gcode
2026-02-13T11:18:02.953178Z  INFO sepial: Available ports: ["/dev/ttyS15", "/dev/ttyS6", "/dev/ttyS23", "/dev/ttyS13", "/dev/ttyS31", "/dev/ttyS4", "/dev/ttyS21", "/dev/ttyS11", "/dev/ttyS2", "/dev/ttyS28", "/dev/ttyS0", "/dev/ttyS18", "/dev/ttyS9", "/dev/ttyS26", "/dev/ttyS16", "/dev/ttyACM0", "/dev/ttyS7", "/dev/ttyS24", "/dev/ttyS14", "/dev/ttyS5", "/dev/ttyS22", "/dev/ttyS12", "/dev/ttyS30", "/dev/ttyS3", "/dev/ttyS20", "/dev/ttyS10", "/dev/ttyS29", "/dev/ttyS1", "/dev/ttyS19", "/dev/ttyS27", "/dev/ttyS17", "/dev/ttyS8", "/dev/ttyS25", "/dev/ttyprintk"]
2026-02-13T11:18:02.953254Z  INFO sepial: Connecting to /dev/ttyACM0 at 250000...
2026-02-13T11:18:02.953461Z  INFO sepial: ok!
2026-02-13T11:18:03.866257Z DEBUG sepial: > "start"
2026-02-13T11:18:03.869925Z DEBUG sepial: > "Marlin bugfix-2.1.x"
2026-02-13T11:18:03.874455Z DEBUG sepial: > "echo: Last Updated: 2023-01-27 | Author: (Marginally Clever, Makelangelo 5 Huge)"
2026-02-13T11:18:03.874513Z DEBUG sepial: > "echo: Compiled: Nov  3 2023"
2026-02-13T11:18:03.877407Z DEBUG sepial: > "echo: Free Memory: 4012  PlannerBufferBytes: 1152"
2026-02-13T11:18:03.997448Z DEBUG sepial: > "//action:notification Polargraph Ready."
2026-02-13T11:18:03.997510Z  INFO sepial: Ready!
2026-02-13T11:18:03.997526Z DEBUG sepial: >> #1 Heartbeat: M400
2026-02-13T11:18:07.024288Z DEBUG sepial: > "//action:prompt_end"
2026-02-13T11:18:07.642725Z DEBUG sepial: > "echo:SD card ok"
2026-02-13T11:18:07.765499Z DEBUG sepial: > "ok"
2026-02-13T11:18:07.765554Z DEBUG sepial: >> #2 PromptsSupported: M876 P1
2026-02-13T11:18:07.768605Z DEBUG sepial: > "ok"
2026-02-13T11:18:07.768660Z DEBUG sepial: >> #3 Pen(90, 250): M280 P0 S90 T250
2026-02-13T11:18:08.772051Z DEBUG sepial: > "ok"
2026-02-13T11:18:08.772107Z DEBUG sepial: >> #4 MotorsEngage: M17
2026-02-13T11:18:08.780839Z DEBUG sepial: > "//action:notification No Move."
2026-02-13T11:18:08.780896Z DEBUG sepial: > "ok"
2026-02-13T11:18:08.780910Z DEBUG sepial: >> #5 FindHome: G28 X Y
2026-02-13T11:18:10.784288Z DEBUG sepial: > "echo:busy: processing"
2026-02-13T11:18:12.786289Z DEBUG sepial: > "echo:busy: processing"
2026-02-13T11:18:14.785927Z DEBUG sepial: > "echo:busy: processing"
2026-02-13T11:18:16.784709Z DEBUG sepial: > "echo:busy: processing"
2026-02-13T11:18:18.787598Z DEBUG sepial: > "echo:busy: processing"
...8<...
2026-02-13T11:19:15.838723Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.838784Z DEBUG sepial: >> #2012 Raw("G1 X69.998 Y-0.565 F3000.0"): G1 X69.998 Y-0.565 F3000.0
2026-02-13T11:19:15.851005Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.851065Z DEBUG sepial: >> #2013 Raw("G1 X69.999 Y-0.377 F3000.0"): G1 X69.999 Y-0.377 F3000.0
2026-02-13T11:19:15.858459Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.858518Z DEBUG sepial: >> #2014 Raw("G1 X70.000 Y-0.189 F3000.0"): G1 X70.000 Y-0.189 F3000.0
2026-02-13T11:19:15.866315Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.866375Z DEBUG sepial: >> #2015 Raw("G1 X70.000 Y0.000 F3000.0"): G1 X70.000 Y0.000 F3000.0
2026-02-13T11:19:15.875573Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.875632Z DEBUG sepial: >> #2016 Raw("M280 P0 S90.0 T150.0"): M280 P0 S90.0 T150.0
2026-02-13T11:19:15.883536Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.883597Z DEBUG sepial: >> #2017 Pen(90, 250): M280 P0 S90 T250
2026-02-13T11:19:15.890960Z DEBUG sepial: > "ok"
2026-02-13T11:19:15.891020Z DEBUG sepial: >> #2018 FindHome: G28 X Y
2026-02-13T11:19:17.092044Z DEBUG sepial: > "ok"
2026-02-13T11:19:17.092103Z DEBUG sepial: >> #2019 MotorsDisengage: M18
2026-02-13T11:19:18.091442Z DEBUG sepial: > "ok"
```

## Inspiration
* https://github.com/MarginallyClever/Makelangelo-software/blob/7.78.3/src/main/java/com/marginallyclever/makelangelo/plotter/plottercontrols/MarlinPanel.java
* https://github.com/MarginallyClever/Makelangelo-software/blob/7.78.3/src/main/java/com/marginallyclever/makelangelo/plotter/plottercontrols/MarlinPlotterPanel.java
* https://github.com/MarginallyClever/Makelangelo-software/blob/7.78.3/src/main/java/com/marginallyclever/makelangelo/plotter/plottercontrols/PlotterControls.java
* https://github.com/MarginallyClever/Makelangelo-software/blob/7.78.3/src/main/java/com/marginallyclever/makelangelo/plotter/plottersettings/PlotterSettings.java
