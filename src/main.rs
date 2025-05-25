//! The following wiring is assumed:
//! - TX/RX => GPIO2, connected internally and with internal pull-up resistor.
//!
//! ESP1/GND --- ESP2/GND
//! ESP1/GPIO2 --- ESP2/GPIO2

//% CHIPS: esp32 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: esp-hal/unstable

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config, I2c},
    main,
    twai::{self, EspTwaiFrame, StandardId, TwaiMode, filter::SingleStandardFilter},
};
use esp_println::println;
use nb::block;
#[cfg(feature="aht20")]
use aht20_driver;
#[cfg(feature="sht3x")]
use embedded_sht3x::{Repeatability::High, Sht3x, DEFAULT_I2C_ADDRESS};


#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut delay = Delay::new();

    let mut led = Output::new(peripherals.GPIO8, Level::High, OutputConfig::default());

    // Create a new peripheral object with the described wiring and standard I2C clock speed.
    let i2c = I2c::new(
        peripherals.I2C0,
        Config::default(),
    )
    .unwrap()
    .with_sda(peripherals.GPIO0)
    .with_scl(peripherals.GPIO1);

    // Configure the AHT20 temperature and humidity sensor.
    #[cfg(feature="aht20")]
    let mut aht20 = aht20_driver::AHT20::new(i2c, aht20_driver::SENSOR_ADDRESS);
    #[cfg(feature="aht20")]
    let mut sensor = aht20.init(&mut delay).unwrap();

    #[cfg(feature="sht3x")]
    // Create the sensor and configure its repeatability
    let mut sensor = {
        let mut sht3x = Sht3x::new(i2c, DEFAULT_I2C_ADDRESS, delay);
        sht3x.repeatability = High;
        sht3x
    };

    // CAN
    let tx_pin = peripherals.GPIO2;
    let rx_pin = peripherals.GPIO3;

    // The speed of the bus.
    const TWAI_BAUDRATE: twai::BaudRate = twai::BaudRate::B125K;

    // !!! Use `new` when using a transceiver. `new_no_transceiver` sets TX to open-drain
    // Self-testing also works using the regular `new` function.

    // Begin configuring the TWAI peripheral. The peripheral is in a reset like
    // state that prevents transmission but allows configuration.
    // For self-testing use `SelfTest` mode of the TWAI peripheral.
    let mut twai_config = twai::TwaiConfiguration::new_no_transceiver(
        peripherals.TWAI0,
        rx_pin,
        tx_pin,
        TWAI_BAUDRATE,
        TwaiMode::Normal,
    );

    // Partially filter the incoming messages to reduce overhead of receiving
    // undesired messages. Note that due to how the hardware filters messages,
    // standard ids and extended ids may both match a filter. Frame ids should
    // be explicitly checked in the application instead of fully relying on
    // these partial acceptance filters to exactly match.
    // A filter that matches StandardId::ZERO.
    twai_config.set_filter(
        const { SingleStandardFilter::new(b"xxxxxxxxxx1", b"x", [b"xxxxxxxx", b"xxxxxxxx"]) },
    );

    // Start the peripheral. This locks the configuration settings of the peripheral
    // and puts it into operation mode, allowing packets to be sent and
    // received.
    let mut twai = twai_config.start();

    let mut buffer = [0u8; 4];

    loop {
        #[cfg(feature="aht20")]
        // Take the temperature and humidity measurement.
        let measurement = sensor.measure(&mut delay).unwrap();

        #[cfg(feature="sht3x")]
        // Perform a temperature and humidity measurement
        let measurement = sensor.single_measurement().unwrap();

        let temperature = (measurement.temperature * 100.0) as i16;
        let humidity = (measurement.humidity * 100.0) as i16;

        println!("Temperature: {} °C, Relative humidity: {} %", temperature, humidity);

        buffer[0..2].clone_from_slice(&temperature.to_be_bytes());
        buffer[2..4].clone_from_slice(&humidity.to_be_bytes());

        let frame = EspTwaiFrame::new(StandardId::new(0x0101u16).unwrap(), &buffer).unwrap();
        // Transmit a new frame
        block!(twai.transmit(&frame)).unwrap();
        println!("Sent a frame");

        led.toggle();

        delay.delay_millis(500);
    }
}
