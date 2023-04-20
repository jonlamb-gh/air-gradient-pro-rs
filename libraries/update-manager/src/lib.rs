#![no_std]
#![forbid(unsafe_code)]
//#![deny(warnings, clippy::all)]
//
//TODO maybe doesn't need smoltcp dep (TcpSocket)
//just process slices instead ?
//
//probably easier to have the socket and call recv/send on it...
//
// TODO ---- all this stuff is going to be in an RTIC task
// but I'm using the bme dev board too, so having it in a lib is easier...
//
//
// what to do when socket IO errors happen?

pub struct UpdateManager {
    // ...
    todo: usize,
}

impl UpdateManager {
    pub const fn new() -> Self {
        Self { todo: 0 }
    }
}
