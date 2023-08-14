# ESP32 Rust Security Camera

This Rust-based project aims to develop a secure home security camera that seamlessly integrates with Telegram. The camera will offer real-time monitoring and instant alerts, providing homeowners with a reliable and convenient way to keep their homes safe.

**Hardware**:

1) ESP32-CAM W-BT (ESP32-S OV2640)
2) HC-SR501 PIR Human Body Motion Sensors

## How To:

1) clone repository (`git clone git@github.com:chilic/espcam-rs.git`)
2) prepare configuration (`cp .env.example .env`) and edit `.env` file
    you will need to specify:
    - WiFi network and password
    - Telegram bot ID ([how to create telegram bot](https://core.telegram.org/bots#how-do-i-create-a-bot), [how to find group chat id](https://stackoverflow.com/questions/32423837/telegram-bot-how-to-get-a-group-chat-id))
3) connect your esp32
4) run `cargo r` to flash development version and `cargo r --release` for production ussage.

![ESP32 Rust Security Camera](/assets/esp32-cam-box-rust.jpeg)
