# air-gradient-pro-rs

TODO
* update my fork of stm32-eth
  - see https://github.com/stm32-rs/stm32-eth/pulls (filter modes PR is WIP)
* try out renode emulation for tests
  - can also emulate the peripherals in Rust: https://antmicro.com/blog/2021/07/rust-peripheral-support-in-renode/
  - https://github.com/antmicro/renode-rust-example
* add features for, compile_error! for conflicts
  - log-rtt
  - log-usart3
  - panic-usart3 (switch to abort instead of loop'n
  - panic-rtt
* Test harness things
  - https://crates.io/crates/substance-framework
  - https://os.phil-opp.com/testing/
  - https://interrupt.memfault.com/blog/test-automation-renode
  - https://github.com/memfault/interrupt-renode-test-automation/blob/master/renode-config.resc
* console: https://github.com/rust-embedded-community/menu


PHY's in here, and touchscreen too
https://github.com/renode/renode/blob/master/platforms/boards/stm32f7_discovery-bb.repl

```
renode renode/emulate.resc

gdb-multiarch target/thumbv7em-none-eabihf/debug/air-gradient-pro-rs
```


https://www.st.com/en/evaluation-tools/nucleo-f429zi.html

https://os.mbed.com/platforms/ST-Nucleo-F429ZI/

USART3 is virtual comm, D8-tx, D9-rx
