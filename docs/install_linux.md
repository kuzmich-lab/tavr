ESP32-S3-WROOM-1U-N16R8:
- dual-core XTensa LX7 MCU
- running at 240 MHz. 
- 512 KB SRAM
- 384 KB ROM
- integrated 2.4 GHz, 802.11 b/g/n Wi-Fi and Bluetooth 5 (LE)
- 45 programmable GPIOs
- Flash 16 MB (Quad SPI)
- PSRAM 8 MB (Octal SPI)

Linux I/O access
> cat /etc/udev/rules.d/99-espressif.rules 
SUBSYSTEMS=="usb", ATTRS{idVendor}=="303a", ATTRS{idProduct}=="1001", GROUP="espressif", MODE="0666"

> sudo usermod -a -G uucp user

ESP-IDF Install
> yay -S esp-idf
> /opt/esp-idf/install.fish --target esp32s3
> source /opt/esp-idf/export.fish

Toolchain Installation
> rustup toolchain install stable --component rust-src
> cargo install ldproxy --locked
> cargo install espup --locked
> espup install --targets esp32s3
> espup completions fish > ~/.config/fish/completions/espup.fish


> cargo install esp-generate --locked
> cargo install espflash --locked
> cargo install esp-config --features=tui --locked

Generate Project
> esp-config

cargo build --target xtensa-esp32s3-none-elf --release
espflash flash --monitor /dev/ttyACM0
