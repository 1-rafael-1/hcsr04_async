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

    let trigger = Output::new(p.PIN_12, Level::Low);
    let echo = Input::new(p.PIN_16, Pull::None);

    let config = Config {
        distance_unit: DistanceUnit::Centimeters,
        temperature_unit: TemperatureUnit::Celsius,
    };

    let mut sensor = Hcsr04::new(trigger, echo, config);

    loop {
        let distance = sensor.measure(20.0).await;
        match distance {
            Ok(distance) => {
                info!("Distance: {} cm", distance);
            }
            Err(_) => {
                info!("Error");
            }
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}
