//! # hc-sr04-async
//!
//! This crate provides an asynchronous driver for the HC-SR04 ultrasonic distance sensor.
//!
//! The driver is designed to work with Celsius and Fahrenheit temperatures and centimeters and inches for distance measurements.
//!
//! ## Features
//!
//! - `blocking_trigger`: (Default) This feature enables blocking behavior for the trigger pulse,
//!   ensuring more accurate timing. For 10us async is not very accurate, because it introduces a few microseconds of management. This should normally not affect the sensor,
//!   many do in fact still trigger if the actual trigger pulse was i.e. 15us. So if you absolutely need the core to do other things while waiting for the trigger pulse,
//!   you can disable this feature.
//!
//! # Example
//!
//! ```rust, ignore
//! #![no_std]
//! #![no_main]
//!
//! use defmt::*;
//! use embassy_executor::Spawner;
//! use embassy_rp::gpio::{Input, Level, Output, Pull};
//! use embassy_time::{Delay, Duration, Instant, Timer};
//! use hcsr04_async::{Config, DistanceUnit, Hcsr04, Now, TemperatureUnit};
//! use {defmt_rtt as _, panic_probe as _};
//!
//! #[embassy_executor::main]
//! async fn main(_spawner: Spawner) {
//!     let p = embassy_rp::init(Default::default());
//!     info!("Running!");
//!
//!     let trigger = Output::new(p.PIN_13, Level::Low);
//!     let echo = Input::new(p.PIN_28, Pull::None);
//!
//!     let config = Config {
//!         distance_unit: DistanceUnit::Centimeters,
//!         temperature_unit: TemperatureUnit::Celsius,
//!     };
//!
//!     // Create clock function that returns microseconds
//!     struct EmbassyClock;
//!
//!     impl Now for EmbassyClock {
//!         fn now_micros(&self) -> u64 {
//!             Instant::now().as_micros()
//!         }
//!     }
//!
//!     let clock = EmbassyClock;
//!     let delay = Delay;
//!
//!     let mut sensor = Hcsr04::new(trigger, echo, config, clock, delay);
//!
//!     // The temperature of the environment, if known, can be used to adjust the speed of sound.
//!     // If unknown, an average estimate must be used.
//!     let temperature = 24.0;
//!
//!     loop {
//!         let distance = sensor.measure(temperature).await;
//!         match distance {
//!             Ok(distance) => {
//!                 info!("Distance: {} cm", distance);
//!             }
//!             Err(e) => {
//!                 info!("Error: {:?}", e);
//!             }
//!         }
//!         Timer::after(Duration::from_secs(1)).await;
//!     }
//! }
//! ```

#![no_std]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use embedded_hal_async::{delay::DelayNs as DelayNsAsync, digital::Wait};
use futures::{select_biased, FutureExt};
use libm::sqrt;

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

pub trait Now {
    // The time elapsed since startup in microseconds
    fn now_micros(&self) -> u64;
}

/// The HC-SR04 ultrasonic distance sensor driver.
///
/// # Note
///
/// The `measure` method will return an error if the echo pin is already high or if the echo pin does not go high or low within 2 seconds each.
pub struct Hcsr04<TRIGPIN, ECHOPIN, CLOCK, DELAY> {
    trigger: TRIGPIN,
    echo: ECHOPIN,
    config: Config,
    clock: CLOCK,
    delay: DELAY,
}

