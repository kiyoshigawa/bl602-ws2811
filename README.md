# bl602-ws2811
This is a repo for testing WS2811 strips with custom animations while learning the embedded-hal for rust. 

This is based on https://github.com/9names/bl602-rust-example but all new code is my own and to be licensed under the GPLv3 instead of the original repo's MIT licensing.

It will use animations similar in nature to the lighting_control C library I built for the oMIDItone v2: https://github.com/kiyoshigawa/oMIDItone_Controller_V2

Initially I will be manually handling the timings for the WS2811 strips using clock cycle counts, but eventually I'd like to switch to using some peripherals, such as SPI, for better timing control if possible.

This is all being built and tested on Windows using the latest stable rust compiler. I'm avoiding nightly builds and unstable features for the time being.

To flash your BL602, you'll need it to be running a bootloader, and be in flashing mode. For my Pinecone BL602 board, I have to swap the jumper on pin IO8 from L to H and reset the board to enter bootloader mode. Once it's in bootloader mode, just run `cargo blflash --release --port=COMXX` and it will compile and upload the code. Then to actually run the program I need to swap the jumper back to L and reset it again.

I would eventually like to switch to using the [blash flasher](https://github.com/bjoernQ/blash) but for now I'm going to keep using what works rather than fight the toolchain before I even have LEDs working.
