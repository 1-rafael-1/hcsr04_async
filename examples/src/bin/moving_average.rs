//! # Simple Measurement with Moving Average Filter
//! This example demonstrates how to use the HCSR04 sensor to measure distance through a Moving Average Filter.
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
use {defmt_rtt as _, panic_probe as _};

/// A simple moving average filter implementation with a fixed-size buffer.
/// The buffer is used to store the last N measurements, where N is the size of the buffer.
struct MovingAverage {
    buffer: [f64; N], // Fixed-size buffer to hold the measurements
    index: usize,     // Current index in the buffer
    sum: f64,         // Sum of the values in the buffer
    count: usize,     // Number of values added (up to N)
}

impl MovingAverage {
    fn new() -> Self {
        Self {
            buffer: [0.0; N],
            index: 0,
            sum: 0.0,
            count: 0,
        }
    }

    fn add_measurement(&mut self, value: f64) {
        // Subtract the value that is being replaced from the sum
        self.sum -= self.buffer[self.index];
        // Add the new value to the buffer and the sum
        self.buffer[self.index] = value;
        self.sum += value;
        // Move to the next index, wrapping around if necessary
        self.index = (self.index + 1) % N;
        // Increment the count up to N
        if self.count < N {
            self.count += 1;
        }
    }

    fn average(&self) -> f64 {
        self.sum / self.count as f64
    }
}

const N: usize = 10; // Size of the moving average buffer

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
    let mut moving_average = MovingAverage::new();

    // The temperature of the environment, if known, can be used to adjust the speed of sound.
    // If unknown, an average estimate must be used.
    let temperature = 24.0;

    loop {
        match sensor.measure(temperature).await {
            Ok(distance) => {
                moving_average.add_measurement(distance);
                let avg_distance = moving_average.average();
                info!("Average distance: {:?}", avg_distance);
            }
            Err(e) => {
                info!("Measurement error: {:?}", e);
            }
        }
        // Adjust the delay as needed. Between N (the buffer size) and the sensor's update rate you should arrive at enough measurements for Your application.
        // Here we wait 50ms between measurements amd the buffer size is 10, so we get a new average every 500ms.
        Timer::after(Duration::from_millis(50)).await;
    }
}
