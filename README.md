# tavr

Linux I/O access
```bash
> cat /etc/udev/rules.d/99-espressif.rules 
SUBSYSTEMS=="usb", ATTRS{idVendor}=="303a", ATTRS{idProduct}=="1001", GROUP="espressif", MODE="0666"
```
```bash
> sudo usermod -a -G uucp user
```
ESP-IDF Install
```bash
> yay -S esp-idf
> /opt/esp-idf/install.fish --target esp32s3
> source /opt/esp-idf/export.fish
```
Toolchain Installation
```bash
> rustup toolchain install stable --component rust-src
> cargo install ldproxy --locked
> cargo install espup --locked
> espup install --targets esp32s3
> espup completions fish > ~/.config/fish/completions/espup.fish
> cargo install esp-generate --locked
> cargo install espflash --locked
```
```bash
cargo build --target xtensa-esp32s3-none-elf --release
espflash flash --monitor /dev/ttyACM0
```
