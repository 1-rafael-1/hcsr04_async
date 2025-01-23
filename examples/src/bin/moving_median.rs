//! # Simple Measurement with Moving Median Filter
//! This example demonstrates how to use the HCSR04 sensor to measure distance through a Moving Median Filter.
//! Such filtering can be useful to reduce the noise in the measurements, especially when the sensor is mounted on a moving platform where the angle and distance to objects can change rapidly.
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
use moving_median::MovingMedian;
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
    let mut moving_median = MovingMedian::<f64, 5>::new();

    loop {
        match sensor.measure(22.0).await {
            Ok(distance) => {
                moving_median.add_value(distance);
                let median_distance = moving_median.median();
                info!("Median distance: {:?}", median_distance);
            }
            Err(e) => {
                info!("Measurement error: {:?}", e);
            }
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}
