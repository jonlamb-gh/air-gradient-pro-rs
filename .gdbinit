target remote :3333

# print demangled symbols by default
set print asm-demangle on

#break main
#break core::panicking::panic_fmt

# p info.message.0.pieces[0]
break panic_rtt_target::panic

monitor start
