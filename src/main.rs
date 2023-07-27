use esp_idf_sys as _;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_sys::esp_cam::{camera_config_t__bindgen_ty_1, camera_config_t__bindgen_ty_2};
use log::*;
use std::{thread::sleep, time::Duration};

use anyhow::{bail, Result};
use embedded_svc::utils::io;
use embedded_svc::{
    http::client::Client,
    io::Write,
    wifi::{ClientConfiguration, Configuration},
};
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};

const SSID: &str = env!("WIFI_SSID");
const PASS: &str = env!("WIFI_PASS");
const TELEGRAM_BOT_ID: &str = env!("TELEGRAM_BOT_ID");
const TELEGRAM_CHAT_ID: &str = env!("TELEGRAM_CHAT_ID");

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs)).unwrap();

    let mut led = PinDriver::output(peripherals.pins.gpio4)?;

    wifi_driver
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            ..Default::default()
        }))
        .unwrap();

    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();
    while !wifi_driver.is_connected().unwrap() {
        let config = wifi_driver.get_configuration().unwrap();
        println!("Waiting for station {:?}", config);
        sleep(Duration::new(1, 0));
    }
    info!("Should be connected now");

    let camera_config = esp_idf_sys::esp_cam::camera_config_t {
        pin_pwdn: 32,
        pin_reset: -1,
        pin_xclk: 0,
        __bindgen_anon_1: { camera_config_t__bindgen_ty_1 { pin_sccb_sda: 26 } }, // C union in struct: pin_sccb_sda: 26,
        __bindgen_anon_2: { camera_config_t__bindgen_ty_2 { pin_sccb_scl: 27 } }, // C union in struct: pin_sccb_scl: 27,

        pin_d7: 35,
        pin_d6: 34,
        pin_d5: 39,
        pin_d4: 36,
        pin_d3: 21,
        pin_d2: 19,
        pin_d1: 18,
        pin_d0: 5,
        pin_vsync: 25,
        pin_href: 23,
        pin_pclk: 22,

        //XCLK 20MHz or 10MHz for OV2640 double FPS (Experimental)
        xclk_freq_hz: 20000000,
        ledc_timer: esp_idf_sys::esp_cam::ledc_timer_t_LEDC_TIMER_0,
        ledc_channel: esp_idf_sys::esp_cam::ledc_channel_t_LEDC_CHANNEL_0,

        pixel_format: esp_idf_sys::esp_cam::pixformat_t_PIXFORMAT_JPEG, //YUV422,GRAYSCALE,RGB565,JPEG
        frame_size: esp_idf_sys::esp_cam::framesize_t_FRAMESIZE_UXGA, //QQVGA-UXGA Do not use sizes above QVGA when not JPEG

        jpeg_quality: 10, //0-63 lower number means higher quality
        fb_count: 1,      //if more than one, i2s runs in continuous mode. Use only with JPEG
        fb_location: esp_idf_sys::esp_cam::camera_fb_location_t_CAMERA_FB_IN_PSRAM,
        grab_mode: esp_idf_sys::esp_cam::camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY,
        sccb_i2c_port: 0,
    };

    if unsafe { esp_idf_sys::esp_cam::esp_camera_init(&camera_config) } != 0 {
        bail!("camera init failed!");
    } else {
        info!("camera ready! >>>>>>>>>>>>>>>>>>>>>>>>>>>>");
    }

    let mut num = 0;
    loop {
        info!(
            "IP info: {:?}",
            wifi_driver.sta_netif().get_ip_info().unwrap()
        );

        led.set_high()?;
        sleep(Duration::new(1, 0));
        info!("Taking picture ... {}", num);
        let fb = unsafe { esp_idf_sys::esp_cam::esp_camera_fb_get() };
        info!("Picture taken! Its size was: {} bytes", unsafe {
            (*fb).len
        });
        led.set_low()?;

        let photo = unsafe { std::slice::from_raw_parts((*fb).buf, (*fb).len) };

        unsafe {
            esp_idf_sys::esp_cam::esp_camera_fb_return(fb);
        }
        num += 1;

        // --------------------
        let url: String = format!("https://api.telegram.org/bot{}/sendPhoto", TELEGRAM_BOT_ID);
        let mut payload = vec![];
        payload.extend_from_slice(b"--X-ESPIDF_MULTIPART\r\n");
        payload.extend_from_slice(b"Content-Disposition: form-data; name=\"chat_id\"\r\n");
        payload.extend_from_slice(b"Content-Type: Content-Type: text/plain\r\n\r\n");
        payload.extend_from_slice(TELEGRAM_CHAT_ID.as_bytes());
        payload.extend_from_slice(b"\r\n");
        payload.extend_from_slice(b"--X-ESPIDF_MULTIPART\r\n");
        payload.extend_from_slice(
            b"Content-Disposition: form-data; name=\"photo\"; filename=\"hoge.jpg\"\r\n",
        );
        payload.extend_from_slice(b"Content-Type: Content-Type: image/jpeg\r\n\r\n");
        payload.extend_from_slice(photo);
        payload.extend_from_slice(b"\r\n");
        payload.extend_from_slice(b"--X-ESPIDF_MULTIPART--\r\n");
        // Prepare headers and URL
        let content_length_header = format!("{}", payload.len());
        let headers = [
            ("accept", "*/*"),
            (
                "content-type",
                "multipart/form-data; boundary=X-ESPIDF_MULTIPART",
            ),
            ("connection", "close"),
            ("content-length", &content_length_header),
        ];
        info!("About to fetch content from {}", url);

        let mut client = Client::wrap(esp_idf_svc::http::client::EspHttpConnection::new(
            &esp_idf_svc::http::client::Configuration {
                crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
                ..Default::default()
            },
        )?);

        // Send request
        let mut request = client.post(&url, &headers)?;
        request.write_all(payload.as_slice())?;
        request.flush()?;
        info!("-> POST {}", url);
        let mut response = request.submit()?;
        let mut body = [0_u8; 3048];

        let read = io::try_read_full(&mut response, &mut body).map_err(|err| err.0)?;

        info!(
            "Body (truncated to 3K):\n{:?}",
            String::from_utf8_lossy(&body[..read]).into_owned()
        );

        // Complete the response
        while response.read(&mut body)? > 0 {}

        sleep(Duration::new(10, 0));
    }
}