impl<TRIGPIN, ECHOPIN, CLOCK, DELAY> Hcsr04<TRIGPIN, ECHOPIN, CLOCK, DELAY>
where
    TRIGPIN: OutputPin,
    ECHOPIN: InputPin + Wait,
    CLOCK: Now,
    DELAY: DelayNs + DelayNsAsync,
{
    /// Initialize a new sensor.
    /// Requires trigger pin and an echo pin, measurements are taken on the echo pin.
    /// Requires a config.
    /// Requires a clock that will provide the time in microseconds via the `Now` trait.
    /// Requires a delay that implements DelayNs, sync and async.
    pub fn new(
        trigger: TRIGPIN,
        echo: ECHOPIN,
        config: Config,
        clock: CLOCK,
        delay: DELAY,
    ) -> Self {
        Self {
            trigger,
            echo,
            config,
            clock,
            delay,
        }
    }

    /// Calculate the speed of sound in meters per second, adjusted for temperature.
    /// Takes a temperature in units specified in the config.
    fn speed_of_sound_temperature_adjusted(&self, temperature: f64) -> f64 {
        let temp = match self.config.temperature_unit {
            TemperatureUnit::Celsius => temperature,
            TemperatureUnit::Fahrenheit => (temperature - 32.0) * 5.0 / 9.0,
        };
        331.5 * sqrt(1.0 + (temp / 273.15))
    }

    /// Calculate the distance in centimeters based on the speed of sound and the duration of the pulse.
    /// The duration is in seconds and must be divided by 2 to account for the round trip.
    /// Returns the distance in the unit specified in the config.
    fn distance(&self, speed_of_sound: f64, duration_secs: f64) -> f64 {
        let distance = (speed_of_sound * 100.0 * duration_secs) / 2.0;
        match self.config.distance_unit {
            DistanceUnit::Centimeters => distance,
            DistanceUnit::Inches => distance / 2.54,
        }
    }

    /// Measure the distance in the unit specified in the config.
    /// Takes a temperature in units specified in the config.
    /// Returns the distance in the unit specified in the config.
    pub async fn measure(&mut self, temperature: f64) -> Result<f64, &'static str> {
        // error if the echo pin is already high
        match self.echo.is_high() {
            Ok(true) => return Err("Echo pin is already high"),
            Ok(false) => (), // all good, continue
            Err(_) => return Err("Error reading echo pin"),
        }

        // Send a 10us pulse to the trigger pin

        // Set the trigger pin high
        if self.trigger.set_high().is_err() {
            return Err("Error setting trigger pin high");
        }

        // Either block for or wait for 10us, depending on active feature flag
        #[cfg(feature = "blocking_trigger")]
        DelayNs::delay_us(&mut self.delay, 10);
        #[cfg(not(feature = "blocking_trigger"))]
        DelayNsAsync::delay_us(&mut self.delay, 10).await;

        // Set the trigger pin low again
        if self.trigger.set_low().is_err() {
            return Err("Error setting trigger pin low");
        }

        let start = select_biased! {
            _ = self.echo.wait_for_high().fuse() => {
                Now::now_micros(&self.clock)
            }
            _ = DelayNsAsync::delay_ms(&mut self.delay, 2000).fuse() => {
                return Err("Timeout waiting for echo pin to go high");
            }
        };

        let end = select_biased! {
            _ = self.echo.wait_for_low().fuse() => {
                Now::now_micros(&self.clock)
            }
            _ = DelayNsAsync::delay_ms(&mut self.delay, 2000).fuse() => {
                return Err("Timeout waiting for echo pin to go low");
            }
        };

        // Calculate the distance
        let pulse_duration_secs = (end - start) as f64 / 1_000_000.0;
        Ok(self.distance(
            self.speed_of_sound_temperature_adjusted(temperature),
            pulse_duration_secs,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};
    use defmt_rtt as _;
    use embedded_hal::digital::{ErrorKind, ErrorType};
    use libm::round;

    // timestamp provider
    static COUNT: AtomicU32 = AtomicU32::new(0);
    defmt::timestamp!("{=u32:us}", COUNT.fetch_add(1, Ordering::Relaxed));

    // Implement the critical_section functions
    use critical_section::RawRestoreState;

    struct CriticalSection;

    unsafe impl critical_section::Impl for CriticalSection {
        unsafe fn acquire() -> RawRestoreState {
            // Implement critical section acquire
        }

        unsafe fn release(_state: RawRestoreState) {
            // Implement critical section release
        }
    }
    critical_section::set_impl!(CriticalSection);

    struct OutputPinMock;
    impl ErrorType for OutputPinMock {
        type Error = ErrorKind;
    }

    impl OutputPin for OutputPinMock {
        fn set_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn set_state(
            &mut self,
            _state: embedded_hal::digital::PinState,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct InputPinMock;
    impl ErrorType for InputPinMock {
        type Error = ErrorKind;
    }
    impl InputPin for InputPinMock {
        fn is_high(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn is_low(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
    }
    impl Wait for InputPinMock {
        async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct ClockMock;
    impl Now for ClockMock {
        fn now_micros(&self) -> u64 {
            0
        }
    }

    struct DelayMock;
    impl DelayNs for DelayMock {
        fn delay_ns(&mut self, _ns: u32) {}
    }
    impl DelayNsAsync for DelayMock {
        async fn delay_ns(&mut self, _ns: u32) {}
    }

    #[test]
    fn speed_of_sound_m_per_s_temperature_adjusted_0() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(
            round(sensor.speed_of_sound_temperature_adjusted(0.0)),
            round(331.5)
        );
    }

    #[test]
    fn speed_of_sound_m_per_s_temperature_adjusted_20() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(
            round(sensor.speed_of_sound_temperature_adjusted(20.0)),
            round(343.42)
        );
    }

    #[test]
    fn speed_of_sound_m_per_s_temperature_adjusted_40() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(
            round(sensor.speed_of_sound_temperature_adjusted(40.0)),
            round(354.94)
        );
    }

    #[test]
    fn distance_cm_duration_0secs() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(sensor.distance(343.14, 0.0), 0.0);
    }

    #[test]
    fn distance_cm_duration_5ms() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(sensor.distance(343.14, 0.005), 85.785);
    }

    #[test]
    fn distance_cm_duration_10ms() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(sensor.distance(343.14, 0.01), 171.57);
    }

    #[test]
    fn can_use_fahrenheit() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Fahrenheit,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(
            round(sensor.speed_of_sound_temperature_adjusted(32.0)),
            round(331.5)
        );
    }

    #[test]
    fn can_use_inches() {
        let config = Config {
            distance_unit: DistanceUnit::Inches,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(round(sensor.distance(343.14, 0.01)), round(67.56));
    }

    #[test]
    fn can_use_fahrenheit_and_inches() {
        let config = Config {
            distance_unit: DistanceUnit::Inches,
            temperature_unit: TemperatureUnit::Fahrenheit,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config, ClockMock, DelayMock);
        assert_eq!(
            round(sensor.speed_of_sound_temperature_adjusted(32.0)),
            round(331.5)
        );
        assert_eq!(round(sensor.distance(343.14, 0.01)), round(67.56));
    }
}
