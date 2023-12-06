# Rust ESP32S3 Lvgl Slider

The purpose of this demo is to use lv-binding-rust (Lvgl) and show slider widget.

## Development Board
Aliexpress ESP32-8048S070 - 7 inch 800x400 TN RGB with ESP32S3, 8M PSRAM, 16M Flash, 512KB SRAM

## Overview
This application uses the lv-binding-rust crate on a ESP32S3 device.  The program will display a slider widget and show the percentage of the sliders position.

## Comment
See rust-esp32s3-lvgl-clickme project for details on individual folders and files.

## Flashing the ESP32S3 device
I used the following command to flash the ESP32S3 device.
```
$ cargo espflash flash --partition-table=partition-table/partitions.csv --monitor
```

## Picture of Aliexpress ESP32S3 running slider app
![esp32s3-slider](photos/slider.jpg)


# Versions
### v1.0 : 
- initial release
