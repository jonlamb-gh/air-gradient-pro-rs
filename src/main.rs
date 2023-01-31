#![no_main]
#![no_std]

// https://docs.rs/stm32f4xx-hal/0.13.2/stm32f4xx_hal/index.html

// TODO
//use panic_abort as _; // panic handler
use panic_rtt_target as _; // panic handler

mod eth;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use log::{error, info, warn};
    use rtt_logger::RTTLogger;
    use rtt_target::rtt_init_print;
    use stm32f4xx_hal::{
        gpio::{Alternate, NoPin, Output, Pin, PushPull, AF5, PB3, PB5, PC13},
        hal::blocking::spi::{Operation, Transactional},
        nb,
        pac::{self, SPI1, USART1},
        prelude::*,
        serial::config::{Config, DmaConfig as SerialDmaConfig, StopBits},
        serial::{Rx, Serial, Tx},
        spi::{self, Spi, TransferModeNormal},
        timer::MonoTimerUs,
    };

    /// LED on PC13
    type LedPin = PC13<Output>;

    const LOGGER: RTTLogger = RTTLogger::new(log::LevelFilter::Info);

    #[shared]
    struct Shared {
        //
    }

    #[local]
    struct Local {
        led: LedPin,
        cnt: usize,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Info))
            .unwrap();

        info!("Starting");

        // Set up the system clock
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(72.MHz()).freeze();

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();

        let mut led = gpioc.pc13.into_push_pull_output();
        led.set_low();

        let mono = ctx.device.TIM2.monotonic_us(&clocks);

        info!("Initialized");

        tick::spawn().ok();

        (Shared {}, Local { led, cnt: 0 }, init::Monotonics(mono))
    }

    /*
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            continue;
        }
    }
    */

    #[task(local = [led, cnt])]
    fn tick(ctx: tick::Context) {
        tick::spawn_after(1.secs()).ok();
        ctx.local.led.toggle();
        info!("cnt = {}", ctx.local.cnt);
        *ctx.local.cnt += 1;
    }
}
