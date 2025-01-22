//! # Simple Measurement
//! This example demonstrates how to use the HCSR04 sensor to measure distance in a loop.
//!
//! ## Note
//!
//! Most of the hc-sr04 sensors are rated for 5V. The Raspberry Pi Pico is a 3.3V device. While some hc-sr04 tolerate to be operated on 3.3V, it is not recommended to do so. At best the sensor will have a reduced precision.
//! When operating the sensor at 5V with a 3.3V controller like the Pi Pico, the following considerations must be taken into account:
//!
//! - The trigger pin can be directly connected to the controller. 3.3V should be enough to trigger the sensor.
//! - The echo pin must be connected to the controller through a voltage divider to reduce the voltage from 5V to 3.3V. The echo pin on the hc-sr04 will output 5V when the sensor is operated at 5V and that will then damage the controller.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_time::{Delay, Duration, Instant, Timer};
use hcsr04_async::{Config, DistanceUnit, Hcsr04, Now, TemperatureUnit};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Running!");

    let trigger = Output::new(p.PIN_13, Level::Low);
    let echo = Input::new(p.PIN_28, Pull::None);

    let config = Config {
        distance_unit: DistanceUnit::Centimeters,
        temperature_unit: TemperatureUnit::Celsius,
    };

    // Create clock function that returns microseconds
    struct EmbassyClock;

    impl Now for EmbassyClock {
        fn now_micros(&self) -> u64 {
            Instant::now().as_micros()
        }
    }

    let clock = EmbassyClock;

    let delay = Delay;

    let mut sensor = Hcsr04::new(trigger, echo, config, clock, delay);

    // The temperature of the environment, if known, can be used to adjust the speed of sound.
    // If unknown, an average estimate must be used.
    let temperature = 24.0;

    loop {
        let distance = sensor.measure(temperature).await;
        match distance {
            Ok(distance) => {
                info!("Distance: {} cm", distance);
            }
            Err(e) => {
                info!("Error: {:?}", e);
            }
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}
