pub mod gt911;
pub mod lcd_panel;

use log::*;

use cstr_core::CString;

use anyhow::Error;

use std::cell::RefCell;
use std::time::Instant;

use esp_idf_hal::{
    delay::{Ets, FreeRtos},
    gpio::PinDriver,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    units::FromValueType,
};

use esp_idf_hal::ledc::{
    config::TimerConfig,
    {LedcDriver, LedcTimerDriver},
};

use lvgl::style::Style;
use lvgl::widgets::{Label, Slider};
use lvgl::{Align, Color, Display, DrawBuffer, Part, TextAlign, Widget};

use embedded_graphics_core::prelude::Point;
use lvgl::input_device::{
    pointer::{Pointer, PointerInputData},
    InputDriver,
};

use crate::gt911::GT911;
use crate::lcd_panel::{LcdPanel, PanelConfig, PanelFlagsConfig, TimingFlagsConfig, TimingsConfig};

fn main() -> anyhow::Result<(), anyhow::Error> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("================ Staring App ================");

    const HOR_RES: u32 = 800;
    const VER_RES: u32 = 480;
    const LINES: u32 = 12; // The number of lines (rows) that will be refreshed

    let peripherals = Peripherals::take()?;

    #[allow(unused)]
    let pins = peripherals.pins;

    //============================================================================================================
    //               Create the I2C to communicate with the touchscreen controller
    //============================================================================================================
    let i2c = peripherals.i2c0;
    let sda = pins.gpio19;
    let scl = pins.gpio20;
    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;
    let rst = PinDriver::output(pins.gpio38)?; // reset pin on GT911

    //============================================================================================================
    //               Create the LedcDriver to drive the backlight on the Lcd Panel
    //============================================================================================================
    let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &TimerConfig::new().frequency(25.kHz().into()),
        )
        .unwrap(),
        pins.gpio2,
    )?;
    channel.set_duty(channel.get_max_duty() / 2)?;
    info!("============= Backlight turned on =============");

    // Initialize lvgl
    lvgl::init();

    //=====================================================================================================
    //                         Create the LCD Display
    //=====================================================================================================
    let mut lcd_panel = LcdPanel::new(
        &PanelConfig::new(),
        &PanelFlagsConfig::new(),
        &TimingsConfig::new(),
        &TimingFlagsConfig::new(),
    )?;

    info!("=============  Registering Display ====================");
    let buffer = DrawBuffer::<{ (HOR_RES * LINES) as usize }>::default();
    let display = Display::register(buffer, HOR_RES, VER_RES, |refresh| {
        lcd_panel
            .set_pixels_lvgl_color(
                refresh.area.x1.into(),
                refresh.area.y1.into(),
                (refresh.area.x2 + 1i16).into(),
                (refresh.area.y2 + 1i16).into(),
                refresh.colors.into_iter(),
            )
            .unwrap();
    })
    .map_err(Error::msg)?;

    //======================================================================================================
    //                          Create the driver for the Touchscreen
    //======================================================================================================
    let gt911_touchscreen = RefCell::new(GT911::new(i2c, rst, Ets));
    gt911_touchscreen.borrow_mut().reset()?;

    // The read_touchscreen_cb is used by Lvgl to detect touchscreen presses and releases
    let read_touchscreen_cb = || {
        let touch = gt911_touchscreen.borrow_mut().read_touch().unwrap();

        match touch {
            Some(tp) => PointerInputData::Touch(Point::new(tp.x as i32, tp.y as i32))
                .pressed()
                .once(),
            None => PointerInputData::Touch(Point::new(0, 0)).released().once(),
        }
    };

    info!("=============  Registering Touchscreen ====================");
    let _touch_screen = Pointer::register(read_touchscreen_cb, &display).map_err(Error::msg)?;

    //=======================================================================================================
    //                               Create the User Interface
    //=======================================================================================================
    // Create screen and widgets
    let mut screen = display.get_scr_act().map_err(Error::msg)?;
    let mut screen_style = Style::default();
    screen_style.set_bg_color(Color::from_rgb((0, 0, 0)));
    screen_style.set_radius(0);
    screen.add_style(Part::Main, &mut screen_style);

    // Create the slider
    let mut slider = Slider::create(&mut screen).map_err(Error::msg)?;
    slider.set_align(Align::Center, 0, 0);

    // Create common label sytle that both labels will use
    let mut style_label = Style::default();
    style_label.set_text_color(Color::from_rgb((255, 255, 255)));
    style_label.set_text_align(TextAlign::Center);

    // Create label above slider
    let mut percent_label_style = style_label.clone();
    let mut percent_label = Label::new().map_err(Error::msg)?;
    percent_label.add_style(Part::Main, &mut percent_label_style);
    percent_label.set_align(Align::Center, 0, -40);
    let _ = percent_label.set_text(&CString::new("%0".to_string()).unwrap());

    // Create label below slider
    let mut speaker_volume_style = style_label.clone();
    let mut speaker_volume_label = Label::new().map_err(Error::msg)?;
    speaker_volume_label.add_style(Part::Main, &mut speaker_volume_style);
    speaker_volume_label.set_align(Align::Center, 0, 40);
    let _ = speaker_volume_label.set_text(&CString::new("Speaker Volume".to_string()).unwrap());

    // Event listener for slider changes
    slider
        .on_event(|slider, event| {
            if let lvgl::Event::ValueChanged = event {
                let value = slider.get_value();
                let _ = percent_label.set_text(&CString::new(format!("%{}", value)).unwrap());
            }
        })
        .map_err(Error::msg)?;

    loop {
        let start = Instant::now();

        lvgl::task_handler();

        // Keep the loop delay short so Lvgl can respond quickly to touchscreen presses and releases
        FreeRtos::delay_ms(20);

        lvgl::tick_inc(Instant::now().duration_since(start));
    }
}
