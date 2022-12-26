#![no_std]
#![no_main]

use panic_halt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use cortex_m::asm;
    use stm32f4xx_hal::{gpio::*, pac, prelude::*, timer::MonoTimerUs};

    #[shared]
    struct Shared {
        led: gpioa::PA5<Output<PushPull>>,
        led_active: bool,
    }

    #[local]
    struct Local {
        button: gpioc::PC13<Input>,
    }

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut syscfg = cx.device.SYSCFG.constrain();
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(16.MHz()).freeze();

        let gpioa = cx.device.GPIOA.split();
        let gpioc = cx.device.GPIOC.split();

        // Initialize LED output
        let led = gpioa.pa5.into_push_pull_output();

        let mut button = gpioc.pc13.into_pull_up_input();
        button.make_interrupt_source(&mut syscfg);
        button.enable_interrupt(&mut cx.device.EXTI);
        button.trigger_on_edge(&mut cx.device.EXTI, Edge::Falling);

        // Initliaze monotonic
        let mono = cx.device.TIM2.monotonic_us(&clocks);

        // Schedule led to turn on
        led_on::spawn().ok();

        (
            Shared {
                led,
                led_active: true,
            },
            Local { button },
            init::Monotonics(mono),
        )
    }

    #[task(shared = [led, led_active])]
    fn led_on(mut cx: led_on::Context) {
        cx.shared.led_active.lock(|led_active| {
            if *led_active {
                cx.shared.led.lock(|led| led.set_high());
                led_off::spawn_after(1.secs()).unwrap();
            }
        });
    }

    #[task(shared = [led, led_active])]
    fn led_off(mut cx: led_off::Context) {
        cx.shared.led_active.lock(|led_active| {
            if *led_active {
                cx.shared.led.lock(|led| led.set_low());
                led_on::spawn_after(1.secs()).unwrap();
            }
        });
    }

    #[task(binds=EXTI0, local = [button], shared = [led_active])]
    fn button(mut cx: button::Context) {
        cx.local.button.clear_interrupt_pending_bit();
        cx.shared.led_active.lock(|led_active| {
            *led_active = !(*led_active);
        });
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            asm::nop();
        }
    }
}
