[![ci](https://github.com/1-rafael-1/hcsr04_async/actions/workflows/rust.yml/badge.svg)](https://github.com/1-rafael-1/hcsr04_async/actions/workflows/rust.yml)
[![Docs](https://docs.rs/hcsr04_async/badge.svg)](https://docs.rs/hcsr04_async/)
[![Latest version](https://img.shields.io/crates/v/hcsr04_async.svg)](https://crates.io/crates/hcsr04_async)
![License](https://img.shields.io/crates/l/hcsr04_async.svg)


# hcsr04_async
Driver for HC-SR04 ultrasonic distance measuring device for async no-std Rust.

The driver is designed to work with Celsius and Fahrenheit temperatures and centimeters and inches for distance measurements.

## Features

- `blocking_trigger`: (Recommended) This feature enables blocking behavior for the trigger pulse, ensuring more accurate timing. It's recommended for most use cases unless you have specific reasons to avoid blocking operations.

Note that this only makes the blocking trigger pulse of 10us blocking, the remainder will still be async.

## Note

Due to the non-blocking nature of this driver there is a probability that either the trigger pulse or the echo measurement get impacted by other async tasks. If this becomes a problem You must either use a blocking driver or You can attempt to run this driver in a higher priority task.

## Example

```rust
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
