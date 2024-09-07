[![ci](https://github.com/1-rafael-1/hcsr04_async/actions/workflows/rust.yml/badge.svg)](https://github.com/1-rafael-1/hcsr04_async/actions/workflows/rust.yml)

# hc-sr04_async
Driver for HC-SR04 ultrasonic distance measuring device for async no-std Rust using Embassy.

The driver is designed to work with Celsius and Fahrenheit temperatures and centimeters and inches for distance measurements.

## Note

Due to the non-blocking nature of this driver there is a probabiity that either the trigger pulse or the echo measurement get impacted by other async tasks. If this becomes a problem You must either use a blocking driver or You can attempt to run this driver in a higher priority task.

## Example

```Rust
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_time::{Duration, Timer};
use hcsr04_async::{Config, DistanceUnit, Hcsr04, TemperatureUnit};
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

    let mut sensor = Hcsr04::new(trigger, echo, config);

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
```

