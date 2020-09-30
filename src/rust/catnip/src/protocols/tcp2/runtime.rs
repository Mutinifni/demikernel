use crate::protocols::{arp, tcp};
use std::net::Ipv4Addr;
use crate::protocols::ethernet2::MacAddress;
use std::rc::Rc;
use std::cell::RefCell;
use std::future::Future;
use std::time::{Duration, Instant};
use rand::Rng;
use rand::distributions::{Standard, Distribution};

pub trait Runtime: Clone + Unpin + 'static {
    fn advance_clock(&self, now: Instant);
    fn transmit(&self, buf: Rc<RefCell<Vec<u8>>>);

    fn local_link_addr(&self) -> MacAddress;
    fn local_ipv4_addr(&self) -> Ipv4Addr;
    fn arp_options(&self) -> arp::Options;
    fn tcp_options(&self) -> tcp::Options;

    type WaitFuture: Future<Output = ()>;
    fn wait(&self, duration: Duration) -> Self::WaitFuture;
    fn wait_until(&self, when: Instant) -> Self::WaitFuture;
    fn now(&self) -> Instant;

    fn rng_gen<T>(&self) -> T where Standard: Distribution<T>;
}

impl Runtime for crate::runtime::Runtime {
    fn advance_clock(&self, now: Instant) {
        crate::runtime::Runtime::advance_clock(self, now);
    }

    fn transmit(&self, buf: Rc<RefCell<Vec<u8>>>) {
        let event = crate::event::Event::Transmit(buf);
        self.emit_event(event);
    }

    fn local_link_addr(&self) -> MacAddress {
        self.options().my_link_addr
    }

    fn local_ipv4_addr(&self) -> Ipv4Addr {
        self.options().my_ipv4_addr
    }

    fn arp_options(&self) -> arp::Options {
        self.options().arp
    }

    fn tcp_options(&self) -> tcp::Options {
        self.options().tcp
    }

    type WaitFuture = crate::runtime::RuntimeWaitFuture;
    fn wait(&self, duration: Duration) -> Self::WaitFuture {
        crate::runtime::Runtime::wait(self, duration)
    }

    fn wait_until(&self, when: Instant) -> Self::WaitFuture {
        crate::runtime::Runtime::wait_until(self, when)
    }

    fn now(&self) -> Instant {
        crate::runtime::Runtime::now(self)
    }

    fn rng_gen<T>(&self) -> T where Standard: Distribution<T> {
        self.with_rng(|r| r.gen())
    }
}