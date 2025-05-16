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
    i2c::master::{Config, I2c},
    main,
    twai::{self, EspTwaiFrame, StandardId, TwaiMode, filter::SingleStandardFilter},
};
use esp_println::println;
use nb::block;
use aht20_driver;


#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut delay = Delay::new();

    // Create a new peripheral object with the described wiring and standard I2C clock speed.
    let i2c = I2c::new(
        peripherals.I2C0,
        Config::default(),
    )
    .unwrap()
    .with_sda(peripherals.GPIO1)
    .with_scl(peripherals.GPIO2);

    // Configure the AHT20 temperature and humidity sensor.
    let mut aht20_uninit = aht20_driver::AHT20::new(i2c, aht20_driver::SENSOR_ADDRESS);
    let mut aht20 = aht20_uninit.init(&mut delay).unwrap();

    // CAN
    let tx_pin = peripherals.GPIO8;
    let rx_pin = peripherals.GPIO9;

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

    loop {
        // Take the temperature and humidity measurement.
        let aht20_measurement = aht20.measure(&mut delay).unwrap();
        let _humidity = aht20_measurement.humidity;
        let _temperature = aht20_measurement.temperature;

        let frame = EspTwaiFrame::new(StandardId::ZERO, &[1, 2, 3]).unwrap();
        // Transmit a new frame
        block!(twai.transmit(&frame)).unwrap();
        println!("Sent a frame");

        delay.delay_millis(250);
    }
}
