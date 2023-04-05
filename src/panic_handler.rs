use core::{
    panic::PanicInfo,
    sync::atomic::{compiler_fence, Ordering::SeqCst},
    sync::atomic::{AtomicBool, Ordering},
};
use cortex_m::asm;

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use cortex_m::interrupt;

    interrupt::disable();

    // Recursion protection
    static PANICKED: AtomicBool = AtomicBool::new(false);
    while PANICKED.load(Ordering::Relaxed) {
        asm::nop();
    }
    PANICKED.store(true, Ordering::Relaxed);

    let w = unsafe { crate::logger::get_logger() };
    writeln!(w, "\n********************************\r").ok();
    writeln!(w, "PANIC\r").ok();
    writeln!(w, "{info}\r").ok();
    writeln!(w, "********************************\r").ok();

    // Halt, assume the watchdog will reset (if enabled)
    loop {
        compiler_fence(SeqCst);
    }
}
