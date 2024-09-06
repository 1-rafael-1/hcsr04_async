//! # hc-sr04-async
//!
//! This crate provides an asynchronous driver for the HC-SR04 ultrasonic distance sensor.

#![no_std]

use embassy_time::{with_timeout, Duration, Instant, Timer};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::digital::Wait;

/// The distance unit to use for measurements.
pub enum DistanceUnit {
    Centimeters,
    Inches,
}

/// The temperature unit to use for measurements.
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

/// The configuration for the sensor.
pub struct Config {
    pub distance_unit: DistanceUnit,
    pub temperature_unit: TemperatureUnit,
}

/// The HC-SR04 ultrasonic distance sensor driver.
///
/// # Example
///
/// ```no_run
/// use embassy_rp::gpio::{Output, Input, Level, Pull};
/// use hcsr04_async::{Hcsr04, Config, DistanceUnit, TemperatureUnit};
///
///
/// let p = embassy_rp::init(Default::default());
/// let trigger = Output::new(p.PIN_12, Level::Low);
/// let echo = Input::new(p.PIN_16, Pull::None);
///
/// let config = Config {
///   distance_unit: DistanceUnit::Centimeters,
///   temperature_unit: TemperatureUnit::Celsius,
/// };
///
/// let mut sensor = Hcsr04::new(trigger, echo, config);
///
/// let distance = sensor.measure(20.0).await;
/// match distance {
///    Ok(distance) => {
///       info!("Distance: {} cm", distance);
///   }
///  Err(_) => {
///     info!("Error");
/// }
/// ```
///
/// # Note
///
/// The `measure` method will return an error if the echo pin is already high.
/// The `measure` method will return an error if the echo pin does not go high or low within 2 seconds each.

pub struct Hcsr04<TRIGPIN: OutputPin, ECHOPIN: InputPin + Wait> {
    trigger: TRIGPIN,
    echo: ECHOPIN,
    config: Config,
}

impl<TRIGPIN: OutputPin, ECHOPIN: InputPin + Wait> Hcsr04<TRIGPIN, ECHOPIN> {
    pub fn new(trigger: TRIGPIN, echo: ECHOPIN, config: Config) -> Self {
        Self {
            trigger,
            echo,
            config,
        }
    }

    fn speed_of_sound_temperature_adjusted(&self, temperature: f32) -> f32 {
        match self.config.temperature_unit {
            TemperatureUnit::Celsius => 331.5 + 0.6 * temperature,
            TemperatureUnit::Fahrenheit => 0.049 * temperature + 331.4,
        }
    }

    fn distance(&self, speed_of_sound: f32, duration: Duration) -> f32 {
        let distance = (speed_of_sound * duration.as_micros() as f32) / 2.0;
        distance
    }

    pub async fn measure(&mut self, temperature: f32) -> Result<f32, &'static str> {
        // error if the echo pin is already high
        if self.echo.is_high().ok().unwrap() {
            return Err("Echo pin is already high");
        }

        // Send a 10us pulse to the trigger pin
        self.trigger.set_high().ok();
        Timer::after(Duration::from_micros(10)).await;
        self.trigger.set_low().ok();

        // Wait for the echo pin to go high with a timeout. If the timeout is reached, return an error.
        let start = match with_timeout(Duration::from_secs(2), self.echo.wait_for_high()).await {
            Ok(_) => Instant::now(),
            Err(_) => return Err("Timeout waiting for echo pin to go high"),
        };

        // Wait for the echo pin to go low with a timeout. If the timeout is reached, return an error.
        let end = match with_timeout(Duration::from_secs(2), self.echo.wait_for_low()).await {
            Ok(_) => Instant::now(),
            Err(_) => return Err("Timeout waiting for echo pin to go low"),
        };

        // Calculate the distance
        let pulse_duration = end - start;
        Ok(self.distance(
            self.speed_of_sound_temperature_adjusted(temperature),
            pulse_duration,
        ))
    }
}
