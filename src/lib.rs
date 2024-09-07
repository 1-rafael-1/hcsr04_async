//! # hc-sr04-async
//! 
//! This crate provides an asynchronous driver for the HC-SR04 ultrasonic distance sensor.
//! 
//! The driver is designed to work with Celsius and Fahrenheit temperatures and centimeters and inches for distance measurements.
//!
//! # Note
//! 
//! Due to the non-blocking nature of this driver there is a probabiity that either the trigger pulse or the echo measurement
//! get impacted by other async tasks. If this becomes a problem You must either use a blocking driver or You can attempt to run this 
//! driver in a higher priority task.

#![no_std]

use embassy_time::{with_timeout, Duration, Instant, Timer};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::digital::Wait;
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

/// The HC-SR04 ultrasonic distance sensor driver.
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
    /// Initialize a new sensor.
    /// Requires trigger pin and an echo pin, measurements are taken on the echo pin.
    /// Requires a config.
    pub fn new(trigger: TRIGPIN, echo: ECHOPIN, config: Config) -> Self {
        Self {
            trigger,
            echo,
            config,
        }
    }

    /// Calculate the speed of sound in meters per second, adjusted for temperature.
    /// Takes a temperature in units specified in the config.
    fn speed_of_sound_temperature_adjusted(&self, temperature: f64) -> f64 {
        let temp = match self.config.temperature_unit {
            TemperatureUnit::Celsius => {
                temperature
            },
            TemperatureUnit::Fahrenheit => {
                (temperature - 32.0) * 5.0 / 9.0
            }
        };
        331.5 * sqrt(1.0 + (temp/273.15))
    }

    /// Calculate the distance in centimeters based on the speed of sound and the duration of the pulse.
    /// The duration is in seconds and must be divided by 2 to account for the round trip.
    /// Returns the distance in the unit specified in the config.
    fn distance(&self, speed_of_sound: f64, duration_secs:f64) -> f64 {
        let distance = (speed_of_sound * 100.0 * duration_secs) / 2.0;
        match self.config.distance_unit 
        {
            DistanceUnit::Centimeters => distance,
            DistanceUnit::Inches => distance / 2.54,
        }
    }

    /// Measure the distance in the unit specified in the config.
    /// Takes a temperature in units specified in the config.
    /// Returns the distance in the unit specified in the config.
    pub async fn measure(&mut self, temperature: f64) -> Result<f64, &'static str> {
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
        let pulse_duration_secs = (end - start).as_micros() as f64 / 1_000_000.0;
        Ok(self.distance(
            self.speed_of_sound_temperature_adjusted(temperature),
            pulse_duration_secs,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::digital::{ErrorType, ErrorKind};
    use defmt_rtt as _;
    use core::sync::atomic::{AtomicU32, Ordering};
    use libm::round;
    
    // timestamp provider
    static COUNT: AtomicU32 = AtomicU32::new(0);
    defmt::timestamp!("{=u32:us}", COUNT.fetch_add(1, Ordering::Relaxed));

    // Implement the critical_section functions
    use critical_section::RawRestoreState;

    struct CriticalSection;

    unsafe impl critical_section::Impl for CriticalSection {
        unsafe fn acquire() -> RawRestoreState {
            () // Implement critical section acquire
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
        fn set_state(&mut self, _state: embedded_hal::digital::PinState) -> Result<(),Self::Error> {
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

    #[test]
    fn speevd_of_sound_m_per_s_temperature_adjusted_0() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(round(sensor.speed_of_sound_temperature_adjusted(0.0)), round(331.5));
    }

    #[test]
    fn speed_of_sound_m_per_s_temperature_adjusted_20() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(round(sensor.speed_of_sound_temperature_adjusted(20.0)), round(343.42));
    }

    #[test]
    fn speed_of_sound_m_per_s_temperature_adjusted_40() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(round(sensor.speed_of_sound_temperature_adjusted(40.0)), round(354.94));
    }

    #[test]
    fn distance_cm_duration_0secs() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(sensor.distance(343.14, 0.0), 0.0);
    }

    #[test]
    fn distance_cm_duration_5ms() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(sensor.distance(343.14, 0.005), 85.785);
    }

    #[test]
    fn distance_cm_duration_10ms() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(sensor.distance(343.14, 0.01), 171.57);
    }

    #[test]
    fn can_use_fahrenheit() {
        let config = Config {
            distance_unit: DistanceUnit::Centimeters,
            temperature_unit: TemperatureUnit::Fahrenheit,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(round(sensor.speed_of_sound_temperature_adjusted(32.0)), round(331.5));
    }

    #[test]
    fn can_use_inches() {
        let config = Config {
            distance_unit: DistanceUnit::Inches,
            temperature_unit: TemperatureUnit::Celsius,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(round(sensor.distance(343.14, 0.01)), round(67.56));
    }

    #[test]
    fn can_use_fahrenheit_and_inches() {
        let config = Config {
            distance_unit: DistanceUnit::Inches,
            temperature_unit: TemperatureUnit::Fahrenheit,
        };
        let sensor = Hcsr04::new(OutputPinMock, InputPinMock, config);
        assert_eq!(round(sensor.speed_of_sound_temperature_adjusted(32.0)), round(331.5));
        assert_eq!(round(sensor.distance(343.14, 0.01)), round(67.56));
    }
}
