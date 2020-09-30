use catnip::protocols::tcp2::runtime::Runtime;
use catnip::protocols::ethernet2::MacAddress;
use catnip::protocols::{arp, tcp};
use catnip::runtime::Timer;
use std::cell::RefCell;
use std::mem;
use std::ptr;
use std::slice;
use std::rc::Rc;
use std::net::Ipv4Addr;
use std::time::Duration;
use std::time::Instant;
use catnip::runtime::TimerPtr;
use crate::bindings::{rte_eth_dev, rte_eth_devices, rte_mbuf, rte_mempool};

#[derive(Clone)]
pub struct TimerRc(Rc<Timer<TimerRc>>);

impl TimerPtr for TimerRc {
    fn timer(&self) -> &Timer<Self> {
        &*self.0
    }
}

#[derive(Clone)]
pub struct LibOSRuntime {
    inner: Rc<RefCell<Inner>>,
}

extern "C" {
    fn catnip_libos_free_pkt(m: *mut rte_mbuf);
    fn catnip_libos_alloc_pkt(mp: *mut rte_mempool) -> *mut rte_mbuf;
    fn catnip_libos_eth_tx_burst(port_id: u16, queue_id: u16, tx_pkts: *mut *mut rte_mbuf, nb_pkts: u16) -> u16;
    fn catnip_libos_eth_rx_burst(port_id: u16, queue_id: u16, rx_pkts: *mut *mut rte_mbuf, nb_pkts: u16) -> u16;
}

impl LibOSRuntime {
    pub fn new(link_addr: MacAddress, ipv4_addr: Ipv4Addr, dpdk_port_id: u16, dpdk_mempool: *mut rte_mempool) -> Self {
        let now = Instant::now();
        let inner = Inner {
            timer: TimerRc(Rc::new(Timer::new(now))),
            link_addr,
            ipv4_addr,
            rng: 1,
            arp_options: arp::Options::default(),
            tcp_options: tcp::Options::default(),

            dpdk_port_id,
            dpdk_mempool,
        };
        Self {
            inner: Rc::new(RefCell::new(inner))
        }
    }

    pub fn receive(&self, mut packet_in: impl FnMut(&[u8])) -> usize {
        let dpdk_port = { self.inner.borrow().dpdk_port_id };

        const MAX_QUEUE_DEPTH: usize = 64;
        let mut packets: [*mut rte_mbuf; MAX_QUEUE_DEPTH] = unsafe { mem::zeroed() };

        // rte_eth_rx_burst is declared `inline` in the header.
        let nb_rx = unsafe {catnip_libos_eth_rx_burst(dpdk_port, 0, packets.as_mut_ptr(), MAX_QUEUE_DEPTH as u16) };
        // let dev = unsafe { rte_eth_devices[dpdk_port as usize] };
        // let rx_burst = dev.rx_pkt_burst.expect("Missing RX burst function");
        // // This only supports queue_id 0.
        // let nb_rx = unsafe { (rx_burst)(*(*dev.data).rx_queues, todo!(), MAX_QUEUE_DEPTH as u16) };

        for &packet in &packets[..nb_rx as usize] {
            // auto * const p = rte_pktmbuf_mtod(packet, uint8_t *);
            let p = unsafe { ((*packet).buf_addr as *const u8).offset((*packet).data_off as isize) };;
            let data = unsafe { slice::from_raw_parts(p, (*packet).data_len as usize) };
            packet_in(data);
            unsafe { catnip_libos_free_pkt(packet as *const _ as *mut _) };
        }

        nb_rx as usize
    }
}

struct Inner {
    timer: TimerRc,
    link_addr: MacAddress,
    ipv4_addr: Ipv4Addr,
    rng: u32,
    arp_options: arp::Options,
    tcp_options: tcp::Options,

    dpdk_port_id: u16,
    dpdk_mempool: *mut rte_mempool,
}

impl Runtime for LibOSRuntime {
    fn transmit(&self, buf: Rc<RefCell<Vec<u8>>>) {
        let pool = { self.inner.borrow().dpdk_mempool };
        let dpdk_port_id = { self.inner.borrow().dpdk_port_id };
        let mut pkt = unsafe { catnip_libos_alloc_pkt(pool) };
        assert!(!pkt.is_null());

        let rte_pktmbuf_headroom = 128;
        let buf_len = unsafe { (*pkt).buf_len } - rte_pktmbuf_headroom;
        assert!(buf_len as usize >= buf.borrow().len());

        let out_ptr = unsafe { ((*pkt).buf_addr as *mut u8).offset((*pkt).data_off as isize) };
        let out_slice = unsafe { slice::from_raw_parts_mut(out_ptr, buf_len as usize) };
        out_slice[..buf.borrow().len()].copy_from_slice(&buf.borrow()[..]);
        let num_sent = unsafe {
            (*pkt).data_len = buf.borrow().len() as u16;
            (*pkt).pkt_len = buf.borrow().len() as u32;
            (*pkt).nb_segs = 1;
            (*pkt).next = ptr::null_mut();

            catnip_libos_eth_tx_burst(dpdk_port_id, 0, &mut pkt as *mut _, 1)
        };
        assert_eq!(num_sent, 1);
    }

    fn local_link_addr(&self) -> MacAddress {
        self.inner.borrow().link_addr.clone()
    }

    fn local_ipv4_addr(&self) -> Ipv4Addr {
        self.inner.borrow().ipv4_addr.clone()
    }

    fn tcp_options(&self) -> tcp::Options {
        self.inner.borrow().tcp_options.clone()
    }

    fn arp_options(&self) -> arp::Options {
        self.inner.borrow().arp_options.clone()
    }

    fn advance_clock(&self, now: Instant) {
        self.inner.borrow_mut().timer.0.advance_clock(now);
    }

    type WaitFuture = catnip::runtime::WaitFuture<TimerRc>;
    fn wait(&self, duration: Duration) -> Self::WaitFuture {
        let self_ = self.inner.borrow_mut();
        let now = self_.timer.0.now();
        self_.timer.0.wait_until(self_.timer.clone(), now + duration)
    }
    fn wait_until(&self, when: Instant) -> Self::WaitFuture {
        let self_ = self.inner.borrow_mut();
        self_.timer.0.wait_until(self_.timer.clone(), when)
    }

    fn now(&self) -> Instant {
        self.inner.borrow().timer.0.now()
    }

    fn rng_gen_u32(&self) -> u32 {
        let mut self_ = self.inner.borrow_mut();
        let r = self_.rng;
        self_.rng += 1;
        r
    }
}